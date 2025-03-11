use std::fmt::Display;

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
    EmbeddedBlock,
}

impl Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueType::User(name) => write!(f, "{}", name),
            ValueType::Empty => write!(f, "Empty"),
            ValueType::Error => write!(f, "Error"),
            ValueType::Channel => write!(f, "Channel"),
            ValueType::Boolean => write!(f, "Boolean"),
            ValueType::Word => write!(f, "Word"),
            ValueType::String => write!(f, "String"),
            ValueType::Number => write!(f, "Number"),
            ValueType::List => write!(f, "List"),
            ValueType::Block => write!(f, "Block"),
            ValueType::Bytes => write!(f, "Bytes"),
            ValueType::EmbeddedBlock => write!(f, "EmbeddedBlock"),
        }
    }
}

impl Default for ValueType {
    fn default() -> Self {
        ValueType::Empty
    }
}
