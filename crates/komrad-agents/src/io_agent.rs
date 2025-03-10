use async_trait::async_trait;
use komrad_ast::prelude::Agent;
use komrad_ast::prelude::Message;
use komrad_ast::prelude::RuntimeError;
use komrad_ast::prelude::{Channel, ChannelListener, Value};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task;
use tracing::{debug, info, warn};

/// **IoInterface** trait for pluggable IO.
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

/// **IoAgent** is an IO actor that implements `Agent`.
/// - It listens for `"println"` or `"shutdown"` commands.
/// - It uses an internal “running” flag to track state.
/// - It uses a `ChannelListener` in the background to handle messages.
pub struct IoAgent {
    io_interface: Arc<RwLock<dyn IoInterface>>,
    running: Arc<Mutex<bool>>,
    channel: Channel, // We'll store our sending handle
    listener: Arc<Mutex<ChannelListener>>,
}

impl IoAgent {
    /// Creates a new IO Agent with the given IoInterface.
    pub fn new(io_interface: Arc<RwLock<dyn IoInterface>>) -> Arc<Self> {
        let (chan, listener) = Channel::new(32);
        Arc::new(Self {
            io_interface,
            running: Arc::new(Mutex::new(true)),
            channel: chan,
            listener: Arc::new(Mutex::new(listener)),
        })
    }

    /// Convenience constructor that uses `StdIo`.
    pub fn default() -> Arc<Self> {
        Self::new(Arc::new(RwLock::new(StdIo)))
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

#[async_trait]
impl Agent for IoAgent {
    fn spawn(self: Arc<Self>) -> Channel {
        // We'll clone ourselves so the task can own the Arc
        let me = self.clone();

        info!("Spawning IoAgent task...");
        task::spawn(async move {
            debug!("IoAgent task started.");

            loop {
                // Check if still running
                if !me.is_running() {
                    debug!("IoAgent says 'running=false', stopping loop.");
                    break;
                }

                // Try receiving a message
                let mut guard = me.listener.lock().await;
                match guard.recv().await {
                    Ok(msg) => {
                        // Let `handle_message` decide if we keep going
                        let keep_going = me.handle_message(msg).await;
                        if !keep_going {
                            debug!("Actor decided to stop after handle_message returned false.");
                            break;
                        }
                    }
                    Err(_) => {
                        warn!("Channel closed, stopping actor.");
                        break;
                    }
                }
            }
            debug!("IoAgent task exited.");
        });

        // Return the sending handle so others can do: agent.send(...)
        self.channel.clone()
    }

    async fn send(&self, msg: Message) -> Result<(), RuntimeError> {
        self.channel.send(msg).await
    }

    async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
    }

    fn is_running(&self) -> bool {
        // It's non-async, so we do a "try_lock" or "blocking lock" carefully:
        //   - If blocking, this is short & safe
        //   - Or we can define `fn is_running_async(...) -> ...`
        match self.running.try_lock() {
            Ok(r) => *r,
            Err(_) => false, // If we can't lock, we assume "still running"
        }
    }

    /// The intelligence: decide how to handle each message.
    async fn handle_message(&self, msg: Message) -> bool {
        if let Some(cmd) = msg.first_word() {
            match cmd.as_str() {
                "println" => {
                    self.handle_println(&msg).await;
                    return true;
                }
                "shutdown" => {
                    warn!("IO agent received shutdown command, stopping.");
                    // We'll set running=false and return false to break the loop:
                    self.stop().await;
                    return false;
                }
                other => {
                    warn!("Unknown IO command: {:?}", other);
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
