use crate::execute::Execute;
use crate::scope::Scope;
use crate::try_bind::TryBind;
use komrad_agents::io_agent::IoAgent;
use komrad_ast::prelude::{Agent, Channel, ChannelListener, Message, Statement, Value};
use std::fmt::{Debug, Display};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub struct Module;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleId(pub Uuid);

#[derive(Debug, Clone)]
pub struct ModuleApi {
    pub id: ModuleId,
    pub name: String,
    command_tx: mpsc::Sender<ModuleCommand>,
    channel: Channel,
}

pub struct ModuleActor {
    pub id: ModuleId,
    pub name: String,
    pub command_rx: mpsc::Receiver<ModuleCommand>,
    scope: Scope,
    channel_listener: ChannelListener,
}

impl ModuleId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub enum ModuleCommand {
    Stop,
    Send(Message),
    ExecuteStatement(Statement),
    ExecuteStatements(Vec<Statement>),
    QueryScope(oneshot::Sender<Scope>),
    ModifyScope { key: String, value: Value },
}

impl Debug for ModuleCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleCommand::Stop => write!(f, "Stop"),
            ModuleCommand::Send(msg) => write!(f, "Send({:?})", msg),
            ModuleCommand::ExecuteStatement(stmt) => write!(f, "Execute({:?})", stmt),
            ModuleCommand::ExecuteStatements(stmts) => {
                write!(f, "ExecuteStatements({:?})", stmts)
            }
            ModuleCommand::QueryScope(_) => write!(f, "QueryScope"),
            ModuleCommand::ModifyScope { key, value } => {
                write!(f, "ModifyScope({:?}, {:?})", key, value)
            }
        }
    }
}

impl Module {
    pub async fn spawn(name: String, capacity: usize) -> Arc<ModuleApi> {
        let id = ModuleId::new();
        let (command_tx, command_rx) = mpsc::channel(32);
        let (exit_tx, exit_rx) = watch::channel(()); // exit signal channel
        let (exited_tx, exited_rx) = watch::channel(()); // exited confirmation channel

        let mut module_scope = Scope::new();
        let io_actor = IoAgent::default();
        let io_actor_spawned = io_actor.clone();
        let io_actor_chan = io_actor_spawned.spawn();

        module_scope
            .set("IO".to_string(), Value::Channel(io_actor_chan))
            .await;

        let (channel, channel_listener) = Channel::new(capacity);

        let actor = ModuleActor {
            id: id.clone(),
            name: name.clone(),
            command_rx,
            scope: module_scope,
            channel_listener,
        };

        let api = ModuleApi {
            id,
            name,
            command_tx,
            channel: channel.clone(),
        };
        let api = Arc::new(api);

        warn!("Created ModuleApi for {} with ID {}", api.name, api.id);

        tokio::spawn(async move {
            actor.run().await;
        });

        api
    }
}

impl ModuleApi {
    pub async fn send_command(&self, command: ModuleCommand) {
        warn!("Sending command to Module {}: {:?}", self.name, command);
        if let Err(e) = self.command_tx.send(command).await {
            warn!("Failed to send command to Module {}: {}", self.name, e);
        }
    }

    pub async fn get_scope(&self) -> Option<Scope> {
        let (reply, mut reply_rx) = oneshot::channel();
        match self.command_tx.send(ModuleCommand::QueryScope(reply)).await {
            Err(e) => {
                warn!(
                    "Failed to send QueryScope command to Module {}: {}",
                    self.name, e
                );
                return None;
            }
            Ok(_) => {
                debug!("Sent QueryScope command to Module {}", self.name);
            }
        }
        match reply_rx.await {
            Ok(scope) => Some(scope),
            Err(_) => {
                warn!("Failed to receive scope from Module {}", self.name);
                None
            }
        }
    }

    pub fn get_channel(&self) -> Channel {
        self.channel.clone()
    }
}

impl ModuleActor {
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                maybe_msg = self.channel_listener.recv() => {
                    match maybe_msg {
                        Ok(message) => {
                            info!("Module {} received message: {:?}", self.name, message);

                            // 2) Dispatch the message by matching all handlers
                            let result = self.dispatch_message(message).await;
                            if let Value::Error(e) = result {
                                warn!("Failed to handle message in Module {}: {}", self.name, e);
                            }
                        },
                        Err(e) => {
                            warn!("Module {} failed to receive message: {}", self.name, e);
                            break;
                        }
                    }
                },
                command = self.command_rx.recv() => {
                    match command {
                        Some(command) => {
                            info!("Module {} received command: {:?}", self.name, command);
                            match command {
                                ModuleCommand::Stop => {
                                    info!("Module {} received Stop command", self.name);
                                    break;
                                }
                                ModuleCommand::Send(message) => {
                                    // Handle sending a message.
                                    info!("Module {} received message: {:?}", self.name, message);
                                }
                                ModuleCommand::ExecuteStatement(statement) => {
                                    // Handle executing a statement.
                                    info!("Module {} executing statement: {:?}", self.name, statement);
                                    // Execute the statement in the module's scope.
                                    if let Value::Error(e) = statement.execute(&mut self.scope).await {
                                        warn!("Failed to execute statement in Module {}: {}", self.name, e);
                                    }
                                }
                                ModuleCommand::ExecuteStatements(statements) => {
                                    // Handle executing multiple statements.
                                    for statement in statements {
                                        info!("Module {} executing statement: {:?}", self.name, statement);
                                        if let Value::Error(e) = statement.execute(&mut self.scope).await {
                                            warn!("Failed to execute statement in Module {}: {}", self.name, e);
                                        }
                                    }
                                }
                                ModuleCommand::QueryScope(sender) => {
                                    // Send the current scope back to the requester.
                                    if let Err(_) = sender.send(self.scope.clone()) {
                                        warn!("Failed to send scope back to requester");
                                    }
                                }
                                ModuleCommand::ModifyScope { key, value } => {
                                    // Modify the module's scope.
                                    self.scope.set(key.clone(), value.clone()).await;
                                    info!("Module {} modified scope: {} = {:?}", self.name, key, value);
                                }
                            }
                        },
                        None => break, // Command channel closed unexpectedly.
                    }
                },
            }
        }
    }

    /// Dispatches a single incoming message by matching all handlers in scope.
    /// Returns whatever the matched handler’s block evaluates to, or Value::Empty if no match.
    async fn dispatch_message(&mut self, message: Message) -> Value {
        // 1) Grab the list of handlers from the scope
        let handlers = self.scope.get_handlers().await;

        // 2) Try each handler’s pattern
        for handler in handlers {
            debug!("Checking handler: {:?}", handler);
            let pattern = handler.pattern();
            if let Some(mut scope) = pattern.try_bind(message.clone(), &mut self.scope).await {
                // 3) If the pattern matches, execute the block
                let block = handler.block();
                let result = block.execute(&mut scope).await;
                debug!("Handler executed successfully, result: {:?}", result);
                return result;
            } else {
                debug!("Pattern did not match: {:?}", pattern);
            }
        }

        warn!("No handler matched for message: {:?}", message.terms());
        Value::Empty
    }
}
