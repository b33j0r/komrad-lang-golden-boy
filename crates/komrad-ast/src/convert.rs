use crate::ast::{Block, Expr, Statement};
use crate::prelude::Value;

pub trait ToStatement {
    fn to_statement(&self) -> Statement;
}

pub trait ToExpr {
    fn to_expr(&self) -> Expr;
}

pub trait ToValue {
    fn to_value(&self) -> Value;
}

pub trait ToBlock {
    fn to_block(&self) -> Block;
}

pub trait ToBoxedExpr {
    fn to_boxed_expr(&self) -> Box<Expr>;
}

impl ToStatement for Value {
    fn to_statement(&self) -> Statement {
        Statement::Expr(self.to_expr())
    }
}

impl ToExpr for Value {
    fn to_expr(&self) -> Expr {
        Expr::Value(self.clone())
    }
}

impl ToBlock for Vec<Statement> {
    fn to_block(&self) -> Block {
        Block::new(self.clone())
    }
}

impl ToBlock for Statement {
    fn to_block(&self) -> Block {
        Block::new(vec![self.clone()])
    }
}

impl ToExpr for Block {
    fn to_expr(&self) -> Expr {
        Expr::Block(Box::new(self.clone()))
    }
}

impl ToValue for Block {
    fn to_value(&self) -> Value {
        Value::Block(Box::new(self.clone()))
    }
}

impl ToBoxedExpr for Block {
    fn to_boxed_expr(&self) -> Box<Expr> {
        Box::new(self.clone().to_expr())
    }
}
