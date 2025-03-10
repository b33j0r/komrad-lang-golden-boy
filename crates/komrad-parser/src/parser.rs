use crate::module_builder::ModuleBuilder;
use crate::parse::{lines, statements};
use crate::span::{KResult, Span};
use komrad_ast::prelude::Statement;
use miette::{NamedSource, Report};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{multispace0, space0};
use nom::combinator::all_consuming;
use nom::multi::many0;
use nom::sequence::{delimited, separated_pair};
use nom::{Finish, Parser};
use std::sync::Arc;

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

    let (remaining, statements) = all_consuming(delimited(
        multispace0,
        //
        many0(alt((
            statements::parse_statement,
            lines::parse_blank_line,
            lines::parse_comment,
        ))), //
        multispace0,
    ))
    .parse(input)?;

    for statement in statements {
        builder.add_statement(statement);
    }

    Ok((remaining, builder))
}

pub fn parse_statement(input: Span) -> KResult<Statement> {
    alt((parse_assignment_statement,)).parse(input)
}

pub fn parse_assignment_statement(input: Span) -> KResult<Statement> {
    separated_pair(
        crate::parse::identifier::parse_identifier,
        delimited(space0, tag("="), space0),
        crate::parse::expressions::parse_expression,
    )
    .map(|(name, expr)| Statement::Assignment(name, expr))
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::strings::test_parse_string::full_span;
    use komrad_ast::prelude::{Expr, Number, Value};

    #[test]
    fn test_parse_assignment_statement() {
        let input = full_span("foo = 42");
        let result = parse_assignment_statement(input);
        assert!(result.is_ok(), "Failed to parse assignment statement");
        let (remaining, statement) = result.unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(
            statement,
            Statement::Assignment(
                "foo".to_string(),
                Expr::Value(Value::Number(Number::UInt(42)))
            )
        );
    }

    #[test]
    fn test_parse_module() {
        let input = full_span(
            r#"
            foo = 42
            bar = 232
            "#,
        );
        let result = parse_module(input);
        println!("{:?}", result);
        assert!(result.is_ok(), "Failed to parse module");
        let (remaining, module) = result.unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(module.statements().len(), 2);
    }
}
