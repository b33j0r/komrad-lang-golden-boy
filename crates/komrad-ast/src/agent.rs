use crate::prelude::{
    Channel, ChannelListener, ControlMessage, Message, RuntimeError, ToSexpr, Value,
};
use crate::scope::Scope;
use async_trait::async_trait;

use std::sync::{mpsc, Arc};
use tokio::select;
use tokio::sync::{watch, Mutex};
use tracing::{debug, error, info, trace};

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
    async fn init(self: Arc<Self>, _scope: &mut Scope) {}
    async fn get_scope(&self) -> Arc<Mutex<Scope>>;

    /// Stops this agent and all agents in its scope. Calls `stop_in_scope`.
    async fn stop(&self) {
        self.stop_in_scope().await;
    }

    /// This is our current method of "garbage collection" for agents.
    async fn stop_in_scope(&self) {
        // send stop message to all channels in scope
        let scope = self.get_scope().await;
        for (_, channel) in scope.lock().await.iter() {
            match channel {
                Value::Channel(chan) => {
                    trace!(
                        "Sending ControlMessage::Stop over channel: {}",
                        chan.to_sexpr().format(0)
                    );
                    match chan.control(ControlMessage::Stop).await {
                        Ok(_) => {}
                        Err(e) => {
                            trace!("Error sending Stop message: {:?}", e);
                        }
                    }
                }
                _ => {
                    // skip non-channel values
                }
            }
        }
        match self.channel().control(ControlMessage::Stop).await {
            Ok(_) => {}
            Err(e) => {
                info!("Error sending Stop message to SELF: {:?}", e);
            }
        }
    }

    fn channel(&self) -> &Channel;
    fn listener(&self) -> Arc<ChannelListener>;
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

    async fn extra_event_source(&self) -> Option<Message> {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        None
    }

    async fn handle_extra_message(&self, _msg: Message) {}

    async fn actor_loop(self: Arc<Self>, _chan: Channel) {
        debug!(
            "Starting actor loop for agent {}",
            self.channel().to_sexpr().format(0)
        );

        // Init with scope
        // IMPORTANT: only hold lock while initializing
        {
            let scope = self.clone().get_scope().await;
            let mut scope = scope.lock().await;
            trace!("Initializing agent");
            self.clone().init(&mut scope).await
        };

        loop {
            // The listener has internal locking that allows us to await
            // both recv and recv_control without deadlocking.
            let listener = self.listener().clone();
            let extra_event_source = self.extra_event_source();
            select! {
                msg = extra_event_source => {
                    if let Some(msg) = msg {
                        // Handle the extra event source message
                        trace!("Received extra event source message: {:?}", msg);
                        self.handle_extra_message(msg).await;
                    }
                }
                // Receive a komrad message from the channel
                msg = listener.recv() => match msg {
                    Ok(msg) => {
                        if !Self::handle_message(&self, msg).await {
                            break;
                        }
                    }
                    Err(_) => break,
                },
                // Receive a control message (just stop for now)
                msg = listener.recv_control() => match msg {
                    Ok(msg) => {
                        match msg {
                            ControlMessage::Stop => {
                                debug!("Received Stop message");
                                self.stop().await;
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error receiving control message: {:?}", e);
                        break
                    },
                }
            }
        }
        trace!("Agent loop exited");
    }

    async fn send(&self, msg: Message) -> Result<(), RuntimeError> {
        self.channel().send(msg).await
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
