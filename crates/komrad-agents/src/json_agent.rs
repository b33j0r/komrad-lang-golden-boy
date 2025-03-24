use komrad_agent::{Agent, AgentBehavior};
use komrad_ast::prelude::{Channel, ChannelListener, Message, Value};
use komrad_macros::agent_stateless_impl;
use serde_json;
use std::sync::Arc;
use tracing::{error, info};

pub struct JsonAgent {
    channel: Channel,
    listener: Arc<ChannelListener>,
}

agent_stateless_impl!(JsonAgent);

#[async_trait::async_trait]
impl AgentBehavior for JsonAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        let cmd = match msg.first_word() {
            Some(w) => w,
            None => return true,
        };

        match cmd.as_str() {
            "encode" => {
                if let Some(value) = msg.terms().get(1) {
                    match serde_json::to_string(value) {
                        Ok(json_str) => {
                            reply_if_possible(&msg, Value::String(json_str)).await;
                        }
                        Err(e) => {
                            error!("encode: failed to serialize value: {}", e);
                            reply_if_possible(&msg, Value::String(format!("Error: {}", e))).await;
                        }
                    }
                } else {
                    error!("encode: missing argument");
                }
                true
            }

            "decode" => {
                if let Some(Value::String(json_str)) = msg.terms().get(1) {
                    match serde_json::from_str::<Value>(json_str) {
                        Ok(val) => {
                            reply_if_possible(&msg, val).await;
                        }
                        Err(e) => {
                            error!("decode: failed to parse JSON: {}", e);
                            reply_if_possible(&msg, Value::String(format!("Error: {}", e))).await;
                        }
                    }
                } else {
                    error!("decode: argument not a string");
                }
                true
            }

            other => {
                error!("JsonAgent: unknown command: {}", other);
                true
            }
        }
    }
}

impl Agent for JsonAgent {}

/// Helper: Sends a single-value message if there's a reply channel.
async fn reply_if_possible(msg: &Message, value: Value) {
    if let Some(reply_chan) = msg.reply_to() {
        let response = Message::new(vec![value], None);
        if let Err(e) = reply_chan.send(response).await {
            error!("JsonAgent: failed to send reply: {}", e);
        }
    }
}
