
use std::collections::HashMap;
use std::net::IpAddr;
use tokio::sync::mpsc;
use uuid::Uuid;
use crate::RuntimeError;

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

pub mod uuid7 {
    use uuid::Uuid;

    #[derive(Debug, Clone)]
    pub struct Uuid7(pub Uuid);

    impl Uuid7 {
        pub fn new() -> Self {
            Self(Uuid::now_v7())
        }
    }
}

/// A literal value in the Komrad language.
#[derive(Debug, Clone)]
pub enum Literal {
    Null,
    Bool(bool),
    String(String),
    Int(literal::Int),
    UInt(literal::UInt),
    Float(literal::Float),
    Bytes(literal::Bytes),
    Json(serde_json::Value),
}

/// Channels are first-class citizens in the Komrad language.
#[derive(Debug, Clone)]
pub struct Channel {
    pub address: Address,
    pub sender: mpsc::Sender<Msg>,
}

/// Addresses are used to identify channels across serialization boundaries.
#[derive(Debug, Clone)]
pub enum Address {
    Named(String),
    UUID(uuid7::Uuid7),
    Ip {
        ip: IpAddr,
        port: u16,
        uuid: Uuid,
    },
}

#[derive(Debug, Clone)]
pub enum Value {
    Empty,

    Error(RuntimeError),
    Literal(Literal), // A machine-defined value
    Word(String),     // Like an atom in Lisp, but no quote
    List(Vec<Value>), // Collection of values
    Block(Block),     // Executable block of statements
    Handler(Handler), // Message handler
    Channel(Channel), // Communication endpoint
}

#[derive(Debug, Clone)]
pub enum Msg {
    Multi(Vec<Msg>), // A batch of messages
    Value(Value),    // A single value
    Unary {
        op: UnaryOp,
        value: Box<Msg>,
    },
    Binary {
        left: Box<Msg>,
        op: BinaryOp,
        right: Box<Msg>,
    },
    Tell {
        callee: Channel,
        message: Box<Msg>,
    },
    Ask {
        callee: Channel,
        args: Vec<Msg>,
        reply: Channel,
    },
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Msg(Msg),
    Assign { target: String, value: Msg },
    Decl { name: String, value: Msg },
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub parent: Option<Box<Scope>>,
    pub bindings: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub struct Handler {
    pub scope: Scope,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Xor,
    Shl,
    Shr,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Neg,
    Not,
    Inc,
    Dec,
}