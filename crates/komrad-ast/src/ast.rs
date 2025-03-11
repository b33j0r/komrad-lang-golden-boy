use crate::operators::BinaryExpr;
use crate::prelude::{BinaryOp, ValueType};
use crate::value::Value;
use std::hash::Hash;
use std::sync::Arc;

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
    Handler(Arc<Handler>),
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

impl Expr {
    pub fn is_block(&self) -> bool {
        matches!(self, Expr::Block(_))
    }
    pub fn is_value(&self) -> bool {
        matches!(self, Expr::Value(_))
    }
    pub fn is_variable(&self) -> bool {
        matches!(self, Expr::Variable(_))
    }
    pub fn is_binary(&self) -> bool {
        matches!(self, Expr::Binary(_))
    }
    pub fn is_call(&self) -> bool {
        matches!(self, Expr::Call(_))
    }
    pub fn is_empty(&self) -> bool {
        matches!(self, Expr::Value(Value::Empty))
    }
}

impl Statement {
    pub fn is_no_op(&self) -> bool {
        matches!(self, Statement::NoOp)
    }
    pub fn is_comment(&self) -> bool {
        matches!(self, Statement::Comment(_))
    }
    pub fn is_expr(&self) -> bool {
        matches!(self, Statement::Expr(_))
    }
    pub fn is_assignment(&self) -> bool {
        matches!(self, Statement::Assignment(_, _))
    }
    pub fn is_field(&self) -> bool {
        matches!(self, Statement::Field(_, _, _))
    }
    pub fn is_handler(&self) -> bool {
        matches!(self, Statement::Handler(_))
    }
}

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

#[derive(Debug, Clone)]
pub struct EmbeddedBlock {
    pub tags: Vec<String>,
    pub text: String,
}

impl EmbeddedBlock {
    pub fn new(tags: Vec<String>, text: String) -> Self {
        EmbeddedBlock { tags, text }
    }

    pub fn tags(&self) -> &Vec<String> {
        &self.tags
    }

    pub fn text(&self) -> &String {
        &self.text
    }
}

impl PartialEq for EmbeddedBlock {
    fn eq(&self, other: &Self) -> bool {
        self.tags == other.tags && self.text == other.text
    }
}

impl Eq for EmbeddedBlock {}

impl Hash for EmbeddedBlock {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.tags.hash(state);
        self.text.hash(state);
    }
}
