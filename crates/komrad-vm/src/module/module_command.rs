use crate::Scope;
use komrad_ast::prelude::{Message, Sexpr, Statement, ToSexpr, Value};
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

impl ToSexpr for ModuleCommand {
    fn to_sexpr(&self) -> Sexpr {
        match self {
            ModuleCommand::Stop => Sexpr::Atom("stop".to_string()),
            ModuleCommand::Send(msg) => msg.to_sexpr(),
            ModuleCommand::ExecuteStatement(stmt) => stmt.to_sexpr(),
            ModuleCommand::ExecuteStatements(stmts) => {
                let sexprs: Vec<Sexpr> = stmts.iter().map(|s| s.to_sexpr()).collect();
                Sexpr::List(sexprs)
            }
            ModuleCommand::QueryScope(_) => Sexpr::Atom("query_scope".to_string()),
            ModuleCommand::ModifyScope { key, value } => Sexpr::List(vec![
                Sexpr::Atom("modify_scope".to_string()),
                Sexpr::Atom(key.clone()),
                value.to_sexpr(),
            ]),
        }
    }
}
