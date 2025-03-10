use komrad_ast::prelude::{Channel, Message, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task;
use tracing::{debug, error, info, warn};

/// Defines an interface for IO operations.
pub trait IoInterface: Send + Sync {
    fn print(&mut self, message: &str);
    fn println(&mut self, message: &str);
}

/// **StdIo: Default Console IO Implementation**
#[derive(Debug, Clone)]
pub struct StdIo;

impl IoInterface for StdIo {
    fn print(&mut self, message: &str) {
        print!("{}", message);
    }
    fn println(&mut self, message: &str) {
        println!("{}", message);
    }
}

/// **IO Agent listens for println/shutdown commands**
pub struct IoAgent {
    io_interface: Arc<RwLock<dyn IoInterface>>,
}

impl Default for IoAgent {
    fn default() -> IoAgent {
        Self {
            io_interface: Arc::new(RwLock::new(StdIo)),
        }
    }
}

impl IoAgent {
    /// **Creates a new IO Agent with a shared IO interface.**
    pub fn new(io_interface: Arc<RwLock<dyn IoInterface>>) -> Self {
        Self { io_interface }
    }

    pub fn spawn_default() -> Channel {
        let io_interface = Arc::new(RwLock::new(StdIo));
        let agent = IoAgent::new(io_interface);
        agent.spawn()
    }

    /// **Spawns the agent and returns its command channel.**
    fn spawn(self) -> Channel {
        let (agent_chan, mut agent_rx) = Channel::new(32);
        let io_interface = self.io_interface.clone();

        info!("Spawning IO agent...");

        task::spawn(async move {
            debug!("IO agent started.");

            loop {
                tokio::select! {
                    msg = agent_rx.recv() => {
                        match msg {
                            Ok(msg) => {
                                if let Some(first_word) = msg.first_word() {
                                    match first_word.as_str() {
                                        "println" => {
                                            IoAgent::handle_println(io_interface.clone(), &msg).await;
                                        }
                                        "shutdown" => {
                                            warn!("IO agent received shutdown command.");
                                            break;
                                        }
                                        _ => {
                                            warn!("Unknown IO command: {:?}", msg);
                                        }
                                    }
                                }
                            }
                            Err(_) => {
                                warn!("IO agent detected channel closure, exiting.");
                                break;
                            }
                        }
                    }
                }
            }
            debug!("IO agent exited.");
        });

        agent_chan
    }

    /// **Handles println messages.**
    async fn handle_println(io_interface: Arc<RwLock<dyn IoInterface>>, msg: &Message) {
        let output: Vec<String> = msg.terms()[1..]
            .iter()
            .map(|part| match part {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Boolean(b) => b.to_string(),
                Value::Channel(c) => format!("Channel: {}", c.uuid()),
                _ => format!("Unknown type: {:?}", part),
            })
            .collect();

        {
            let mut io = io_interface.write().await;
            for line in &output {
                io.println(line);
            }
        } // ✅ Lock is dropped here before sending ACK

        if let Some(reply_chan) = msg.reply_to() {
            let ack_msg = Message::new(vec![Value::String("ack".into())], None);
            let _ = reply_chan.send(ack_msg).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// **Mock IO for Testing**
    pub struct MockIo {
        pub printed: String,
    }

    #[async_trait]
    impl IoInterface for MockIo {
        fn print(&mut self, msg: &str) {
            print!("{}", msg);
            self.printed.push_str(msg);
        }
        fn println(&mut self, msg: &str) {
            println!("{}", msg);
            self.printed.push_str(msg);
            self.printed.push('\n');
        }
    }

    #[tokio::test]
    async fn test_io_agent_println() {
        let mock_io = Arc::new(RwLock::new(MockIo {
            printed: String::new(),
        }));
        let agent = IoAgent::new(mock_io.clone());
        let agent_chan = agent.spawn();

        // Channel for receiving ACKs
        let (ack_chan, mut ack_rx) = Channel::new(10);

        // Send `println "Hello, World!"`
        let test_msg = Message::new(
            vec![
                Value::Word("println".into()),
                Value::String("Hello, World!".into()),
            ],
            Some(ack_chan.clone()),
        );

        agent_chan.send(test_msg).await.unwrap();

        // ✅ Wait for ACK
        let ack = ack_rx.recv().await.expect("Should get ack");
        assert_eq!(ack.terms(), &[Value::String("ack".into())]);

        // ✅ Verify output
        assert_eq!(mock_io.read().await.printed, "Hello, World!\n");
    }

    #[tokio::test]
    async fn test_io_agent_multiple_prints() {
        let mock_io = Arc::new(RwLock::new(MockIo {
            printed: String::new(),
        }));
        let agent = IoAgent::new(mock_io.clone());
        let agent_chan = agent.spawn();

        let (ack_chan, mut ack_rx) = Channel::new(10);

        let test_msg = Message::new(
            vec![
                Value::Word("println".into()),
                Value::String("Hello".into()),
                Value::String("World".into()),
            ],
            Some(ack_chan.clone()),
        );

        agent_chan.send(test_msg).await.unwrap();

        let ack = ack_rx.recv().await.expect("Should get ack");
        assert_eq!(ack.terms(), &[Value::String("ack".into())]);

        assert_eq!(mock_io.read().await.printed, "Hello\nWorld\n");
    }

    #[tokio::test]
    async fn test_io_agent_shutdown() {
        let mock_io = Arc::new(RwLock::new(MockIo {
            printed: String::new(),
        }));
        let agent = IoAgent::new(mock_io.clone());
        let agent_chan = agent.spawn();

        let shutdown_msg = Message::new(vec![Value::Word("shutdown".into())], None);
        agent_chan.send(shutdown_msg).await.unwrap();

        tokio::task::yield_now().await;

        drop(agent_chan);

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}
