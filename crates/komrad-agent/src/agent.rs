use async_trait::async_trait;
use komrad_ast::prelude::{Channel, ChannelListener, Message, RuntimeError};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Core trait: requires only the minimal methods.
#[async_trait]
pub trait AgentLifecycle: Send + Sync + 'static {
    async fn init(self: Arc<Self>) {
        // Default: do nothing
    }
    async fn stop(&self);
    fn is_running(&self) -> bool;
    fn channel(&self) -> &Channel;
    fn listener(&self) -> &Mutex<ChannelListener>;
}

/// Extension trait providing default implementations.
#[async_trait]
pub trait AgentBehavior: AgentLifecycle {
    fn spawn(self: Arc<Self>) -> Channel {
        let chan = self.channel().clone();
        let agent = self.clone();
        tokio::spawn(Self::actor_loop(agent, chan.clone()));
        chan
    }

    async fn actor_loop(self: Arc<Self>, _chan: Channel) {
        self.clone().init().await;
        while self.is_running() {
            match Self::recv(&self).await {
                Ok(msg) => {
                    if !Self::handle_message(&self, msg).await {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    }

    async fn send(&self, msg: Message) -> Result<(), RuntimeError> {
        self.channel().send(msg).await
    }

    async fn recv(&self) -> Result<Message, RuntimeError> {
        let mut listener = self.listener().lock().await;
        listener.recv().await
    }

    async fn handle_message(&self, msg: Message) -> bool {
        let _ = msg; // default: do nothing
        true
    }
}

pub trait Agent: AgentLifecycle + AgentBehavior {}

pub trait AgentFactory: Send + Sync + 'static {
    fn create_agent(&self, name: &str) -> Arc<dyn Agent>;
}
