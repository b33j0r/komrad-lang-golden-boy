use crate::parse::expressions::parse_value_expression;
use crate::parse::identifier::parse_identifier;
use crate::parse::primitives;
use crate::parse::value_type::parse_value_type;
use crate::span::{KResult, Span};
use komrad_ast::prelude::{Expr, TypeExpr, Value};
use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::space0;
use nom::sequence::{delimited, preceded, separated_pair};
use tracing::error;

pub fn parse_named_hole(input: Span) -> KResult<TypeExpr> {
    // parse and underscore and an identifier
    preceded(tag("_"), crate::parse::identifier::parse_identifier)
        .map(|identifier| TypeExpr::Hole(identifier.to_string()))
        .parse(input)
}

pub fn parse_block_hole(input: Span) -> KResult<TypeExpr> {
    // parse _ { identifier }
    preceded(
        tag("_"),
        delimited(
            tag("{"),
            preceded(space0, crate::parse::identifier::parse_identifier),
            preceded(space0, tag("}")),
        ),
    )
    .map(|identifier| TypeExpr::BlockHole(identifier.to_string()))
    .parse(input)
}

pub fn parse_type_constraint(input: Span) -> KResult<TypeExpr> {
    separated_pair(
        parse_identifier,
        preceded(space0, tag(":")),
        preceded(space0, parse_value_type),
    )
    .map(|(identifier, value_type)| TypeExpr::TypeHole(identifier.to_string(), value_type))
    .parse(input)
}

pub fn parse_binary_constraint(input: Span) -> KResult<TypeExpr> {
    (
        parse_identifier,
        preceded(
            space0,
            alt((
                tag("=="),
                tag("!="),
                tag("<="),
                tag(">="),
                tag("<"),
                tag(">"),
                tag("%%"),
            )),
        ),
        preceded(space0, parse_value_expression),
    )
        .map(|(identifier, op, value)| {
            let op = match *op {
                "==" => komrad_ast::prelude::ComparisonOp::Eq,
                "!=" => komrad_ast::prelude::ComparisonOp::Ne,
                "<=" => komrad_ast::prelude::ComparisonOp::Le,
                ">=" => komrad_ast::prelude::ComparisonOp::Ge,
                "<" => komrad_ast::prelude::ComparisonOp::Lt,
                ">" => komrad_ast::prelude::ComparisonOp::Gt,
                "%%" => komrad_ast::prelude::ComparisonOp::Divisible,
                _ => unreachable!(),
            };
            // the expr has to be a value or a variable

            if let Expr::Variable(name) = *value {
                TypeExpr::Binary(identifier.to_string(), op, Value::Word(name.to_string()))
            } else if let Expr::Value(value) = *value {
                TypeExpr::Binary(identifier.to_string(), op, value)
            } else {
                error!("Expected a value expression, but got: {:?}", value);
                TypeExpr::Empty
            }
        })
        .parse(input)
}

pub fn parse_type_expr_hole(input: Span) -> KResult<TypeExpr> {
    // parse _(identifier:ValueType)
    preceded(
        tag("_"),
        delimited(
            tag("("),
            preceded(
                space0,
                alt((
                    primitives::parse_boolean.map(|value: Value| TypeExpr::Value(value)),
                    parse_type_constraint,
                    parse_binary_constraint,
                )),
            ),
            preceded(space0, tag(")")),
        ),
    )
    .parse(input)
}

#[cfg(test)]
mod test_holes {
    use super::*;
    use crate::parse::strings::test_parse_string::full_span;
    use komrad_ast::prelude::{Number, ValueType};

    #[test]
    fn test_parse_named_hole() {
        let input = full_span("_hello");
        let (remaining, hole) = parse_named_hole(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(hole, TypeExpr::Hole("hello".to_string()));
    }

    #[test]
    fn test_parse_block_hole() {
        let input = full_span("_{hello}");
        let (remaining, hole) = parse_block_hole(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(hole, TypeExpr::BlockHole("hello".to_string()));
    }

    #[test]
    fn test_parse_block_hole_with_whitespace() {
        let input = full_span("_{ hello }");
        let (remaining, hole) = parse_block_hole(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(hole, TypeExpr::BlockHole("hello".to_string()));
    }

    #[test]
    fn test_type_expr_hole_type_constraint() {
        let input = full_span("_(hello:Number)");
        let (remaining, hole) = parse_type_expr_hole(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(
            hole,
            TypeExpr::TypeHole("hello".to_string(), ValueType::Number)
        );
    }

    #[test]
    fn test_type_expr_hole_type_constraint_with_whitespace() {
        let input = full_span("_( hello : Number )");
        let (remaining, hole) = parse_type_expr_hole(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(
            hole,
            TypeExpr::TypeHole("hello".to_string(), ValueType::Number)
        );
    }

    #[test]
    fn test_type_expr_hole_binary_constraint() {
        let input = full_span("_(hello==42)");
        let (remaining, hole) = parse_type_expr_hole(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(
            hole,
            TypeExpr::Binary(
                "hello".to_string(),
                komrad_ast::prelude::ComparisonOp::Eq,
                Value::Number(Number::UInt(42))
            )
        );
    }

    #[test]
    fn test_type_expr_hole_binary_divisible_constraint() {
        let input = full_span("_(hello %% 42)");
        let (remaining, hole) = parse_type_expr_hole(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(
            hole,
            TypeExpr::Binary(
                "hello".to_string(),
                komrad_ast::prelude::ComparisonOp::Divisible,
                Value::Number(Number::UInt(42))
            )
        );
    }

    #[test]
    fn test_type_expr_hole_binary_constraint_with_whitespace() {
        let input = full_span("_( hello == 42 )");
        let (remaining, hole) = parse_type_expr_hole(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(
            hole,
            TypeExpr::Binary(
                "hello".to_string(),
                komrad_ast::prelude::ComparisonOp::Eq,
                Value::Number(Number::UInt(42))
            )
        );
    }

    #[test]
    fn test_type_expr_hole_binary_constraint_with_variable() {
        let input = full_span("_(hello==world)");
        let (remaining, hole) = parse_type_expr_hole(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(
            hole,
            TypeExpr::Binary(
                "hello".to_string(),
                komrad_ast::prelude::ComparisonOp::Eq,
                Value::Word("world".to_string())
            )
        );
    }
}
