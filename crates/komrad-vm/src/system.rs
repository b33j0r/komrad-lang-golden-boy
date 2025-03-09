use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::mpsc;
use tracing::warn;
use crate::module::{Module, ModuleApi, ModuleEvent, ModuleId, ModuleStatus};

pub struct System {
    // Consolidate module lookup into a single DashMap keyed by module name.
    module_map: Arc<DashMap<String, ModuleApi>>,
    event_tx: mpsc::Sender<ModuleEvent>,
    statuses: Arc<DashMap<ModuleId, ModuleStatus>>,
}

impl System {
    pub async fn spawn() -> Self {
        let (event_tx, mut event_rx) = mpsc::channel::<ModuleEvent>(32);

        let statuses = Arc::new(DashMap::new());
        let actor_statuses = statuses.clone();

        // Listen for module events to update statuses.
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                let id = event.id;
                let status = event.status;

                actor_statuses.insert(id.clone(), status.clone());

                match status {
                    ModuleStatus::Started => {
                        warn!("Module {} started", id);
                    }
                    ModuleStatus::Stopped => {
                        warn!("Module {} stopped", id);
                    }
                }
            }
        });

        System {
            module_map: Arc::new(DashMap::new()),
            event_tx,
            statuses,
        }
    }

    pub async fn create_module(&mut self, name: &str) -> ModuleApi {
        let api = Module::spawn(name.to_string(), self.event_tx.clone()).await;
        self.module_map.insert(api.name.clone(), api.clone());
        api
    }

    // Lookup a module by its ModuleId by iterating over stored modules.
    pub fn get_module_by_id(&self, id: &ModuleId) -> Option<ModuleApi> {
        self.module_map.iter().find_map(|entry| {
            if entry.value().id == *id {
                Some(entry.value().clone())
            } else {
                None
            }
        })
    }

    // Get the current status of a module.
    pub fn get_status(&self, id: &ModuleId) -> Option<ModuleStatus> {
        self.statuses.get(id).map(|s| s.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    use tracing_subscriber;

    #[tokio::test]
    async fn test_module_lifecycle() {
        // Initialize logging to see debug messages.
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .try_init();

        let mut system = System::spawn().await;
        let module = system.create_module("lifecycle_test").await;

        // Send a Start command and wait a bit for event propagation.
        module.send_command(crate::module::ModuleCommand::Start).await;
        sleep(Duration::from_millis(100)).await;

        let status = system.get_status(&module.id);
        assert_eq!(status, Some(crate::module::ModuleStatus::Started));

        // Send a Stop command and wait for the actor to terminate.
        module.send_command(crate::module::ModuleCommand::Stop).await;
        sleep(Duration::from_millis(100)).await;

        let status = system.get_status(&module.id);
        assert_eq!(status, Some(crate::module::ModuleStatus::Stopped));
    }

    #[tokio::test]
    async fn test_get_module_by_id() {
        let mut system = System::spawn().await;
        let module = system.create_module("lookup_test").await;

        let fetched = system.get_module_by_id(&module.id);
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "lookup_test");
    }
}
