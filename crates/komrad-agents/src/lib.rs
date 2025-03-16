mod agent_agent;
mod default_agents;
mod dynamic_agent;
mod fs_agent;
mod io_agent;

#[macro_use]
mod macros;
mod registry_agent;
mod spawn_agent;
mod stdlib_agent;

pub mod prelude {
    pub use crate::agent_agent::AgentAgent;
    pub use crate::default_agents::DefaultAgents;
    pub use crate::dynamic_agent::DynamicAgent;
    pub use crate::io_agent::{IoAgent, StdIo};
    pub use crate::registry_agent::{RegistryAgent, RegistryFactory};
    pub use crate::spawn_agent::SpawnAgent;
}
