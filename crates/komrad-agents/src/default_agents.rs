use crate::agent_agent::AgentAgent;
use crate::io_agent::IoAgent;
use crate::registry_agent::RegistryAgent;
use komrad_ast::prelude::{Agent, Channel};
use std::collections::HashMap;
use std::sync::Arc;

pub struct DefaultAgents {
    pub io_agent: Arc<IoAgent>,
    pub registry_agent: Arc<RegistryAgent>,
    pub agent_agent: Arc<AgentAgent>,
    pub spawn_agent: Arc<AgentAgent>,
}

pub struct DefaultAgentChannels {
    pub io_agent: Channel,
    pub registry_agent: Channel,
    pub agent_agent: Channel,
    pub spawn_agent: Channel,
}

impl DefaultAgents {
    pub fn new() -> (Self, DefaultAgentChannels) {
        let io_agent = IoAgent::default();
        let registry_agent = RegistryAgent::new();
        let agent_agent = AgentAgent::new(registry_agent.clone());
        let spawn_agent = AgentAgent::new(registry_agent.clone());

        let io_agent_channel = io_agent.clone().spawn();
        let registry_agent_channel = registry_agent.clone().spawn();
        let agent_agent_channel = agent_agent.clone().spawn();
        let spawn_agent_channel = spawn_agent.clone().spawn();

        (
            Self {
                io_agent,
                registry_agent,
                agent_agent,
                spawn_agent,
            },
            DefaultAgentChannels {
                io_agent: io_agent_channel,
                registry_agent: registry_agent_channel,
                agent_agent: agent_agent_channel,
                spawn_agent: spawn_agent_channel,
            },
        )
    }
}

impl DefaultAgentChannels {
    pub fn get_channels(&self) -> HashMap<String, Channel> {
        let mut channels = HashMap::new();
        channels.insert("IO".to_string(), self.io_agent.clone());
        channels.insert("Registry".to_string(), self.registry_agent.clone());
        channels.insert("agent".to_string(), self.agent_agent.clone());
        channels.insert("spawn".to_string(), self.spawn_agent.clone());
        channels
    }
}
