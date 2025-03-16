#![feature(associated_type_defaults)]

mod agent;
pub mod closure;
pub mod execute;
pub mod scope;
pub mod try_bind;

#[macro_use]
pub mod macros;
pub mod stdlib_agent;

pub use agent::*;
