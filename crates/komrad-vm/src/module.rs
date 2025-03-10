use crate::execute::Execute;
use crate::scope::Scope;
use komrad_agents::io_agent::IoAgent;
use komrad_ast::prelude::{Agent, Channel, ChannelListener, Message, Statement, Value};
use std::fmt::{Debug, Display};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};
use tracing::{debug, warn};
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
                message = self.channel_listener.recv() => {
                    match message {
                        Ok(message) => {
                            warn!("Module {} received message: {:?}", self.name, message);
                            // Handle incoming messages.
                            if let Some(reply_to) = message.reply_to() {
                                let reply_message = Message::new(vec![Value::String("ack".into())], None);
                                if let Err(e) = reply_to.send(reply_message).await {
                                    warn!("Failed to send reply: {}", e);
                                }
                            }
                        },
                        Err(e) => {
                            warn!("Module {} failed to receive message: {}", self.name, e);
                            break; // Exit the loop if the channel is closed.
                        }
                    }
                },
                command = self.command_rx.recv() => {
                    match command {
                        Some(command) => {
                            warn!("Module {} received command: {:?}", self.name, command);
                            match command {
                                ModuleCommand::Stop => {
                                    warn!("Module {} received Stop command", self.name);
                                    break;
                                }
                                ModuleCommand::Send(message) => {
                                    // Handle sending a message.
                                    warn!("Module {} received message: {:?}", self.name, message);
                                }
                                ModuleCommand::ExecuteStatement(statement) => {
                                    // Handle executing a statement.
                                    warn!("Module {} executing statement: {:?}", self.name, statement);
                                    // Execute the statement in the module's scope.
                                    if let Value::Error(e) = statement.execute(&mut self.scope).await {
                                        warn!("Failed to execute statement in Module {}: {}", self.name, e);
                                    }
                                }
                                ModuleCommand::ExecuteStatements(statements) => {
                                    // Handle executing multiple statements.
                                    for statement in statements {
                                        warn!("Module {} executing statement: {:?}", self.name, statement);
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
                                    warn!("Module {} modified scope: {} = {:?}", self.name, key, value);
                                }
                            }
                        },
                        None => break, // Command channel closed unexpectedly.
                    }
                },
            }
        }
    }
}
