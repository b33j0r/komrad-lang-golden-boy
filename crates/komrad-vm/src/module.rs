use std::fmt::Display;
use tokio::sync::mpsc;
use tracing::warn;
use uuid::Uuid;
use komrad_types::Msg;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModuleCommand {
    Start,
    Stop,
    Restart,
    Execute(Msg),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModuleStatus {
    Started,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleEvent {
    pub id: ModuleId,
    pub status: ModuleStatus,
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
    pub event_tx: mpsc::Sender<ModuleEvent>,
}

impl Module {
    pub async fn spawn(name: String, event_tx: mpsc::Sender<ModuleEvent>) -> ModuleApi {
        let id = ModuleId::new();
        let (command_tx, command_rx) = mpsc::channel(32);

        let api = ModuleApi {
            id: id.clone(),
            name,
            command_tx,
        };

        let actor = ModuleActor {
            id,
            name: api.name.clone(),
            command_rx,
            event_tx,
        };

        tokio::spawn(async move {
            actor.run().await;
        });

        api
    }
}

impl ModuleApi {
    pub async fn send_command(&self, command: ModuleCommand) {
        if let Err(e) = self.command_tx.send(command).await {
            warn!("Failed to send command to Module {}: {}", self.name, e);
        }
    }
}

impl ModuleActor {
    pub async fn run(mut self) {
        while let Some(command) = self.command_rx.recv().await {
            match command {
                ModuleCommand::Start => {
                    warn!("Module {} started", self.name);
                    if let Err(e) = self.event_tx.send(ModuleEvent {
                        id: self.id.clone(),
                        status: ModuleStatus::Started,
                    }).await {
                        warn!("Failed to send start event for Module {}: {}", self.name, e);
                    }
                }
                ModuleCommand::Stop => {
                    warn!("Module {} stopped", self.name);
                    if let Err(e) = self.event_tx.send(ModuleEvent {
                        id: self.id.clone(),
                        status: ModuleStatus::Stopped,
                    }).await {
                        warn!("Failed to send stop event for Module {}: {}", self.name, e);
                    }
                    break; // Terminate actor on stop command
                }
                ModuleCommand::Restart => {
                    warn!("Module {} restarted", self.name);
                    // Optionally implement restart logic here.
                }
                ModuleCommand::Execute(msg) => {
                    warn!("Module {} executed message: {:?}", self.name, msg);
                    // Optionally handle message execution here.
                }
            }
        }
    }
}
