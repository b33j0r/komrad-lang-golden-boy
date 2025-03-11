use crate::Scope;
use komrad_ast::prelude::{Message, Statement, Value};
use std::fmt::Debug;
use tokio::sync::oneshot;

pub enum ModuleCommand {
    Stop,
    Send(Message),
    ExecuteStatement(Statement),
    ExecuteStatements(Vec<Statement>),
    QueryScope(oneshot::Sender<Scope>),
    ModifyScope { key: String, value: Value },
}

impl Debug for ModuleCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleCommand::Stop => write!(f, "Stop"),
            ModuleCommand::Send(msg) => write!(f, "Send({:?})", msg),
            ModuleCommand::ExecuteStatement(stmt) => write!(f, "Execute({:?})", stmt),
            ModuleCommand::ExecuteStatements(stmts) => {
                write!(f, "ExecuteStatements({:?})", stmts)
            }
            ModuleCommand::QueryScope(_) => write!(f, "QueryScope"),
            ModuleCommand::ModifyScope { key, value } => {
                write!(f, "ModifyScope({:?}, {:?})", key, value)
            }
        }
    }
}
