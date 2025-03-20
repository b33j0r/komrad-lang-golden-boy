use komrad_agent::{AgentBehavior, AgentLifecycle};
use komrad_ast::prelude::Message;
use komrad_ast::prelude::{Channel, ChannelListener, Value};
use komrad_macros::agent_lifecycle_impl;
use owo_colors::colored::Color;
use owo_colors::OwoColorize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

/// **IoInterface** trait for pluggable Io.
pub trait IoInterface: Send + Sync {
    fn print(&mut self, msg: &str);
    fn println(&mut self, msg: &str);
}

/// **StdIo**: A default (console) IoInterface implementation.
#[derive(Debug, Clone)]
pub struct StdIo;

const STDIO_COLOR: Color = Color::BrightGreen;

impl IoInterface for StdIo {
    fn print(&mut self, msg: &str) {
        print!("{}", msg.color(STDIO_COLOR));
    }

    fn println(&mut self, msg: &str) {
        println!("{}", msg.color(STDIO_COLOR));
    }
}

/// **IoAgent** is an Io actor that implements `Agent`.
/// - It listens for `"println"` or `"shutdown"` commands.
/// - It uses an internal “running” flag to track state.
/// - It uses a `ChannelListener` in the background to handle messages.
pub struct IoAgent {
    io_interface: Arc<RwLock<dyn IoInterface>>,
    channel: Channel, // We'll store our sending handle
    listener: Arc<ChannelListener>,
}

impl IoAgent {
    /// Creates a new Io Agent with the given IoInterface.
    pub fn new(io_interface: Arc<RwLock<dyn IoInterface>>) -> Arc<Self> {
        let (chan, listener) = Channel::new(32);
        Arc::new(Self {
            io_interface,
            channel: chan,
            listener: Arc::new(listener),
        })
    }

    #[allow(dead_code)]
    fn default() -> Arc<Self> {
        let io_interface = Arc::new(RwLock::new(StdIo));
        let (chan, listener) = Channel::new(32);
        Arc::new(Self {
            io_interface,
            channel: chan,
            listener: Arc::new(listener),
        })
    }

    /// **Helper**: actual logic for "println" commands.
    async fn handle_println(&self, msg: &Message) {
        let output: Vec<String> = msg.terms()[1..]
            .iter()
            .map(|part| match part {
                Value::List(l) => {
                    let mut s = String::new();
                    s.push_str("[");
                    let inner_s = l
                        .iter()
                        .map(|v| match v {
                            Value::String(s) => s.clone(),
                            Value::Number(n) => n.to_string(),
                            Value::Boolean(b) => b.to_string(),
                            Value::Channel(ch) => format!("Channel: {}", ch.uuid()),
                            Value::Embedded(b) => b.text().to_string(),
                            _ => format!("(no formatter: {:?})", v),
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    s.push_str(&inner_s);
                    s.push_str("]");
                    s
                }
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Boolean(b) => b.to_string(),
                Value::Channel(ch) => format!("Channel: {}", ch.uuid()),
                Value::Embedded(b) => b.text().to_string(),
                _ => format!("(no formatter: {:?})", part),
            })
            .collect();

        {
            let mut io = self.io_interface.write().await;
            for line in output {
                io.println(&line);
            }
        }

        // Acknowledge if there's a reply channel
        if let Some(reply_chan) = msg.reply_to() {
            let ack = Message::new(vec![Value::String("ack".into())], None);
            let _ = reply_chan.send(ack).await;
        }
    }

    /// **Helper**: actual logic for "print" commands.
    async fn handle_print(&self, msg: &Message) {
        let output: Vec<String> = msg.terms()[1..]
            .iter()
            .map(|part| match part {
                Value::List(l) => {
                    let mut s = String::new();
                    s.push_str("[");
                    let inner_s = l
                        .iter()
                        .map(|v| match v {
                            Value::String(s) => s.clone(),
                            Value::Number(n) => n.to_string(),
                            Value::Boolean(b) => b.to_string(),
                            Value::Channel(ch) => format!("Channel: {}", ch.uuid()),
                            Value::Embedded(b) => b.text().to_string(),
                            _ => format!("(no formatter: {:?})", v),
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    s.push_str(&inner_s);
                    s.push_str("]");
                    s
                }
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Boolean(b) => b.to_string(),
                Value::Channel(ch) => format!("Channel: {}", ch.uuid()),
                Value::Embedded(b) => b.text().to_string(),
                _ => format!("(no formatter: {:?})", part),
            })
            .collect();

        {
            let mut io = self.io_interface.write().await;
            for part in output {
                io.print(&part);
            }
        }

        // Acknowledge if there's a reply channel
        if let Some(reply_chan) = msg.reply_to() {
            let ack = Message::new(vec![Value::String("ack".into())], None);
            let _ = reply_chan.send(ack).await;
        }
    }
}

agent_lifecycle_impl!(IoAgent);

#[async_trait::async_trait]
impl AgentBehavior for IoAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        if let Some(cmd) = msg.first_word() {
            match cmd.as_str() {
                "println" => {
                    self.handle_println(&msg).await;
                    return true;
                }
                "print" => {
                    self.handle_print(&msg).await;
                }
                "shutdown" => {
                    warn!("Io agent received shutdown command, stopping.");
                    // We'll set running=false and return false to break the loop:
                    self.stop().await;
                    return false;
                }
                other => {
                    warn!("Unknown Io command: {:?}", other);
                    return true; // Keep running
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;

    #[tokio::test]
    async fn test_io_agent() {
        let io_agent = IoAgent::default();
        let io_chan = io_agent.clone().spawn();

        // Send a println message
        let (reply_chan, reply_listener) = Channel::new(10);
        let msg = Message::new(
            vec![
                Value::Word("println".into()),
                Value::String("Hello, World!".into()),
            ],
            Some(reply_chan.clone()),
        );
        let _ = io_chan.send(msg).await;

        // Wait for the reply
        tokio::time::sleep(Duration::from_millis(100)).await;
        let reply = reply_listener.recv().await.unwrap();
        assert_eq!(reply.terms()[0], Value::String("ack".into()));
    }
}
