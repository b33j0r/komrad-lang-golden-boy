#![feature(associated_type_defaults)]

mod agent;
mod closure;
pub mod execute;
pub mod scope;
pub mod try_bind;

pub use agent::{Agent, AgentBehavior, AgentLifecycle};
