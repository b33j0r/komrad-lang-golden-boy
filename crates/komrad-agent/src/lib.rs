#![feature(associated_type_defaults)]

pub mod closure;
pub mod execute;
pub mod try_bind;

pub mod stdlib_agent;

pub use komrad_ast::agent::*;

pub use komrad_macros::agent_lifecycle_impl;
