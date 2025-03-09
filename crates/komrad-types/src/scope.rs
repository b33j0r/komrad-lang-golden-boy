use crate::Value;
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

impl Hash for Scope {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let bindings = self.bindings.blocking_read(); // Using blocking_read() for sync context
        for (key, value) in &*bindings {
            key.hash(state);
            "=".hash(state);
            value.hash(state);
        }
        if let Some(parent) = &self.parent {
            parent.hash(state);
        }
    }
}

impl PartialEq for Scope {
    fn eq(&self, other: &Self) -> bool {
        let self_bindings = self.bindings.blocking_read();
        let other_bindings = other.bindings.blocking_read();

        if *self_bindings != *other_bindings {
            return false;
        }
        if self.parent.is_none() && other.parent.is_none() {
            return true;
        }
        if self.parent.is_some() && other.parent.is_some() {
            return self
                .parent
                .as_ref()
                .unwrap()
                .eq(other.parent.as_ref().unwrap());
        }
        false
    }
}

impl Eq for Scope {}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bindings = self.bindings.blocking_read();
        let mut result = String::new();
        for (key, value) in bindings.iter() {
            result.push_str(&format!("{}: {}\n", key, value));
        }
        if let Some(parent) = &self.parent {
            result.push_str(&format!("Parent: {}", parent));
        }
        write!(f, "{}", result)
    }
}

impl std::fmt::Debug for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Scope {{\n{}\n}}", self)
    }
}
