use dashmap::DashMap;
use komrad_agent::AgentBehavior;
use komrad_agents::prelude::DynamicAgent;
use komrad_ast::prelude::{Block, Channel};
use std::sync::Arc;

// system.rs
pub struct System {
    agents: DashMap<String, Arc<DynamicAgent>>,
}

impl System {
    pub fn new() -> Self {
        Self {
            agents: DashMap::new(),
        }
    }

    pub async fn create_agent(&self, name: &str, block: &Block) -> Channel {
        let agent = DynamicAgent::from_block(name, block, None).await;
        let chan = agent.clone().spawn();
        self.agents.insert(name.into(), agent);
        chan
    }
}
