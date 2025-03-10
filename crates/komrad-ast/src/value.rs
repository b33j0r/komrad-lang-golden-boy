use crate::ast::Block;
use crate::channel::Channel;
use crate::error::RuntimeError;
use crate::number::Number;
use crate::prelude::literal;
use std::fmt::Display;
use std::hash::Hash;

#[derive(Debug, Clone)]
pub enum Value {
    Empty,
    Error(RuntimeError),
    Channel(Channel),
    Boolean(bool),
    Word(String),
    String(String),
    Number(Number),
    List(Vec<Value>),
    Block(Box<Block>),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValueType {
    User(String),
    Empty,
    Error,
    Channel,
    Boolean,
    Word,
    String,
    Number,
    List,
    Block,
    Bytes,
}

impl Default for Value {
    fn default() -> Self {
        Value::Empty
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::String(value.to_string())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Boolean(value)
    }
}

impl From<Channel> for Value {
    fn from(value: Channel) -> Self {
        Value::Channel(value)
    }
}

impl From<Number> for Value {
    fn from(value: Number) -> Self {
        Value::Number(value)
    }
}

impl From<Vec<Value>> for Value {
    fn from(value: Vec<Value>) -> Self {
        Value::List(value)
    }
}

impl From<literal::Int> for Value {
    fn from(value: literal::Int) -> Self {
        Value::Number(Number::Int(value))
    }
}

impl From<literal::UInt> for Value {
    fn from(value: literal::UInt) -> Self {
        Value::Number(Number::UInt(value))
    }
}

impl From<literal::Float> for Value {
    fn from(value: literal::Float) -> Self {
        Value::Number(Number::Float(value))
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Value::Number(Number::UInt(literal::UInt::from(value)))
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Number(Number::Int(literal::Int::from(value)))
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value::Number(Number::Float(literal::Float::from(value)))
    }
}

impl Value {
    pub fn is_empty(&self) -> bool {
        matches!(self, Value::Empty)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Value::Error(_))
    }

    pub fn is_channel(&self) -> bool {
        matches!(self, Value::Channel(_))
    }

    pub fn is_boolean(&self) -> bool {
        matches!(self, Value::Boolean(_))
    }

    pub fn is_word(&self) -> bool {
        matches!(self, Value::Word(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    pub fn get_type(&self) -> ValueType {
        match self {
            Value::Empty => ValueType::Empty,
            Value::Error(_) => ValueType::Error,
            Value::Channel(_) => ValueType::Channel,
            Value::Boolean(_) => ValueType::Boolean,
            Value::Word(_) => ValueType::Word,
            Value::String(_) => ValueType::String,
            Value::Number(_) => ValueType::Number,
            Value::List(_) => ValueType::List,
            Value::Block(_) => ValueType::Block,
            Value::Bytes(_) => ValueType::Bytes,
        }
    }

    pub fn is_type(&self, typ: &ValueType) -> bool {
        match (self, typ) {
            (Value::Empty, ValueType::Empty) => true,
            (Value::Error(_), ValueType::Error) => true,
            (Value::Channel(_), ValueType::Channel) => true,
            (Value::Boolean(_), ValueType::Boolean) => true,
            (Value::Word(_), ValueType::Word) => true,
            (Value::String(_), ValueType::String) => true,
            (Value::Number(_), ValueType::Number) => true,
            (Value::List(_), ValueType::List) => true,
            (Value::Block(_), ValueType::Block) => true,
            (Value::Bytes(_), ValueType::Bytes) => true,
            _ => false,
        }
    }
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
            (Value::List(l1), Value::List(l2)) => l1 == l2,
            (Value::Block(b1), Value::Block(b2)) => b1 == b2,
            (Value::Bytes(b1), Value::Bytes(b2)) => b1 == b2,
            _ => false,
        }
    }
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
            Value::Block(b) => write!(f, "Block: {:?}", b),
            Value::Bytes(b) => write!(f, "Bytes: {:?}", b),
        }
    }
}

impl Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Value::Empty => state.write_u8(0),
            Value::Error(e) => e.hash(state),
            Value::Channel(c) => c.uuid().hash(state),
            Value::Boolean(b) => b.hash(state),
            Value::Word(w) => w.hash(state),
            Value::String(s) => s.hash(state),
            Value::Number(n) => n.hash(state),
            Value::List(l) => l.hash(state),
            Value::Block(b) => b.hash(state),
            Value::Bytes(b) => b.hash(state),
        }
    }
}

impl Eq for Value {}
