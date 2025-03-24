use crate::agent_agent::AgentAgent;
use crate::assert_agent::AssertAgent;
use crate::fs_agent::FsAgent;
use crate::io_agent::IoAgent;
use crate::json_agent::JsonAgent;
use crate::prelude::StdIo;
use crate::spawn_agent::SpawnAgent;
use komrad_agent::stdlib_agent::DictAgent;
use komrad_agent::AgentBehavior;
use komrad_ast::prelude::Channel;
use std::collections::HashMap;
use std::sync::Arc;

pub struct DefaultAgents {
    pub io_agent: Arc<IoAgent>,
    pub fs_agent: Arc<FsAgent>,
    pub agent_agent: Arc<AgentAgent>,
    pub spawn_agent: Arc<SpawnAgent>,
    pub assert_agent: Arc<AssertAgent>,
    pub dict_agent: Arc<DictAgent>,
    pub json_agent: Arc<JsonAgent>,
}

pub struct DefaultAgentChannels {
    pub io_agent: Channel,
    pub fs_agent: Channel,
    pub registry_agent: Channel,
    pub agent_agent: Channel,
    pub spawn_agent: Channel,
    pub assert_agent: Channel,
    pub dict_agent: Channel,
    pub json_agent: Channel,
}

/// The channels for each agent constructed within `DefaultAgents`
/// is injected into every new DynamicAgent.
///
/// - `Io` is the IO agent.
/// - `Fs` is the file system agent.
/// - `Registry` is the registry agent.
/// - `agent` is the agent keyword in Komrad (everything is agents!)
/// - `spawn` is the spawn keyword in Komrad (for spawning agents).
///
/// They are organized here to provide a single source of truth.
///
/// One future direction is to use a configuration system to enable
/// or disable certain agents. (I like the way starlark does this.)
impl DefaultAgents {
    pub fn new(registry_channel: Channel) -> (Self, DefaultAgentChannels) {
        let io_agent = IoAgent::new(Arc::new(tokio::sync::RwLock::new(StdIo)));
        let fs_agent = FsAgent::new();
        let agent_agent = AgentAgent::new(registry_channel.clone());
        let spawn_agent = SpawnAgent::new(registry_channel.clone());
        let assert_agent = AssertAgent::new();
        let dict_agent = DictAgent::new();
        let json_agent = JsonAgent::new();

        let io_agent_channel = io_agent.clone().spawn();
        let fs_agent_channel = fs_agent.clone().spawn();
        let agent_agent_channel = agent_agent.clone().spawn();
        let spawn_agent_channel = spawn_agent.clone().spawn();
        let assert_agent_channel = assert_agent.clone().spawn();
        let dict_agent_channel = dict_agent.clone().spawn();
        let json_agent_channel = json_agent.clone().spawn();

        (
            Self {
                io_agent,
                fs_agent,
                agent_agent,
                spawn_agent,
                assert_agent,
                dict_agent,
                json_agent,
            },
            DefaultAgentChannels {
                io_agent: io_agent_channel,
                fs_agent: fs_agent_channel,
                registry_agent: registry_channel,
                agent_agent: agent_agent_channel,
                spawn_agent: spawn_agent_channel,
                assert_agent: assert_agent_channel,
                dict_agent: dict_agent_channel,
                json_agent: json_agent_channel,
            },
        )
    }
}

impl DefaultAgentChannels {
    /// Used to enumerate the channels by scope alias
    /// (i.e. variable name) for the default agents.
    ///
    /// This decouples the caller from knowing anything
    /// about the default agents and default namespace.
    pub fn get_channels(&self) -> HashMap<String, Channel> {
        let mut channels = HashMap::new();

        // Built-in Agents
        channels.insert("Io".to_string(), self.io_agent.clone());
        channels.insert("Fs".to_string(), self.fs_agent.clone());
        channels.insert("Registry".to_string(), self.registry_agent.clone());

        // Special Agents (Keywords)
        channels.insert("agent".to_string(), self.agent_agent.clone());
        channels.insert("spawn".to_string(), self.spawn_agent.clone());
        channels.insert("assert".to_string(), self.assert_agent.clone());
        channels.insert("dict".to_string(), self.dict_agent.clone());
        channels.insert("json".to_string(), self.json_agent.clone());

        // Return the map of channels
        channels
    }
}
