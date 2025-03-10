use crate::ast::Number;
use crate::channel::Channel;
use crate::error::RuntimeError;
use crate::prelude::literal;
use std::fmt::Display;

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
        Value::Number(Number::Uint(value))
    }
}

impl From<literal::Float> for Value {
    fn from(value: literal::Float) -> Self {
        Value::Number(Number::Float(value))
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
