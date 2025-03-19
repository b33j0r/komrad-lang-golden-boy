use crate::prelude::RegistryAgent;
use komrad_agent::execute::Execute;
use komrad_agent::try_bind::TryBind;
use komrad_agent::{AgentBehavior, AgentLifecycle};
use komrad_ast::prelude::{
    Block, Channel, ChannelListener, Handler, Message, RuntimeError, Statement, ToSexpr, Value,
};
use komrad_ast::scope::Scope;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, trace};
use tracing_subscriber::registry;

/// A universal dynamic "module" or "agent" that handles an AST block.
pub struct DynamicAgent {
    name: String,             // Possibly store a name for debugging
    scope: Arc<Mutex<Scope>>, // All variables and data
    handlers: Arc<RwLock<Vec<Handler>>>,
    channel: Channel,
    listener: Arc<ChannelListener>,
}

impl DynamicAgent {
    /// Construct from an AST Block, collecting any Handler statements
    /// and optionally executing others in the scope.
    pub async fn from_block(
        name: &str,
        block: &Block,
        scope: Scope,
        registry_channel: Channel,
    ) -> Arc<Self> {
        let mut scope = scope.clone();
        let (channel, listener) = Channel::new(32);
        let (_default_agents, default_channels) =
            crate::default_agents::DefaultAgents::new(registry_channel.clone());

        scope
            .set("me".to_string(), Value::Channel(channel.clone()))
            .await;
        for (name, channel) in default_channels.get_channels() {
            trace!(
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
            listener: Arc::new(listener),
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

    fn listener(&self) -> Arc<ChannelListener> {
        self.listener.clone()
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
                match result {
                    Value::Bytes(_) => {
                        debug!("DynamicAgent {} -> bytes result", self.name);
                    }
                    _ => {
                        debug!(
                            "DynamicAgent {} -> result: {:}",
                            self.name,
                            result.to_sexpr().format(0)
                        );
                    }
                }
                return true; // handled
            }
        }

        // Check if there is a reply_to channel
        if let Some(reply_to) = msg.reply_to() {
            // If there is a reply_to channel, send an empty message
            let msg_str = msg.to_sexpr().format(0);
            let reply_msg = Message::new(
                vec![Value::Error(RuntimeError::HandlerNotFound(msg_str))],
                None,
            );
            match reply_to.send(reply_msg).await {
                Ok(_) => {
                    debug!("DynamicAgent {} -> empty reply sent", self.name);
                }
                Err(e) => {
                    debug!("DynamicAgent {} -> empty reply error: {:?}", self.name, e);
                }
            }
        }

        // No match, but keep running
        true
    }
}
