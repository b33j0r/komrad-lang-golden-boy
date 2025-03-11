// or wherever your `Execute` trait is
use komrad_agent::scope::Scope;
use komrad_agent::try_bind::TryBind;
use komrad_agent::{AgentBehavior, AgentLifecycle};

use komrad_agent::execute::Execute;
use komrad_ast::prelude::{Block, Channel, ChannelListener, Handler, Message, Statement};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::error;

/// DynamicAgent is a generic agent that holds a set of handlers
/// extracted from an AST Block, and a runtime Scope for executing them.
pub struct DynamicAgent {
    scope: Arc<Mutex<Scope>>,
    handlers: Arc<RwLock<Vec<Handler>>>,
    channel: Channel,
    channel_listener: Arc<Mutex<ChannelListener>>,
    running: Arc<Mutex<bool>>,
}

impl DynamicAgent {
    /// Construct a DynamicAgent from a pre-parsed AST Block.
    /// We scan the block for Handler(...) statements, store them,
    /// and keep the rest of the statements in the scope if needed.
    pub async fn from_block(block: &Block) -> Arc<Self> {
        let (channel, listener) = Channel::new(32);

        let mut scope = Scope::new();
        let mut collected_handlers = Vec::new();

        // We interpret the block to gather all Handler statements, plus
        // optionally run any "immediate" statements. For simplicity, let's
        // just store the Handler statements.
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
            scope: Arc::new(Mutex::new(scope)),
            handlers: Arc::new(RwLock::new(collected_handlers)),
            channel,
            channel_listener: Arc::new(Mutex::new(listener)),
            running: Arc::new(Mutex::new(true)),
        })
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for DynamicAgent {
    async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
    }

    fn is_running(&self) -> bool {
        match self.running.try_lock() {
            Ok(lock) => *lock,
            Err(_) => true,
        }
    }

    fn channel(&self) -> &Channel {
        &self.channel
    }

    fn listener(&self) -> &Mutex<ChannelListener> {
        &self.channel_listener
    }
}

#[async_trait::async_trait]
impl AgentBehavior for DynamicAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        // 1) Clone the list of handlers so we can iterate without locking each time
        let all_handlers = { self.handlers.read().await.clone() };

        // 2) For each handler, try to pattern-match the incoming message
        for handler in all_handlers {
            let pattern = handler.pattern();
            // We'll need a reference to the "base" scope
            let mut base_scope = self.scope.lock().await.clone();

            if let Some(mut bound_scope) = pattern.try_bind(msg.clone(), &mut base_scope).await {
                // 3) If it matches, run the handler’s block in that bound scope
                //    (merging in any global definitions if needed)
                let block = handler.block();
                let result = block.execute(&mut bound_scope).await;

                error!("DynamicAgent: handler result: {:?}", result);

                return true;
            }
        }

        // If no handler matched, do nothing special.
        true
    }
}
