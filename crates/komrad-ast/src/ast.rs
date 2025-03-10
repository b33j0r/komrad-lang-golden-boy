use crate::operators::BinaryExpr;
use crate::prelude::{BinaryOp, ValueType};
use crate::value::Value;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallExpr {
    target: Box<Expr>,
    args: Vec<Box<Expr>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    Value(Value),
    Variable(String),
    Binary(BinaryExpr),
    Call(CallExpr),
    Block(Box<Block>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Statement {
    NoOp,
    Comment(String),
    Expr(Expr),
    Assignment(String, Expr),
    Field(String, TypeExpr, Option<Expr>),
    Handler(Handler),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Block {
    statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeExpr {
    Empty,
    Type(ValueType),
    Word(String),
    Hole(String),
    TypeHole(String, ValueType),
    BlockHole(String),
    Value(Value),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pattern {
    terms: Vec<TypeExpr>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Handler {
    pattern: Pattern,
    block: Block,
}

impl BinaryExpr {
    pub fn new(left: Expr, op: BinaryOp, right: Expr) -> Self {
        BinaryExpr {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }

    pub fn left(&self) -> &Expr {
        &self.left
    }

    pub fn operator(&self) -> &BinaryOp {
        &self.op
    }

    pub fn right(&self) -> &Expr {
        &self.right
    }
}

impl CallExpr {
    pub fn target(&self) -> &Expr {
        &self.target
    }

    pub fn args(&self) -> &Vec<Box<Expr>> {
        &self.args
    }

    pub fn new(target: Expr, args: Vec<Box<Expr>>) -> Self {
        CallExpr {
            target: Box::new(target),
            args,
        }
    }
}

impl Expr {}

impl Statement {}

impl Block {
    pub fn new(statements: Vec<Statement>) -> Self {
        Block { statements }
    }

    pub fn statements(&self) -> &Vec<Statement> {
        &self.statements
    }
}

impl Pattern {
    pub fn new(terms: Vec<TypeExpr>) -> Self {
        Pattern { terms }
    }

    pub fn terms(&self) -> &Vec<TypeExpr> {
        &self.terms
    }
}

impl TypeExpr {
    pub fn new_empty() -> Self {
        TypeExpr::Empty
    }

    pub fn new_word(word: String) -> Self {
        TypeExpr::Word(word)
    }

    pub fn new_hole(hole: String) -> Self {
        TypeExpr::Hole(hole)
    }

    pub fn new_block_hole(block_hole: String) -> Self {
        TypeExpr::BlockHole(block_hole)
    }

    pub fn new_value(value: Value) -> Self {
        TypeExpr::Value(value)
    }
}

impl Handler {
    pub fn new(pattern: Pattern, block: Block) -> Self {
        Handler { pattern, block }
    }

    pub fn pattern(&self) -> &Pattern {
        &self.pattern
    }

    pub fn block(&self) -> &Block {
        &self.block
    }
}
