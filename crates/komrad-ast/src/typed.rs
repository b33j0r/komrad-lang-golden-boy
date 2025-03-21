use crate::prelude::TypeExpr;
use crate::value_type::ValueType;

// pub enum ValueType {
//     User(String),
//     Dependent(String, Arc<ValueType>),
//     Empty,
//     Error,
//     Channel,
//     Boolean,
//     Word,
//     String,
//     Number,
//     List,
//     Block,
//     Bytes,
//     EmbeddedBlock,
// }

pub trait Typed {
    fn is_same_type(&self, other: &Self) -> bool;
    fn is_subtype_of(&self, other: &Self) -> bool;
}

impl Typed for ValueType {
    fn is_same_type(&self, other: &Self) -> bool {
        self == other
    }

    fn is_subtype_of(&self, other: &Self) -> bool {
        match (self, other) {
            (ValueType::Empty, _) => true,
            (ValueType::Error, ValueType::Error) => true,
            (ValueType::Channel, ValueType::Channel) => true,
            (ValueType::Boolean, ValueType::Boolean) => true,
            (ValueType::Word, ValueType::Word) => true,
            (ValueType::String, ValueType::String) => true,
            (ValueType::Number, ValueType::Number) => true,
            (ValueType::List, ValueType::List) => true,
            (ValueType::Block, ValueType::Block) => true,
            (ValueType::Bytes, ValueType::Bytes) => true,
            (ValueType::EmbeddedBlock, ValueType::EmbeddedBlock) => true,
            (ValueType::User(u1), ValueType::User(u2)) => u1 == u2,
            _ => false,
        }
    }
}

impl Typed for TypeExpr {
    fn is_same_type(&self, other: &Self) -> bool {
        match (self, other) {
            (TypeExpr::Empty, TypeExpr::Empty) => true,
            (TypeExpr::HasType(t1), TypeExpr::HasType(t2)) => t1 == t2,
            (TypeExpr::Word(w1), TypeExpr::Word(w2)) => w1 == w2,
            (TypeExpr::Hole(h1), TypeExpr::Hole(h2)) => h1 == h2,
            (TypeExpr::BlockHole(bh1), TypeExpr::BlockHole(bh2)) => bh1 == bh2,
            (TypeExpr::Value(v1), TypeExpr::Value(v2)) => v1 == v2,
            _ => false,
        }
    }

    fn is_subtype_of(&self, other: &Self) -> bool {
        match (self, other) {
            (TypeExpr::Empty, _) => true,
            (TypeExpr::HasType(t1), TypeExpr::HasType(t2)) => t1.is_subtype_of(t2),
            (TypeExpr::Word(w1), TypeExpr::Word(w2)) => w1 == w2,
            (TypeExpr::Hole(h1), TypeExpr::Hole(h2)) => h1 == h2,
            (TypeExpr::BlockHole(bh1), TypeExpr::BlockHole(bh2)) => bh1 == bh2,
            (TypeExpr::Value(v1), TypeExpr::Value(v2)) => v1 == v2,
            _ => false,
        }
    }
}
