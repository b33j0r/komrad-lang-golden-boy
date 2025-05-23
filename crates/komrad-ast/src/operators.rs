use crate::prelude::Expr;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComparisonOp {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    Divisible,
}

impl Display for ComparisonOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = match self {
            ComparisonOp::Eq => "==",
            ComparisonOp::Ne => "!=",
            ComparisonOp::Gt => ">",
            ComparisonOp::Ge => ">=",
            ComparisonOp::Lt => "<",
            ComparisonOp::Le => "<=",
            ComparisonOp::Divisible => "%%",
        };
        write!(f, "{}", op)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Eq,
    Ne,
    Access,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Not,
    Inc,
    Dec,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BinaryExpr {
    pub op: BinaryOp,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub expr: Box<Expr>,
}
