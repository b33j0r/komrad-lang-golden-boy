use crate::registry_agent::RegistryAgent;
use komrad_ast::prelude::{Agent, Channel, ChannelListener, Message, RuntimeError, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

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
    running: Arc<Mutex<bool>>,
}

impl AgentAgent {
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
impl Agent for AgentAgent {
    fn spawn(self: Arc<Self>) -> Channel {
        let chan = self.channel.clone();
        let agent = self.clone();
        tokio::spawn(async move {
            while agent.is_running() {
                let msg_result = {
                    let mut listener = agent.listener.lock().await;
                    listener.recv().await
                };
                if let Ok(msg) = msg_result {
                    let _ = agent.handle_message(msg).await;
                } else {
                    break;
                }
            }
        });
        chan
    }

    async fn send(&self, msg: Message) -> Result<(), RuntimeError> {
        self.channel.send(msg).await
    }

    async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
    }

    fn is_running(&self) -> bool {
        match self.running.try_lock() {
            Ok(lock) => *lock,
            Err(_) => true,
        }
    }

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
        let _ = self.registry.send(new_msg).await;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_agent::RegistryAgent;
    use komrad_ast::prelude::{Agent, Block, Message, Statement, Value};

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
        // On success, the registry replies with "defined"
        assert_eq!(reply.terms(), &[Value::String("defined".into())]);

        let reg_map = registry.registry.read().await;
        assert!(reg_map.contains_key("Alice"));
        assert_eq!(reg_map.get("Alice").unwrap(), &block);
    }
}
