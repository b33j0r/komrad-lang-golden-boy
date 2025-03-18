// statements.rs
use crate::parse::expressions::parse_expression;
use crate::parse::handlers::parse_handler_statement;
use crate::parse::lines::{parse_blank_line, parse_comment};
use crate::parse::{fields, identifier};
use crate::span::{KResult, Span};
use komrad_ast::prelude::Statement;
use nom::branch::alt;
use nom::character::complete::space0;
use nom::combinator::map;
use nom::sequence::{delimited, preceded, separated_pair};
use nom::Parser;

/// Parse a single statement: possible forms are:
/// - "IDENT: Type = expression" (field)
/// - "[pattern] { ... }" (handler)
/// - "IDENT = expression" (assignment)
/// - expression alone
/// - blank lines
/// - comments
pub fn parse_statement(input: Span) -> KResult<Statement> {
    // Optionally consume leading blank lines or partial whitespace
    let (remaining, _) = space0.parse(input)?;

    let (remaining, statement) = alt((
        fields::parse_field_definition,
        parse_assignment_statement,
        parse_handler_statement,
        parse_expander_statement,
        map(parse_expression::parse_expression, Statement::Expr),
        parse_blank_line,
        parse_comment,
    ))
    .parse(remaining)?;

    Ok((remaining, statement))
}

/// Assignment parser: "IDENT = expression"
pub fn parse_assignment_statement(input: Span) -> KResult<Statement> {
    let assignment_parser = separated_pair(
        identifier::parse_identifier,
        delimited(space0, nom::bytes::complete::tag("="), space0),
        parse_expression::parse_expression,
    );

    map(assignment_parser, |(name, expr)| {
        Statement::Assignment(name, expr)
    })
    .parse(input)
}

/// A minimal expander parser: "*IDENT"
pub fn parse_expander_statement(input: Span) -> KResult<Statement> {
    let expander_parser = preceded(
        nom::bytes::complete::tag("*"),
        parse_expression::parse_expression,
    );

    map(expander_parser, |name| Statement::Expander(name)).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::strings::test_parse_string::full_span;
    use komrad_ast::prelude::{BinaryExpr, BinaryOp, Expr, Number, Statement, Value};

    fn test_parse(input: &str, expected: Statement) {
        let (remaining, result) = parse_statement(full_span(input)).unwrap();
        assert_eq!(remaining.fragment().to_string(), "");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_assignment() {
        test_parse(
            "x = 42",
            Statement::Assignment(
                "x".to_string(),
                Expr::Value(Value::Number(Number::UInt(42))),
            ),
        );
    }

    #[test]
    fn test_parse_assignment_with_variable() {
        test_parse(
            "x = y",
            Statement::Assignment("x".to_string(), Expr::Variable("y".to_string())),
        );
    }

    #[test]
    fn test_parse_assignment_with_expression() {
        test_parse(
            "x = 4 + 2",
            Statement::Assignment(
                "x".to_string(),
                Expr::Binary(BinaryExpr::new(
                    Expr::Value(Value::Number(Number::UInt(4))),
                    BinaryOp::Add,
                    Expr::Value(Value::Number(Number::UInt(2))),
                )),
            ),
        );
    }

    #[test]
    fn test_parse_assignment_with_variable_expression() {
        test_parse(
            "x = x + 2",
            Statement::Assignment(
                "x".to_string(),
                Expr::Binary(BinaryExpr::new(
                    Expr::Variable("x".to_string()),
                    BinaryOp::Add,
                    Expr::Value(Value::Number(Number::UInt(2))),
                )),
            ),
        );
    }

    #[test]
    fn test_parse_assignment_list() {
        test_parse(
            "x = [4 2]",
            Statement::Assignment(
                "x".to_string(),
                Expr::List(vec![
                    Expr::Value(Value::Number(Number::UInt(4))),
                    Expr::Value(Value::Number(Number::UInt(2))),
                ]),
            ),
        );
    }

    #[test]
    fn test_parse_assignment_list_with_words() {
        test_parse(
            "x = [4 2 hello]",
            Statement::Assignment(
                "x".to_string(),
                Expr::List(vec![
                    Expr::Value(Value::Number(Number::UInt(4))),
                    Expr::Value(Value::Number(Number::UInt(2))),
                    Expr::Variable("hello".to_string()),
                ]),
            ),
        );
    }

    #[test]
    fn test_parse_assignment_list_with_strings() {
        test_parse(
            "x = [say 2 \"hello\"]",
            Statement::Assignment(
                "x".to_string(),
                Expr::List(vec![
                    Expr::Variable("say".to_string()),
                    Expr::Value(Value::Number(Number::UInt(2))),
                    Expr::Value(Value::String("hello".to_string())),
                ]),
            ),
        );
    }

    #[test]
    fn test_parse_expander() {
        test_parse("*x", Statement::Expander(Expr::Variable("x".to_string())));
    }
}
