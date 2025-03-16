// statements.rs
use crate::parse::handlers::parse_handler_statement;
use crate::parse::lines::{parse_blank_line, parse_comment};
use crate::parse::{expressions, fields, identifier};
use crate::span::{KResult, Span};
use komrad_ast::prelude::Statement;
use nom::branch::alt;
use nom::character::complete::{line_ending, space0};
use nom::combinator::map;
use nom::multi::separated_list0;
use nom::sequence::{delimited, preceded, separated_pair};
use nom::Parser;

pub fn parse_block_statements(input: Span) -> KResult<Vec<Statement>> {
    separated_list0(
        line_ending,
        alt((parse_statement, parse_blank_line, parse_comment)),
    )
    .parse(input)
}

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
        parse_expander_statement,
        fields::parse_field_definition,
        parse_handler_statement,
        parse_assignment_statement,
        map(expressions::parse_expression, Statement::Expr),
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
        delimited(
            nom::character::complete::space0,
            nom::bytes::complete::tag("="),
            nom::character::complete::space0,
        ),
        expressions::parse_expression,
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
        expressions::parse_expression,
    );

    map(expander_parser, |name| Statement::Expander(name)).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::strings::test_parse_string::full_span;
    use komrad_ast::prelude::{Expr, Number, Statement, Value};

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
    fn test_parse_expander() {
        test_parse("*x", Statement::Expander(Expr::Variable("x".to_string())));
    }
}
