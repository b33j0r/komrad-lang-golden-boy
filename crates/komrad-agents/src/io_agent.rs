use crate::agent_agent::AgentAgent;
use komrad_agent::scope::Scope;
use komrad_agent::{AgentBehavior, AgentControl, AgentLifecycle, AgentState};
use komrad_ast::prelude::Message;
use komrad_ast::prelude::{Channel, ChannelListener, Value};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

/// **IoInterface** trait for pluggable Io.
pub trait IoInterface: Send + Sync {
    fn print(&mut self, msg: &str);
    fn println(&mut self, msg: &str);
}

/// **StdIo**: A default (console) IoInterface implementation.
#[derive(Debug, Clone)]
pub struct StdIo;

impl IoInterface for StdIo {
    fn print(&mut self, msg: &str) {
        print!("{}", msg);
    }
    fn println(&mut self, msg: &str) {
        println!("{}", msg);
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

    /// **Helper**: actual logic for "println" commands.
    async fn handle_println(&self, msg: &Message) {
        let output: Vec<String> = msg.terms()[1..]
            .iter()
            .map(|part| match part {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Boolean(b) => b.to_string(),
                Value::Channel(ch) => format!("Channel: {}", ch.uuid()),
                Value::Embedded(b) => b.text().to_string(),
                _ => format!("Unknown: {:?}", part),
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
}

#[async_trait::async_trait]
impl AgentLifecycle for IoAgent {
    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
        // We don't have a specific scope for this agent, but we can return a new one.
        Arc::new(Mutex::new(Scope::new()))
    }

    fn channel(&self) -> &Channel {
        &self.channel
    }

    fn listener(&self) -> Arc<ChannelListener> {
        self.listener.clone()
    }
}

#[async_trait::async_trait]
impl AgentBehavior for IoAgent {
    /// The intelligence: decide how to handle each message.
    async fn handle_message(&self, msg: Message) -> bool {
        if let Some(cmd) = msg.first_word() {
            match cmd.as_str() {
                "println" => {
                    self.handle_println(&msg).await;
                    return true;
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
    async fn test_orca_io_agent_println() {
        let agent = IoAgent::default();
        let agent_chan = agent.spawn();

        // Another channel to receive ack
        let (ack_chan, mut ack_listener) = Channel::new(10);
        let msg = Message::new(
            vec![
                Value::Word("println".into()),
                Value::String("Hello, Orca!".into()),
            ],
            Some(ack_chan.clone()),
        );

        agent_chan.send(msg).await.unwrap();
        let ack = ack_listener.recv().await.unwrap();
        assert_eq!(ack.terms(), &[Value::String("ack".into())]);
    }

    #[tokio::test]
    async fn test_orca_io_agent_shutdown() {
        let agent = IoAgent::default();
        let spawned_agent = agent.clone();
        let chan = spawned_agent.spawn();

        // Check running
        assert!(agent.is_running());

        // Send shutdown
        let stop_msg = Message::new(vec![Value::Word("shutdown".into())], None);
        chan.send(stop_msg).await.unwrap();

        // Let the agent process the shutdown
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(!agent.is_running(), "Agent should have stopped by now.");
    }
}
