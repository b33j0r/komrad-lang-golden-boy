use crate::prelude::Value;
use crate::value_type::ValueType;

pub enum Predicate {}

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
