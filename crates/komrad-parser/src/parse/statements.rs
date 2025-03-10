// statements.rs
use crate::parse::handlers::parse_handler_statement;
use crate::parse::lines::{parse_blank_line, parse_comment};
use crate::parse::{expressions, fields, identifier};
use crate::span::{KResult, Span};
use komrad_ast::prelude::Statement;
use nom::branch::alt;
use nom::character::complete::{multispace0, newline};
use nom::combinator::{map, opt};
use nom::sequence::{delimited, separated_pair};
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
    let (remaining, _) = multispace0.parse(input)?;

    let (remaining, statement) = alt((
        fields::parse_field_definition,
        parse_handler_statement,
        parse_assignment_statement,
        map(expressions::parse_expression, Statement::Expr),
        parse_blank_line,
        parse_comment,
    ))
    .parse(remaining)?;

    // Optionally consume a trailing newline so that statements can appear on multiple lines
    let (remaining, _) = opt(newline).parse(remaining)?;

    Ok((remaining, statement))
}

/// A minimal assignment parser: "IDENT = expression"
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
