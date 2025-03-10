use crate::error::{KResult, Span};
use crate::parse::handlers::parse_handler_statement;
use crate::parse::lines::{parse_blank_line, parse_comment};
use crate::parse::{expressions, fields, identifier};
use komrad_runtime::prelude::Statement;
use nom::branch::alt;
use nom::combinator::map;
use nom::sequence::{delimited, separated_pair};
use nom::Parser;

/// Parse a single statement: either "IDENT = expression" or just "expression".
pub fn parse_statement(input: Span) -> KResult<Statement> {
    alt((
        fields::parse_field_definition,
        parse_handler_statement,
        parse_assignment_statement,
        map(expressions::parse_expression, Statement::Expr),
        parse_blank_line,
        parse_comment,
    ))
    .parse(input)
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
