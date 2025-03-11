use async_trait::async_trait;
use komrad_ast::prelude::{Channel, ChannelListener, Message, RuntimeError};
use tokio::sync::Mutex;

/// Core trait: requires only the minimal methods.
#[async_trait]
pub trait AgentLifecycle: Send + Sync + 'static {
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

    async fn actor_loop(agent: Arc<Self>, _chan: Channel) {
        while agent.is_running() {
            match Self::recv(&agent).await {
                Ok(msg) => {
                    if !Self::handle_message(&agent, msg).await {
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
