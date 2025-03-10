use crate::span::Span;
use komrad_ast::prelude::Statement;
use miette::{NamedSource, Report};
use nom::combinator::all_consuming;
use nom::sequence::{delimited, separated_pair};
use std::path::PathBuf;
use std::sync::Arc;

pub struct ModuleBuilder {
    name: String,
    source_file: Option<PathBuf>,
    statements: Vec<Statement>,
}

pub fn parse_verbose(input: &str) -> Result<ModuleBuilder, Report> {
    // Create a full source context for error reporting.
    let full_src = Arc::new(NamedSource::new("repl.kom", input.to_string()));
    let span = Span::new_extra(input, full_src);

    match all_consuming(parse_module).parse(span).finish() {
        Ok((_remaining, module)) => Ok(module),

        // If the parser yields an error, we already have a `ParseError`.
        Err(e) => Err(Report::new(e)),
    }
}

pub fn parse_module(input: Span) -> KResult<ModuleBuilder> {
    let mut builder = ModuleBuilder::new();

    let (remaining, statements) = all_consuming(many0(alt((parse_statement,)))).parse(input)?;

    Ok((remaining, builder))
}

pub fn parse_statement(input: Span) -> KResult<Statement> {
    alt((parse_assignment_statement,)).parse(input)
}

pub fn parse_assignment_statement(input: Span) -> KResult<Statement> {
    separated_pair(
        parse_identifier,
        delimited(space0, tag("="), space0),
        parse_expression,
    )
}

pub fn parse_identifier(input: Span) -> KResult<String> {
    // Implement identifier parsing logic here
    unimplemented!()
}
