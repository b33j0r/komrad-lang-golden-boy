use crate::channel::Channel;
use crate::error::RuntimeError;
use crate::message::Message;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait Agent: Send + Sync + 'static {
    /// Spawn this actor as a new background task.
    /// Returns the channel that others can use to send messages.
    fn spawn(self: Arc<Self>) -> Channel;

    /// Send a message to this actor.
    async fn send(&self, msg: Message) -> Result<(), RuntimeError>;

    /// Stop the actor gracefully.
    async fn stop(&self);

    /// Check if the actor is still running.
    fn is_running(&self) -> bool;

    /// Hook: Called when a message is received.
    /// Return `true` if you want to keep running, `false` to break the loop.
    async fn handle_message(&self, msg: Message) -> bool {
        let _ = msg; // default: do nothing
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::Channel;
    use crate::error::RuntimeError;
    use crate::message::Message;
    use async_trait::async_trait;
    use std::sync::{Arc, RwLock};
    use tokio::time::{sleep, Duration};

    /// A simple test agent that prints received messages.
    struct TestAgent {
        name: String,
        running: Arc<RwLock<bool>>,
    }

    impl TestAgent {
        fn new(name: String) -> Self {
            Self {
                name,
                running: Arc::new(RwLock::new(true)),
            }
        }
    }

    #[async_trait]
    impl Agent for TestAgent {
        fn spawn(self: Arc<Self>) -> Channel {
            // Create a channel with a default capacity (assume Channel::new() exists without parameters)
            let (tx, mut rx) = Channel::new(32);
            let agent = self.clone();
            tokio::spawn(async move {
                // Loop while the agent is running.
                while agent.is_running() {
                    if let Ok(msg) = rx.recv().await {
                        // For test purposes, simply print the message.
                        println!("{} received message: {:?}", agent.name, msg);
                        // Let the agent decide if it should continue running.
                        let _ = agent.handle_message(msg).await;
                    } else {
                        // Channel closed.
                        break;
                    }
                }
                println!("{} has stopped.", agent.name);
            });
            tx
        }

        async fn send(&self, _msg: Message) -> Result<(), RuntimeError> {
            // For testing, this method is not used; we send via the channel.
            Ok(())
        }

        async fn stop(&self) {
            let mut running = self.running.write().unwrap();
            *running = false;
        }

        fn is_running(&self) -> bool {
            *self.running.read().unwrap()
        }

        async fn handle_message(&self, msg: Message) -> bool {
            // For testing, simply print the message.
            println!("{} handling message: {:?}", self.name, msg);
            true
        }
    }

    #[tokio::test]
    async fn test_agent_receives_message() {
        let agent = Arc::new(TestAgent::new("TestAgent".into()));
        let channel = agent.clone().spawn();

        // Create a test message.
        let msg = Message::new(vec!["Test message".into()], None);
        channel.send(msg).await.unwrap();

        // Give the agent some time to process the message.
        sleep(Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_agent_stop() {
        let agent = Arc::new(TestAgent::new("TestAgent".into()));
        let channel = agent.clone().spawn();

        // Ensure the agent is running.
        assert!(agent.is_running(), "Agent should be running after spawn.");

        // Stop the agent.
        agent.stop().await;

        // Give the agent some time to stop.
        sleep(Duration::from_millis(50)).await;
        assert!(
            !agent.is_running(),
            "Agent should be stopped after calling stop()."
        );

        // Optionally, close the channel to ensure the background task can exit.
        drop(channel);
        sleep(Duration::from_millis(10)).await;
    }
}
