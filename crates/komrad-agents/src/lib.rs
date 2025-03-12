mod agent_agent;
mod default_agents;
mod dynamic_agent;
mod io_agent;
mod registry_agent;
mod spawn_agent;

pub mod prelude {
    pub use crate::agent_agent::AgentAgent;
    pub use crate::default_agents::DefaultAgents;
    pub use crate::dynamic_agent::DynamicAgent;
    pub use crate::io_agent::{IoAgent, StdIo};
    pub use crate::registry_agent::RegistryAgent;
    pub use crate::spawn_agent::SpawnAgent;
}
