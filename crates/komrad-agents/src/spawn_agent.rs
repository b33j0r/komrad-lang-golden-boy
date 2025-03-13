use crate::registry_agent::RegistryAgent;
use komrad_agent::scope::Scope;
use komrad_agent::{AgentBehavior, AgentControl, AgentLifecycle, AgentState};
use komrad_ast::prelude::{Channel, ChannelListener, Message, ToSexpr, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::debug;

/// SpawnAgent is a syntax proxy bound as `spawn`.
/// It forwards messages like:
///    spawn agent Bob { ... }
/// as:
///    spawn agent Bob { ... }
/// to the RegistryAgent.
pub struct SpawnAgent {
    registry: Arc<RegistryAgent>,
    channel: Channel,
    listener: Arc<Mutex<ChannelListener>>,
    control_tx: tokio::sync::mpsc::Sender<AgentControl>,
    control_rx: Mutex<tokio::sync::mpsc::Receiver<AgentControl>>,
    state_tx: tokio::sync::watch::Sender<AgentState>,
    state_rx: tokio::sync::watch::Receiver<AgentState>,
}

impl SpawnAgent {
    pub fn new(registry: Arc<RegistryAgent>) -> Arc<Self> {
        let (channel, listener) = Channel::new(32);
        let (control_tx, control_rx) = tokio::sync::mpsc::channel(8);
        let (state_tx, state_rx) = tokio::sync::watch::channel(AgentState::Started);

        Arc::new(Self {
            registry,
            channel,
            listener: Arc::new(Mutex::new(listener)),
            control_tx,
            control_rx: Mutex::new(control_rx),
            state_tx,
            state_rx,
        })
    }
}

#[async_trait::async_trait]
impl AgentBehavior for SpawnAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        // Transform a message like: [Bob, ...] into: [spawn, agent, Bob, ...]
        let mut new_terms = Vec::new();
        new_terms.push(Value::Word("spawn".into()));
        new_terms.push(Value::Word("agent".into()));
        for term in msg.terms() {
            new_terms.push(term.clone());
        }
        let new_msg = Message::new(new_terms, msg.reply_to());
        debug!("⏭️ SpawnAgent {:}", new_msg.to_sexpr().format(0));
        let _ = self.registry.send(new_msg).await;
        true
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for SpawnAgent {
    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
        // We don't have a specific scope for this agent, but we can return a new one.
        Arc::new(Mutex::new(Scope::new()))
    }

    async fn stop(&self) {
        self.control_tx.send(AgentControl::Stop).await.unwrap();
    }

    fn channel(&self) -> &Channel {
        &self.channel
    }

    fn listener(&self) -> &Mutex<ChannelListener> {
        &self.listener
    }

    async fn recv_control(&self) -> Result<AgentControl, komrad_ast::prelude::RuntimeError> {
        let mut control = self.control_rx.lock().await;
        match control.recv().await {
            Some(control) => Ok(control),
            None => Err(komrad_ast::prelude::RuntimeError::ReceiveControlError),
        }
    }

    async fn notify_stopped(&self) {
        // Notify the agent that it has stopped
        let _ = self.state_tx.send(AgentState::Stopped);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_agent::RegistryAgent;
    use komrad_ast::prelude::{Channel, Message, RuntimeError, Value};

    #[tokio::test]
    async fn test_spawn_agent_forwarding_defined() {
        let registry = RegistryAgent::new();
        let _ = registry.clone().spawn();

        // Predefine an agent "Bob" in the registry with a dummy block.
        let block = komrad_ast::prelude::Block::new(vec![]);
        {
            let mut reg_map = registry.registry.write().await;
            reg_map.insert("Bob".to_string(), block);
        }

        let spawn_agent = SpawnAgent::new(registry.clone());
        let spawn_chan = spawn_agent.clone().spawn();

        let (reply_chan, mut reply_listener) = Channel::new(10);
        let msg = Message::new(vec![Value::Word("Bob".into())], Some(reply_chan.clone()));
        spawn_chan.send(msg).await.unwrap();

        let reply = reply_listener.recv().await.unwrap();
        match reply.terms().get(0) {
            Some(Value::Channel(_ch)) => { /* success */ }
            other => panic!("Expected a channel, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_spawn_agent_forwarding_not_defined() {
        let registry = RegistryAgent::new();
        let _ = registry.clone().spawn();

        let spawn_agent = SpawnAgent::new(registry.clone());
        let spawn_chan = spawn_agent.clone().spawn();

        let (reply_chan, mut reply_listener) = Channel::new(10);

        // Send a properly formatted "spawn agent" message with an undefined agent.
        let msg = Message::new(
            vec![
                Value::Word("agent".into()),
                Value::Word("NonExistent".into()),
            ],
            Some(reply_chan.clone()),
        );

        spawn_chan.send(msg).await.unwrap();

        let reply = reply_listener.recv().await.unwrap();

        // Expect an error return because the agent "NonExistent" is not defined.
        assert_eq!(reply.terms(), &[Value::Error(RuntimeError::AgentNotFound)]);
    }
}
