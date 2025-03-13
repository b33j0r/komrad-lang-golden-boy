use komrad_agent::execute::Execute;
use komrad_agent::scope::Scope;
use komrad_agent::try_bind::TryBind;
use komrad_agent::{AgentBehavior, AgentControl, AgentLifecycle, AgentState};
use komrad_ast::prelude::{
    Block, Channel, ChannelListener, Handler, Message, Statement, ToSexpr, Value,
};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{debug, trace};

/// A universal dynamic "module" or "agent" that handles an AST block.
pub struct DynamicAgent {
    name: String,             // Possibly store a name for debugging
    scope: Arc<Mutex<Scope>>, // All variables and data
    handlers: Arc<RwLock<Vec<Handler>>>,
    channel: Channel,
    listener: Arc<Mutex<ChannelListener>>,
    control_tx: tokio::sync::mpsc::Sender<AgentControl>,
    control_rx: Mutex<tokio::sync::mpsc::Receiver<AgentControl>>,
    state_tx: tokio::sync::watch::Sender<AgentState>,
    state_rx: tokio::sync::watch::Receiver<AgentState>,
}

impl DynamicAgent {
    /// Construct from an AST Block, collecting any Handler statements
    /// and optionally executing others in the scope.
    pub async fn from_block(name: &str, block: &Block, scope: Scope) -> Arc<Self> {
        let mut scope = scope.clone();
        let (channel, listener) = Channel::new(32);
        let (_default_agents, default_channels) = crate::default_agents::DefaultAgents::new();
        let (control_tx, control_rx) = tokio::sync::mpsc::channel(8);
        let (state_tx, state_rx) = tokio::sync::watch::channel(AgentState::Started);

        scope
            .set("me".to_string(), Value::Channel(channel.clone()))
            .await;
        for (name, channel) in default_channels.get_channels() {
            debug!(
                "DynamicAgent: adding default channel {} -> {:?}",
                name, channel
            );
            scope
                .set(name.clone(), Value::Channel(channel.clone()))
                .await;
        }

        let mut collected_handlers = Vec::new();

        // We already have scope from any initial scope block, but now we need to
        // extend this with the scope from the agent's definition block.
        // This is also where HANDLERS are collected:
        for stmt in block.statements() {
            match stmt {
                Statement::Handler(h) => {
                    collected_handlers.push((**h).clone());
                }
                _ => {
                    let _ = stmt.execute(&mut scope).await;
                }
            }
        }

        Arc::new(Self {
            name: name.to_string(),
            scope: Arc::new(Mutex::new(scope)),
            handlers: Arc::new(RwLock::new(collected_handlers)),
            channel,
            listener: Arc::new(Mutex::new(listener)),
            control_tx,
            control_rx: Mutex::new(control_rx),
            state_tx,
            state_rx,
        })
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for DynamicAgent {
    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
        self.scope.clone()
    }

    fn channel(&self) -> &Channel {
        &self.channel
    }

    fn listener(&self) -> &Mutex<ChannelListener> {
        &self.listener
    }

    async fn recv_control(&self) -> Result<AgentControl, komrad_ast::prelude::RuntimeError> {
        let mut control = self.control_rx.lock().await;
        match control.recv().await {
            Some(control) => Ok(control),
            None => Err(komrad_ast::prelude::RuntimeError::ReceiveControlError),
        }
    }

    async fn notify_stopped(&self) {
        // Notify the agent that it has stopped
        let _ = self.state_tx.send(AgentState::Stopped);
    }
}

#[async_trait::async_trait]
impl AgentBehavior for DynamicAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        debug!("😎 {} handling {:}", self.name, msg.to_sexpr().format(0));

        // Copy out the handlers once, to avoid repeated locking
        let local_handlers = self.handlers.read().await.clone();

        // Lock the scope
        let mut base_scope = self.scope.lock().await.clone();

        // Pattern match against each handler
        for h in &local_handlers {
            if let Some(mut bound) = h.pattern().try_bind(msg.clone(), &mut base_scope).await {
                let block = h.block();
                let result = block.execute(&mut bound).await;
                if let Some(reply_to) = msg.reply_to() {
                    let reply_msg = Message::new(vec![result.clone()], None);
                    match reply_to.send(reply_msg).await {
                        Ok(_) => {
                            debug!("DynamicAgent {} -> reply sent", self.name);
                        }
                        Err(e) => {
                            debug!("DynamicAgent {} -> reply error: {:?}", self.name, e);
                        }
                    }
                }
                trace!("DynamicAgent {} -> handler result: {:?}", self.name, result);
                return true; // handled
            }
        }

        // No match -> do nothing
        true
    }
}
