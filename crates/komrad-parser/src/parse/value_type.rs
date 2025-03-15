use crate::parse::identifier::parse_identifier;
use crate::span::{KResult, Span};
use komrad_ast::prelude::ValueType;
use nom::Parser;

pub fn parse_value_type(input: Span) -> KResult<ValueType> {
    let (remaining, typ) = parse_identifier.parse(input)?;
    let value_type = match typ.as_str() {
        "Empty" => ValueType::Empty,
        "Error" => ValueType::Error,
        "Word" => ValueType::Word,
        "List" => ValueType::List,
        "Channel" => ValueType::Channel,
        "Boolean" => ValueType::Boolean,
        "String" => ValueType::String,
        "Number" => ValueType::Number,
        "Bytes" => ValueType::Bytes,
        "Block" => ValueType::Block,
        _ => ValueType::User(typ.to_string()),
    };
    Ok((remaining, value_type))
}
