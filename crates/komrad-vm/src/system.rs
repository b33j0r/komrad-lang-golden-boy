use dashmap::DashMap;
use komrad_agent::scope::Scope;
use komrad_agent::AgentBehavior;
use komrad_agents::prelude::DynamicAgent;
use komrad_ast::prelude::{Block, Channel};
use std::sync::Arc;
use std::time::Duration;

// system.rs
pub struct System {
    agents: DashMap<String, Arc<DynamicAgent>>,
    shutdown_token: tokio_util::sync::CancellationToken,
}

impl System {
    pub fn new() -> Self {
        Self {
            agents: DashMap::new(),
            shutdown_token: tokio_util::sync::CancellationToken::new(),
        }
    }

    pub async fn create_agent(&self, name: &str, block: &Block) -> Channel {
        let agent =
            DynamicAgent::from_block(name, block, Scope::new(), self.shutdown_token.clone()).await;
        let chan = agent.clone().spawn();
        self.agents.insert(name.into(), agent);
        chan
    }
}

impl Drop for System {
    fn drop(&mut self) {
        // Cancel all agents
        self.shutdown_token.cancel();
        // yield to allow all agents to finish
        tokio::task::block_in_place(|| tokio::time::sleep(Duration::from_millis(100)));
    }
}
