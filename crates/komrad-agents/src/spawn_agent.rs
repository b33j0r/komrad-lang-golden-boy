use crate::agent_agent::AgentAgent;
use crate::registry_agent::RegistryAgent;
use komrad_agent::{AgentBehavior, AgentLifecycle};
use komrad_ast::prelude::{Channel, ChannelListener, Message, RuntimeError, ToSexpr, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

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
    running: Arc<Mutex<bool>>,
}

impl SpawnAgent {
    pub fn new(registry: Arc<RegistryAgent>) -> Arc<Self> {
        let (channel, listener) = Channel::new(32);
        Arc::new(Self {
            registry,
            channel,
            listener: Arc::new(Mutex::new(listener)),
            running: Arc::new(Mutex::new(true)),
        })
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for SpawnAgent {
    async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
    }

    fn is_running(&self) -> bool {
        match self.running.try_lock() {
            Ok(guard) => *guard,
            Err(_) => false,
        }
    }

    fn channel(&self) -> &Channel {
        &self.channel
    }

    fn listener(&self) -> &Mutex<ChannelListener> {
        &self.listener
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
        info!(
            "😍SpawnAgent sending message: {:?}",
            new_msg.to_sexpr().format(0)
        );
        let _ = self.registry.send(new_msg).await;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_agent::RegistryAgent;
    use komrad_ast::prelude::{Channel, Message, Value};

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
