use komrad_ast::prelude::{Handler, Value};
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Scope {
    parent: Option<Box<Scope>>,
    bindings: Arc<RwLock<HashMap<String, Value>>>,
    handlers: Arc<RwLock<Vec<Arc<Handler>>>>,
    dirty: bool,
}

impl Scope {
    pub fn new() -> Self {
        Scope {
            parent: None,
            bindings: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(Vec::new())),
            dirty: false,
        }
    }

    pub fn with_parent(parent: Scope) -> Self {
        Scope {
            parent: Some(Box::new(parent)),
            bindings: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(Vec::new())),
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

    pub async fn add_handler(&mut self, handler: Arc<Handler>) {
        self.handlers.write().await.push(handler);
        self.dirty = true;
    }

    pub async fn get_handlers(&self) -> Vec<Arc<Handler>> {
        let mut handlers = self.handlers.read().await.clone();
        let mut current_scope = self.parent.as_deref();
        while let Some(scope) = current_scope {
            let parent_handlers = scope.handlers.read().await.clone();
            handlers.extend(parent_handlers);
            current_scope = scope.parent.as_deref();
        }
        handlers
    }
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scope")
            .field("parent", &self.parent)
            // .field("bindings", &self.bindings)
            .field("dirty", &self.dirty)
            .finish()
    }
}

impl Scope {
    // Add this method to your Scope implementation
    pub fn debug_str<'a>(
        &'a self,
    ) -> Pin<Box<dyn std::future::Future<Output = String> + Send + 'a>> {
        Box::pin(async move {
            let mut result = String::new();

            // Access the bindings safely in an async context
            let bindings = self.bindings.read().await;
            for (name, value) in bindings.iter() {
                result.push_str(&format!("{}: {} = {}\n", name, value.get_type(), value));
            }

            // Access parent if it exists
            if let Some(parent) = &self.parent {
                let parent_str = parent.debug_str().await;
                result.push_str(&format!("Parent: {}", parent_str));
            }

            result
        })
    }
}

impl Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let debug_str = futures::executor::block_on(self.debug_str());
        write!(f, "\n{}", debug_str.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use komrad_ast::prelude::*;

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
