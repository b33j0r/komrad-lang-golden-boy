use crate::error::KResult;
use crate::parse::expressions::parse_expression;
use crate::parse::identifier::parse_identifier;
use crate::prelude::Span;
use komrad_runtime::prelude::{Statement, ValueType};
use nom::bytes::complete::tag;
use nom::character::complete::space0;
use nom::combinator::opt;
use nom::sequence::{pair, preceded};
use nom::Parser;

pub fn parse_value_type(input: Span) -> KResult<ValueType> {
    let (remaining, typ) = parse_identifier.parse(input)?;
    let value_type = match typ.as_str() {
        "()" => ValueType::Empty,
        "Err" => ValueType::Err,
        "Maybe" => ValueType::Maybe,
        "Tuple" => ValueType::Tuple,
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

pub fn parse_field_definition(input: Span) -> KResult<Statement> {
    let (remaining, field) = (
        parse_identifier,
        space0,
        preceded(pair(tag(":"), space0), parse_value_type),
        space0,
        opt(preceded(pair(tag("="), space0), parse_expression)),
    )
        .parse(input)?;

    let (name, _, typ, _, expr) = field;
    let field_definition = Statement::FieldDefinition(name.to_string(), typ, expr);
    Ok((remaining, field_definition))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::full_span;
    use komrad_runtime::prelude::Expr;

    #[test]
    fn test_parse_field_definition() {
        let input = full_span("foo: Number = 42");
        let result = parse_field_definition(input);
        assert!(result.is_ok(), "Failed to parse field definition");
        let (remaining, field) = result.unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(
            field,
            Statement::FieldDefinition(
                "foo".to_string(),
                ValueType::Number,
                Some(Expr::Value(42.into()))
            )
        );
    }
}
