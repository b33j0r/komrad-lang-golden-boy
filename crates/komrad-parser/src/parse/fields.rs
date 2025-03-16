use crate::parse::expressions::parse_expression;
use crate::parse::identifier::parse_identifier;
use crate::parse::value_type;
use crate::span::{KResult, Span};
use komrad_ast::prelude::{Statement, TypeExpr};
use nom::bytes::complete::tag;
use nom::character::complete::space0;
use nom::combinator::opt;
use nom::sequence::{pair, preceded};
use nom::Parser;

pub fn parse_field_definition(input: Span) -> KResult<Statement> {
    let (remaining, field) = (
        parse_identifier,
        space0,
        preceded(pair(tag(":"), space0), value_type::parse_value_type),
        space0,
        opt(preceded(pair(tag("="), space0), parse_expression)),
    )
        .parse(input)?;

    let (name, _, typ, _, expr) = field;
    let type_expr = TypeExpr::Type(typ.clone());
    let field_definition = Statement::Field(name.to_string(), type_expr, expr);
    Ok((remaining, field_definition))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::strings::test_parse_string::full_span;
    use komrad_ast::prelude::{Expr, Number, Value, ValueType};

    #[test]
    fn test_parse_field_definition() {
        let input = full_span("foo: Number = 42");
        let result = parse_field_definition(input);
        assert!(result.is_ok(), "Failed to parse field definition");
        let (remaining, field) = result.unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(
            field,
            Statement::Field(
                "foo".to_string(),
                TypeExpr::Type(ValueType::Number),
                Some(Expr::Value(Value::Number(Number::UInt(42))))
            )
        );
    }
}
