use std::fmt::Display;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleId(pub Uuid);

impl ModuleId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
