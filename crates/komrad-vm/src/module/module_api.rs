use crate::module::module_command::ModuleCommand;
use crate::module::module_id::ModuleId;
use crate::scope::Scope;
use komrad_ast::prelude::{Channel, ToSexpr};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct ModuleApi {
    pub id: ModuleId,
    pub name: String,
    pub(crate) command_tx: mpsc::Sender<ModuleCommand>,
    pub(crate) channel: Channel,
}

impl ModuleApi {
    pub async fn send_command(&self, command: ModuleCommand) {
        info!("📡 {} -> {:}", self.name, command.to_sexpr().format(0));
        if let Err(e) = self.command_tx.send(command).await {
            warn!("Failed to send command to Module {}: {}", self.name, e);
        }
    }

    pub async fn get_scope(&self) -> Option<Scope> {
        let (reply, reply_rx) = oneshot::channel();
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
