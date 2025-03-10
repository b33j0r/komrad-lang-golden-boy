use crate::execute::Execute;
use crate::scope::Scope;
use komrad_ast::prelude::{Message, Statement, Value};
use std::fmt::{Debug, Display};
use std::sync::{Arc, RwLock};
use tokio::sync::{mpsc, oneshot, watch};
use tracing::warn;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleId(pub Uuid);

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
    Execute(Statement),
    QueryScope(oneshot::Sender<Scope>),
}

impl Debug for ModuleCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleCommand::Stop => write!(f, "Stop"),
            ModuleCommand::Send(msg) => write!(f, "Send({:?})", msg),
            ModuleCommand::Execute(stmt) => write!(f, "Execute({:?})", stmt),
            ModuleCommand::QueryScope(_) => write!(f, "QueryScope"),
        }
    }
}

pub struct Module;

#[derive(Debug, Clone)]
pub struct ModuleApi {
    pub id: ModuleId,
    pub name: String,
    command_tx: mpsc::Sender<ModuleCommand>,
}

pub struct ModuleActor {
    pub id: ModuleId,
    pub name: String,
    pub command_rx: mpsc::Receiver<ModuleCommand>,
    scope: Scope,
}

impl Module {
    pub async fn spawn(name: String) -> Arc<ModuleApi> {
        let id = ModuleId::new();
        let (command_tx, command_rx) = mpsc::channel(32);

        let actor = ModuleActor {
            id: id.clone(),
            name: name.clone(),
            command_rx,
            scope: Scope::new(),
        };

        let api = ModuleApi {
            id,
            name,
            command_tx,
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

    pub async fn get_scope(&self) -> Scope {
        let (reply, mut reply_rx) = oneshot::channel();
        self.command_tx
            .send(ModuleCommand::QueryScope(reply))
            .await
            .unwrap();
        match reply_rx.await {
            Ok(scope) => scope,
            Err(_) => {
                warn!("Failed to receive scope from Module {}", self.name);
                Scope::default()
            }
        }
    }
}

impl ModuleActor {
    pub async fn run(mut self) {
        while let Some(command) = self.command_rx.recv().await {
            warn!("Module {} received command: {:?}", self.name, command);
            match command {
                ModuleCommand::Stop => {
                    warn!("Module {} stopped", self.name);
                    break; // Terminate actor on stop command
                }
                ModuleCommand::Send(message) => {
                    // Handle sending a message.
                    warn!("Module {} received message: {:?}", self.name, message);
                }
                ModuleCommand::Execute(statement) => {
                    // Handle executing a statement.
                    warn!("Module {} executing statement: {:?}", self.name, statement);
                    // Execute the statement in the module's scope.
                    if let Value::Error(e) = statement.execute(&mut self.scope).await {
                        warn!("Failed to execute statement in Module {}: {}", self.name, e);
                    }
                }
                ModuleCommand::QueryScope(sender) => {
                    // Send the current scope back to the requester.
                    if let Err(_) = sender.send(self.scope.clone()) {
                        warn!("Failed to send scope back to requester");
                    }
                }
            }
        }
    }
}
