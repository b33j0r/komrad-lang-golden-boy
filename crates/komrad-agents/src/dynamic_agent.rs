use komrad_agent::execute::Execute;
use komrad_agent::scope::Scope;
use komrad_agent::try_bind::TryBind;
use komrad_agent::{AgentBehavior, AgentLifecycle};
use komrad_ast::prelude::{
    Block, Channel, ChannelListener, Handler, Message, Statement, ToSexpr, Value,
};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::debug;

/// A universal dynamic "module" or "agent" that handles an AST block.
pub struct DynamicAgent {
    name: String,             // Possibly store a name for debugging
    scope: Arc<Mutex<Scope>>, // All variables and data
    handlers: Arc<RwLock<Vec<Handler>>>,
    channel: Channel,
    listener: Arc<Mutex<ChannelListener>>,
    running: Arc<Mutex<bool>>,
}

impl DynamicAgent {
    /// Construct from an AST Block, collecting any Handler statements
    /// and optionally executing others in the scope.
    pub async fn from_block(name: &str, block: &Block, scope: Scope) -> Arc<Self> {
        let mut scope = scope.clone();
        let (channel, listener) = Channel::new(32);
        let (_default_agents, default_channels) = crate::default_agents::DefaultAgents::new();
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
            running: Arc::new(Mutex::new(true)),
        })
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for DynamicAgent {
    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
        self.scope.clone()
    }

    async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
    }

    fn is_running(&self) -> bool {
        match self.running.try_lock() {
            Ok(b) => *b,
            Err(_) => true, // if we can’t lock, assume running
        }
    }

    fn channel(&self) -> &Channel {
        &self.channel
    }

    fn listener(&self) -> &Mutex<ChannelListener> {
        &self.listener
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
                debug!("DynamicAgent {} -> handler result: {:?}", self.name, result);
                return true; // handled
            }
        }

        // No match -> do nothing
        true
    }
}
