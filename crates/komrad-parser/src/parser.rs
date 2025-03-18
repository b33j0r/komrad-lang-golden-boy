use crate::module_builder::ModuleBuilder;
use crate::parse::block::parse_block_statements;
use crate::span::{KResult, Span};
use komrad_ast::prelude::Statement;
use miette::{NamedSource, Report};
use nom::bytes::complete::tag;
use nom::character::complete::{multispace0, space0};
use nom::combinator::all_consuming;
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

    let (remaining, statements) =
        all_consuming(delimited(multispace0, parse_block_statements, multispace0)).parse(input)?;

    for statement in statements {
        builder.add_statement(statement);
    }

    Ok((remaining, builder))
}

pub fn parse_assignment_statement(input: Span) -> KResult<Statement> {
    separated_pair(
        crate::parse::identifier::parse_identifier,
        delimited(space0, tag("="), space0),
        crate::parse::expressions::parse_expression::parse_expression,
    )
    .map(|(name, expr)| Statement::Assignment(name, expr))
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::strings::test_parse_string::full_span;
    use komrad_ast::prelude::{Block, CallExpr, Expr, Number, Pattern, TypeExpr, Value};

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

    #[test]
    fn test_parse_module_with_agent() {
        let input = full_span(
            r#"
agent Alice {
}
            "#,
        );
        let result = parse_module(input);
        println!("{:?}", result);
        assert!(result.is_ok(), "Failed to parse module");
        let (remaining, module) = result.unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(module.statements().len(), 1);
    }

    #[test]
    fn test_parse_module_with_agent_and_handler() {
        let input = full_span(
            r#"
agent Alice {
    [foo bar] {}
}
            "#,
        );
        let result = parse_module(input);
        println!("{:?}", result);
        assert!(result.is_ok(), "Failed to parse module");
        let (remaining, module) = result.unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(module.statements().len(), 1);
        // it's a CallExpr with a Handler inside a Block
        assert_eq!(
            module.statements()[0],
            Statement::Expr(Expr::Call(CallExpr::new(
                Expr::Variable("agent".into()),
                vec![
                    Expr::Variable("Alice".into()).into(),
                    Expr::Block(
                        Block::new(vec![Statement::Handler(
                            komrad_ast::prelude::Handler::new(
                                Pattern::new(vec![
                                    TypeExpr::Word("foo".to_string()),
                                    TypeExpr::Word("bar".to_string())
                                ]),
                                Block::new(vec![]),
                            )
                            .into()
                        ),])
                        .into()
                    )
                    .into(),
                ]
            )))
        );
    }

    #[test]
    fn test_parse_module_with_agent_and_handler_with_statement() {
        let input = full_span(
            r#"
agent Alice {
    [foo bar] {
        Io println "Hello, world!"
    }
}
            "#,
        );
        let result = parse_module(input);
        println!("{:?}", result);
        assert!(result.is_ok(), "Failed to parse module");
        let (remaining, module) = result.unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(module.statements().len(), 1);
        // it's a CallExpr with a Handler inside a Block
        assert_eq!(
            module.statements()[0],
            Statement::Expr(Expr::Call(CallExpr::new(
                Expr::Variable("agent".into()),
                vec![
                    Expr::Variable("Alice".into()).into(),
                    Expr::Block(
                        Block::new(vec![Statement::Handler(
                            komrad_ast::prelude::Handler::new(
                                Pattern::new(vec![
                                    TypeExpr::Word("foo".to_string()),
                                    TypeExpr::Word("bar".to_string())
                                ]),
                                Block::new(vec![Statement::Expr(Expr::Call(CallExpr::new(
                                    Expr::Variable("Io".into()),
                                    vec![
                                        Expr::Variable("println".into()).into(),
                                        Expr::Value(Value::String("Hello, world!".to_string()))
                                            .into()
                                    ]
                                ))),]),
                            )
                            .into()
                        ),])
                        .into()
                    )
                    .into(),
                ]
            )))
        );
    }
}
