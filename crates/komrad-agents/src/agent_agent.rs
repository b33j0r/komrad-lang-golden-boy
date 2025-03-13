use crate::dynamic_agent::DynamicAgent;
use crate::registry_agent::RegistryAgent;
use komrad_agent::scope::Scope;
use komrad_agent::{AgentBehavior, AgentControl, AgentLifecycle, AgentState};
use komrad_ast::prelude::{Channel, ChannelListener, Message, ToSexpr, Value};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;
use tracing::debug;

/// AgentAgent is a syntax proxy bound as `agent`.
/// It forwards an incoming message such as:
///    agent Alice { ... }
/// as:
///    define agent Alice { ... }
/// to the RegistryAgent.
pub struct AgentAgent {
    registry: Arc<RegistryAgent>,
    channel: Channel,
    listener: Arc<Mutex<ChannelListener>>,
    control_rx: Mutex<mpsc::Receiver<AgentControl>>,
    control_tx: mpsc::Sender<AgentControl>,
    state_tx: tokio::sync::watch::Sender<AgentState>,
    state_rx: tokio::sync::watch::Receiver<AgentState>,
}

impl AgentAgent {
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
impl AgentLifecycle for AgentAgent {
    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
        // We don't have a specific scope for this agent, but we can return a new one.
        Arc::new(Mutex::new(Scope::new()))
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

#[async_trait::async_trait]
impl AgentBehavior for AgentAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        // Transform a message like: [Alice, <block>]
        // into: [define, agent, Alice, <block>]
        let mut new_terms = Vec::new();
        new_terms.push(Value::Word("define".into()));
        new_terms.push(Value::Word("agent".into()));
        for term in msg.terms() {
            new_terms.push(term.clone());
        }
        let new_msg = Message::new(new_terms, msg.reply_to());
        debug!("⏭️ AgentAgent {:}", new_msg.to_sexpr().format(0));
        let _ = self.registry.send(new_msg).await;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_agent::RegistryAgent;
    use komrad_ast::prelude::{Block, Message, Statement, Value};

    #[tokio::test]
    async fn test_agent_agent_define_forwarding() {
        let registry = RegistryAgent::new();
        let _ = registry.clone().spawn();

        let agent_agent = AgentAgent::new(registry.clone());
        let agent_chan = agent_agent.clone().spawn();

        let (reply_chan, mut reply_listener) = Channel::new(10);
        let block = Block::new(vec![Statement::NoOp]);
        let msg = Message::new(
            vec![
                Value::Word("Alice".into()),
                Value::Block(Box::new(block.clone())),
            ],
            Some(reply_chan.clone()),
        );
        agent_chan.send(msg).await.unwrap();

        let reply = reply_listener.recv().await.unwrap();
        // On success, the registry replies with the Word("Alice")
        assert_eq!(reply.terms(), &[Value::Word("Alice".into())]);

        let reg_map = registry.registry.read().await;
        assert!(reg_map.contains_key("Alice"));
        assert_eq!(reg_map.get("Alice").unwrap(), &block);
    }
}
