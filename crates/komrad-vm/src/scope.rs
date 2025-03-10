use komrad_ast::prelude::Value;
use std::collections::HashMap;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Scope {
    parent: Option<Box<Scope>>,
    bindings: Arc<RwLock<HashMap<String, Value>>>,
    dirty: bool,
}

impl Scope {
    pub fn new() -> Self {
        Scope {
            parent: None,
            bindings: Arc::new(RwLock::new(HashMap::new())),
            dirty: false,
        }
    }

    pub fn with_parent(parent: Scope) -> Self {
        Scope {
            parent: Some(Box::new(parent)),
            bindings: Arc::new(RwLock::new(HashMap::new())),
            dirty: false,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn get<'a>(
        &'a self,
        name: &'a str,
    ) -> Pin<Box<dyn std::future::Future<Output = Option<Value>> + Send + 'a>> {
        Box::pin(async move {
            let bindings = self.bindings.read().await;
            if let Some(value) = bindings.get(name) {
                return Some(value.clone());
            }
            if let Some(parent) = &self.parent {
                return parent.get(name).await;
            }
            None
        })
    }

    pub async fn set(&mut self, name: String, value: Value) {
        let mut bindings = self.bindings.write().await;
        bindings.insert(name, value);
        self.dirty = true;
    }
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use komrad_ast::prelude::{Number, Value};

    #[tokio::test]
    async fn test_scope() {
        let mut scope = Scope::new();
        scope
            .set("x".to_string(), Value::Number(Number::Float(1.0)))
            .await;
        scope
            .set("y".to_string(), Value::Number(Number::Float(2.0)))
            .await;
        assert_eq!(
            scope.get("x").await,
            Some(Value::Number(Number::Float(1.0))),
        );
    }

    #[tokio::test]
    async fn test_scope_with_parent() {
        let mut parent_scope = Scope::new();
        parent_scope
            .set("x".to_string(), Value::Number(Number::Float(1.0)))
            .await;

        let mut child_scope = Scope::with_parent(parent_scope);
        child_scope
            .set("y".to_string(), Value::Number(Number::Float(2.0)))
            .await;

        assert_eq!(
            child_scope.get("x").await,
            Some(Value::Number(Number::Float(1.0))),
        );
        assert_eq!(
            child_scope.get("y").await,
            Some(Value::Number(Number::Float(2.0))),
        );
    }
}
