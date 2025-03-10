use crate::module::{Module, ModuleApi, ModuleId};
use dashmap::DashMap;
use std::sync::Arc;

pub struct System {
    module_map: Arc<DashMap<String, Arc<ModuleApi>>>,
}

impl System {
    pub async fn spawn() -> Self {
        Self {
            module_map: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_module(&self, name: &str) -> Arc<ModuleApi> {
        let api = Module::spawn(name.to_string()).await;
        self.module_map.insert(api.name.clone(), api.clone());
        tokio::task::yield_now().await;
        api
    }

    pub fn get_module_by_id(&self, id: &ModuleId) -> Option<Arc<ModuleApi>> {
        self.module_map.iter().find_map(|entry| {
            if entry.value().id == *id {
                Some(entry.value().clone())
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber;

    #[tokio::test]
    async fn test_get_module_by_id() {
        let mut system = System::spawn().await;
        let module = system.create_module("lookup_test").await;

        let fetched = system.get_module_by_id(&module.id);
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "lookup_test");
    }
}
