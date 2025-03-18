use dashmap::DashMap;
use komrad_agent::{AgentBehavior, AgentLifecycle};
use komrad_agents::prelude::{DynamicAgent, RegistryAgent};
use komrad_ast::prelude::{Block, Channel};
use komrad_ast::scope::Scope;
use std::sync::Arc;

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
        let registry = RegistryAgent::new();
        let registry_channel = registry.clone().spawn();

        let agent = DynamicAgent::from_block(name, block, Scope::new(), registry_channel).await;
        let chan = agent.clone().spawn();
        self.agents.insert(name.into(), agent);
        chan
    }

    pub async fn shutdown(&self) {
        for agent in self.agents.clone().iter() {
            agent.value().stop().await;
            self.agents.remove(agent.key());
        }
    }
}

impl Drop for System {
    fn drop(&mut self) {
        self.shutdown_token.cancel();
    }
}
