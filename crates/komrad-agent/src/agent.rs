use crate::scope::Scope;
use async_trait::async_trait;
use komrad_ast::prelude::{Channel, ChannelListener, Message, RuntimeError, ToSexpr};
use std::sync::{mpsc, Arc};
use tokio::select;
use tokio::sync::{watch, Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

pub enum AgentControl {
    Stop,
}

pub enum AgentState {
    Started,
    Stopped,
}

pub struct AgentData {
    pub name: String,
    pub scope: Arc<Mutex<Scope>>,
    pub channel: Channel,
    pub listener: Mutex<ChannelListener>,
    pub control_rx: mpsc::Receiver<AgentControl>,
    pub state_tx: watch::Sender<AgentState>,
}

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
    async fn stop(&self) {}

    fn channel(&self) -> &Channel;
    fn listener(&self) -> &Mutex<ChannelListener>;

    async fn recv_control(&self) -> Result<AgentControl, RuntimeError>;

    async fn notify_stopped(&self);
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
        info!(
            "Starting actor loop for agent {}",
            self.channel().to_sexpr().format(0)
        );
        let join_handle = {
            let scope = self.clone().get_scope().await;
            let mut scope = scope.lock().await;
            self.clone().init(&mut scope).await
        };

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
                _ = self.recv_control() => {
                    info!("Received control message");
                    if let Some(ref handle) = join_handle {
                        info!("Stopping join handle as part of cancellation");
                        handle.abort();
                    }
                    self.notify_stopped();
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
    fn create_agent(&self, name: &str, initial_scope: Scope) -> Arc<dyn Agent>;
}
