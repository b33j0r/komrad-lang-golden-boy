use async_trait::async_trait;
use komrad_ast::prelude::{
    Agent, AgentBehavior, BinaryExpr, Channel, ChannelListener, Expr, Message, RuntimeError,
    ToSexpr, Value,
};
use komrad_macros::agent_lifecycle_impl;
use std::sync::Arc;
use tracing::{debug, error};

#[derive(Debug, Clone)]
pub struct AssertAgent {
    channel: Channel,
    listener: Arc<ChannelListener>,
}

impl AssertAgent {
    pub fn new() -> Arc<Self> {
        let (channel, listener) = Channel::new(32);
        Arc::new(Self {
            channel,
            listener: Arc::new(listener),
        })
    }

    pub async fn handle_assert_statement(&self, value: &Value) -> Value {
        match value {
            Value::Boolean(true) => {
                debug!("AssertAgent -> assert true");
                Value::Boolean(true)
            }
            Value::Boolean(false) => {
                error!("AssertAgent -> assert false");
                Value::Boolean(false)
            }
            _ => {
                error!("AssertAgent -> assert error: {:?}", value);
                Value::Error(RuntimeError::AssertionFailed(format!(
                    "Not a boolean value: {:?}",
                    value
                )))
            }
        }
    }
}

agent_lifecycle_impl!(AssertAgent);

#[async_trait]
impl AgentBehavior for AssertAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        let expression = msg.terms().get(0).unwrap_or(&Value::Boolean(true)).clone();

        let result_value = self.handle_assert_statement(&expression).await;

        if let Some(reply_chan) = msg.reply_to() {
            let reply = Message::new(vec![result_value], None);
            match reply_chan.send(reply).await {
                Ok(_) => {
                    debug!("AssertAgent -> reply sent");
                }
                Err(e) => {
                    debug!("AssertAgent -> reply error: {:?}", e);
                }
            }
        }

        true
    }
}

impl Agent for AssertAgent {}

pub struct AssertAgentFactory;
