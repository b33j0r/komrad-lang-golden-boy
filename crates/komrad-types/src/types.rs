use crate::channel::Channel;
use crate::operators::{BinaryOp, UnaryOp};
use crate::RuntimeError;
use std::fmt::Display;
use std::hash::Hash;

pub mod uuid7 {
    use std::fmt::Display;
    use uuid::Uuid;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct Uuid7(pub Uuid);

    impl Uuid7 {
        pub fn new() -> Self {
            Self(Uuid::now_v7())
        }
    }

    impl Display for Uuid7 {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
}

#[cfg(not(feature = "wasm"))]
pub mod literal {
    pub type Int = i64;
    pub type UInt = u64;
    pub type Float = f64;
    pub type Bytes = Vec<u8>;
}

#[cfg(feature = "wasm")]
pub mod literal {
    pub type Int = i32;
    pub type UInt = u32;
    pub type Float = f32;
    pub type Bytes = Vec<u8>;
}

pub enum ValueType {
    Empty,
    List,
    Channel,
    Error,
    Word,
    Msg,
    Bool,
    String,
    Int,
    UInt,
    Float,
    Bytes,
    Json,
}

#[derive(Debug, Clone)]
pub enum Value {
    Empty,
    List(Vec<Value>), // Collection of values
    Channel(Channel), // Communication endpoint
    Error(RuntimeError),
    Word(String), // Like an atom in scheme, but no quote
    Msg(Msg),

    // Literals
    Bool(bool),
    String(String),
    Int(literal::Int),
    UInt(literal::UInt),
    Float(literal::Float),
    Bytes(literal::Bytes),
    Json(serde_json::Value),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Empty, Value::Empty) => true,
            (Value::Channel(c1), Value::Channel(c2)) => c1 == c2,
            (Value::Error(e1), Value::Error(e2)) => e1 == e2,
            (Value::Word(w1), Value::Word(w2)) => w1 == w2,
            (Value::List(l1), Value::List(l2)) => l1 == l2,
            (Value::Msg(m1), Value::Msg(m2)) => m1 == m2,
            (Value::Bool(b1), Value::Bool(b2)) => b1 == b2,
            (Value::String(s1), Value::String(s2)) => s1 == s2,
            (Value::Int(i1), Value::Int(i2)) => i1 == i2,
            (Value::UInt(u1), Value::UInt(u2)) => u1 == u2,
            (Value::Float(f1), Value::Float(f2)) => f1 == f2,
            (Value::Bytes(b1), Value::Bytes(b2)) => b1 == b2,
            (Value::Json(j1), Value::Json(j2)) => j1 == j2,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Value::Empty => "Empty".hash(state),
            Value::Channel(c) => c.hash(state),
            Value::Error(e) => e.hash(state),
            Value::Word(w) => w.hash(state),
            Value::List(l) => l.hash(state),
            Value::Msg(m) => m.hash(state),
            Value::Bool(b) => b.hash(state),
            Value::String(s) => s.hash(state),
            Value::Int(i) => i.hash(state),
            Value::UInt(u) => u.hash(state),
            Value::Float(f) => {
                let bits = f.to_bits();
                bits.hash(state);
            }
            Value::Bytes(b) => b.hash(state),
            Value::Json(j) => j.hash(state),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Empty => write!(f, "()"),
            Value::Channel(c) => write!(f, "{}", c),
            Value::Error(e) => write!(f, "{}", e),
            Value::Word(w) => write!(f, "{}", w),
            Value::List(l) => {
                let mut s = String::new();
                for (i, v) in l.iter().enumerate() {
                    if i > 0 {
                        s.push(' ');
                    }
                    s.push_str(&v.to_string());
                }
                write!(f, "({})", s)
            }
            Value::Msg(m) => write!(f, "{}", m),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Int(i) => write!(f, "{}", i),
            Value::UInt(u) => write!(f, "{}", u),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Bytes(b) => write!(f, "{:?}", b),
            Value::Json(j) => write!(f, "{}", j),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Msg {
    pub callee: Box<Value>,
    pub command: Option<String>,
    pub message: Vec<Value>, // Using Expr ensures Msg stays evaluable
    pub reply: Option<Box<Value>>,
}

impl Display for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // (reply =) [callee (command) message]
        let mut s = String::new();
        if let Some(reply) = &self.reply {
            s.push_str(&format!("(reply {}) ", reply));
        }
        s.push_str(&format!("({} ", self.callee));
        if let Some(command) = &self.command {
            s.push_str(&format!("({}) ", command));
        }
        for (i, v) in self.message.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            s.push_str(&v.to_string());
        }
        s.push(')');
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    Value(Value),
    Variable(String),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Command {
        callee: Box<Expr>,
        command: String,
        args: Vec<Expr>,
    },
    Tell {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Ask {
        callee: Box<Expr>,
        args: Vec<Expr>,
        reply: Box<Expr>,
    },
}
