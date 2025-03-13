use crate::scope::Scope;
use async_trait::async_trait;
use komrad_ast::prelude::{Channel, ChannelListener, Message, RuntimeError};
use std::sync::Arc;
use tokio::select;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

/// Core trait: requires only the minimal methods.
#[async_trait]
pub trait AgentLifecycle: Send + Sync + 'static {
    async fn init(self: Arc<Self>, _scope: &mut Scope) -> Option<JoinHandle<()>> {
        None
    }
    async fn get_scope(&self) -> Arc<Mutex<Scope>>;

    // we still need this for when the agent is dropped.
    // maybe it has a global cancellation token AND
    // a local cancellation token.
    async fn stop(&self) {
        self.local_cancellation_token().cancel();
    }

    /// Returns a global cancellation token for this agent to select! on.
    fn global_cancellation_token(&self) -> CancellationToken;

    /// Returns a local cancellation token for this agent to select! on.
    fn local_cancellation_token(&self) -> CancellationToken;

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
        let join_handle = {
            let scope = self.clone().get_scope().await;
            let mut scope = scope.lock().await;
            self.clone().init(&mut scope).await
        };

        let global_cancel = self.global_cancellation_token();
        let local_cancel = self.local_cancellation_token();

        loop {
            let recv_on_channel = self.recv();
            select! {
                msg = recv_on_channel => match msg {
                    Ok(msg) => {
                        if !Self::handle_message(&self, msg).await {
                            break;
                        }
                    }
                    Err(_) => break,
                },
                _ = global_cancel.cancelled() => {
                    self.local_cancellation_token().cancel();
                    warn!("Agent stopped");
                }
                _ = local_cancel.cancelled() => {
                    if let Some(ref handle) = join_handle {
                        info!("Stopping join handle as part of cancellation");
                        handle.abort();
                    }
                    break;
                }
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
    fn create_agent(
        &self,
        name: &str,
        initial_scope: Scope,
        global_cancellation_token: CancellationToken,
    ) -> Arc<dyn Agent>;
}
