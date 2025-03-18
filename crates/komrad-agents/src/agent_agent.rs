use crate::registry_agent::RegistryAgent;
use komrad_agent::AgentBehavior;
use komrad_ast::prelude::{Channel, ChannelListener, Message, ToSexpr, Value};
use komrad_macros::agent_lifecycle_impl;
use std::sync::Arc;
use tracing::debug;

/// AgentAgent is a syntax proxy bound as `agent`.
/// It forwards an incoming message such as:
///    agent Alice { ... }
/// as:
///    define agent Alice { ... }
/// to the RegistryAgent.
pub struct AgentAgent {
    registry: Channel,
    channel: Channel,
    listener: Arc<ChannelListener>,
}

impl AgentAgent {
    pub fn new(registry: Channel) -> Arc<Self> {
        let (channel, listener) = Channel::new(32);
        Arc::new(Self {
            registry,
            channel,
            listener: Arc::new(listener),
        })
    }
}

agent_lifecycle_impl!(AgentAgent);

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
        let registry_channel = registry.clone().spawn();

        let agent_agent = AgentAgent::new(registry_channel.clone());
        let agent_chan = agent_agent.clone().spawn();

        let (reply_chan, reply_listener) = Channel::new(10);
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
    }
}
