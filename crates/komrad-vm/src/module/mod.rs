use komrad_agent::scope::Scope;
use komrad_ast::prelude::{Channel, Value};
use module_actor::ModuleActor;
use module_api::ModuleApi;
use module_command::ModuleCommand;
use module_id::ModuleId;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, warn};

pub mod module_actor;
pub mod module_api;
pub mod module_command;
pub mod module_id;

pub struct Module;

impl Module {
    pub async fn spawn(name: String, capacity: usize) -> Arc<ModuleApi> {
        debug!("Creating Module for {}", name);
        let id = ModuleId::new();
        let (command_tx, command_rx) = mpsc::channel(32);

        let mut module_scope = Scope::new();

        let (_default_agents, default_agent_channels) =
            komrad_agents::default_agents::DefaultAgents::new();

        for (agent_name, agent_channel) in default_agent_channels.get_channels() {
            module_scope
                .set(agent_name.to_string(), Value::Channel(agent_channel))
                .await;
        }

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
