use crate::channel::Channel;
use crate::operators::BinaryExpr;
use crate::prelude::BinaryOp;
use crate::types::literal;
use std::fmt::Display;
use std::ops::{Add, Div, Mul, Sub};
use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum Error {
    #[error("Failed to send message")]
    SendError,
    #[error("Failed to receive message")]
    ReceiveError,
    #[error("Failed to parse message")]
    ParseError,
    #[error("Division by zero")]
    DivisionByZero,
}

#[derive(Debug, Clone)]
pub enum Value {
    Empty,
    Error(Error),
    Channel(Channel),
    Boolean(bool),
    Word(String),
    String(String),
    Number(Number),
    List(Vec<Value>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Empty, Value::Empty) => true,
            (Value::Error(e1), Value::Error(e2)) => e1 == e2,
            (Value::Channel(c1), Value::Channel(c2)) => c1 == c2,
            (Value::Boolean(b1), Value::Boolean(b2)) => b1 == b2,
            (Value::Word(w1), Value::Word(w2)) => w1 == w2,
            (Value::String(s1), Value::String(s2)) => s1 == s2,
            (Value::Number(n1), Value::Number(n2)) => n1 == n2,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Number {
    Int(literal::Int),
    Uint(literal::UInt),
    Float(literal::Float),
}

impl Number {
    pub fn is_zero(&self) -> bool {
        match self {
            Number::Int(i) => *i == 0,
            Number::Uint(u) => *u == 0,
            Number::Float(f) => *f == 0.0,
        }
    }
}

impl From<literal::Int> for Number {
    fn from(value: literal::Int) -> Self {
        Number::Int(value)
    }
}

impl From<literal::UInt> for Number {
    fn from(value: literal::UInt) -> Self {
        Number::Uint(value)
    }
}

impl From<literal::Float> for Number {
    fn from(value: literal::Float) -> Self {
        Number::Float(value)
    }
}

impl Add for Number {
    type Output = Number;

    fn add(self, other: Number) -> Self::Output {
        match (self, other) {
            (Number::Int(i1), Number::Int(i2)) => Number::Int(i1 + i2),
            (Number::Uint(u1), Number::Uint(u2)) => Number::Uint(u1 + u2),
            (Number::Float(f1), Number::Float(f2)) => Number::Float(f1 + f2),
            _ => panic!("Cannot add different number types"),
        }
    }
}

impl Sub for Number {
    type Output = Number;

    fn sub(self, other: Number) -> Self::Output {
        match (self, other) {
            (Number::Int(i1), Number::Int(i2)) => Number::Int(i1 - i2),
            (Number::Uint(u1), Number::Uint(u2)) => Number::Uint(u1 - u2),
            (Number::Float(f1), Number::Float(f2)) => Number::Float(f1 - f2),
            _ => panic!("Cannot subtract different number types"),
        }
    }
}

impl Mul for Number {
    type Output = Number;

    fn mul(self, other: Number) -> Self::Output {
        match (self, other) {
            (Number::Int(i1), Number::Int(i2)) => Number::Int(i1 * i2),
            (Number::Uint(u1), Number::Uint(u2)) => Number::Uint(u1 * u2),
            (Number::Float(f1), Number::Float(f2)) => Number::Float(f1 * f2),
            _ => panic!("Cannot multiply different number types"),
        }
    }
}

impl Div for Number {
    type Output = Number;

    fn div(self, other: Number) -> Self::Output {
        match (self, other) {
            (Number::Int(i1), Number::Int(i2)) => Number::Int(i1 / i2),
            (Number::Uint(u1), Number::Uint(u2)) => Number::Uint(u1 / u2),
            (Number::Float(f1), Number::Float(f2)) => Number::Float(f1 / f2),
            _ => panic!("Cannot divide different number types"),
        }
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Number::Int(i1), Number::Int(i2)) => i1 == i2,
            (Number::Uint(u1), Number::Uint(u2)) => u1 == u2,
            (Number::Float(f1), Number::Float(f2)) => f1 == f2,
            _ => false,
        }
    }
}

impl Eq for Number {}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::Int(i) => write!(f, "{}", i),
            Number::Uint(u) => write!(f, "{}", u),
            Number::Float(fl) => write!(f, "{}", fl),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    target: Box<Expr>,
    args: Vec<Expr>,
}

impl CallExpr {
    pub fn target(&self) -> &Expr {
        &self.target
    }

    pub fn args(&self) -> &Vec<Expr> {
        &self.args
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Value(Value),
    Variable(String),
    Binary(BinaryExpr),
    Call(CallExpr),
}

#[derive(Debug, Clone)]
pub enum Statement {
    Expression(Expr),
    Assignment(String, Expr),
}

#[derive(Debug, Clone)]
pub struct Block {
    statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Empty,
    Word(String),
    Hole(String),
    BlockHole(String),
    Value(Value),
}

#[derive(Debug, Clone)]
pub struct Pattern {
    terms: Vec<TypeExpr>,
}

#[derive(Debug, Clone)]
pub struct Handler {
    pattern: Pattern,
    block: Block,
}

fn indent_lines(s: &str, indent: usize) -> String {
    let indent_str = " ".repeat(indent);
    s.lines()
        .map(|line| format!("{}{}", indent_str, line))
        .collect::<Vec<_>>()
        .join("\n")
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Empty => write!(f, "Empty"),
            Value::Error(e) => write!(f, "Error: {}", e),
            Value::Channel(c) => write!(f, "Channel: {}", c.uuid()),
            Value::Boolean(b) => write!(f, "Boolean: {}", b),
            Value::Word(w) => write!(f, "Word: {}", w),
            Value::String(s) => write!(f, "String: {}", s),
            Value::Number(n) => write!(f, "Number: {}", n),
            Value::List(l) => write!(f, "List: {:?}", l),
        }
    }
}

impl Number {}

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
    pub fn new(target: Expr, args: Vec<Expr>) -> Self {
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
