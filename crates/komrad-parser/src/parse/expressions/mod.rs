use crate::parse::embedded_block::parse_embedded_block_value;
use crate::parse::expressions::binary_expressions::parse_binary_expression;
use crate::parse::primitives;
use crate::parse::{block, identifier};
use crate::span::{KResult, Span};
use komrad_ast::prelude::{CallExpr, Expr, Value};
use nom::branch::alt;
use nom::character::complete::space1;
use nom::combinator::map;
use nom::multi::separated_list0;
use nom::sequence::{pair, preceded};
use nom::Parser;

pub mod binary_expressions;

/// Parse an expression that is not a "call" — i.e. block, number, string, or variable
pub fn parse_value_expression(input: Span) -> KResult<Box<Expr>> {
    map(
        alt((
            parse_binary_expression,
            block::parse_block_expression,
            parse_number_expression,
            parse_string_expression,
            map(identifier::parse_identifier, Expr::Variable),
        )),
        Box::new,
    )
    .parse(input)
}

/// Parse a call expression like `foo bar { ... } baz`.
///
/// The first identifier is the target (`foo`).
/// Then we parse zero or more arguments, each preceded by *multispace1* so newlines are allowed.
pub fn parse_call_expression(input: Span) -> KResult<Expr> {
    pair(
        identifier::parse_identifier.map(|name| Expr::Variable(name)),
        preceded(space1, separated_list0(space1, parse_value_expression)),
    )
    .map(|(target, args)| {
        Expr::Call(CallExpr::new(
            target,
            args.into_iter().map(|arg| arg.into()).collect(),
        ))
    })
    .parse(input)
}

/// Parse an expression (calls, block, number, string, variable).
pub fn parse_expression(input: Span) -> KResult<Expr> {
    alt((
        parse_call_expression,
        binary_expressions::parse_binary_expression,
        parse_number_expression,
        parse_string_expression,
        map(parse_embedded_block_value, Expr::Value),
        map(identifier::parse_identifier, Expr::Variable),
    ))
    .parse(input)
}

/// Minimal approach: parse digits as a number.
pub fn parse_number_expression(input: Span) -> KResult<Expr> {
    map(primitives::parse_number, |number| {
        Expr::Value(Value::Number(number))
    }) // Wrap in Value::Number
    .parse(input)
}

/// Minimal string parser (delegates to your strings module).
pub fn parse_string_expression(input: Span) -> KResult<Expr> {
    crate::parse::strings::parse_string(input).map(|(remaining, val)| (remaining, Expr::Value(val)))
}

#[cfg(test)]
mod test_parse_expression {
    use crate::parse::expressions::parse_value_expression;
    use crate::parse::statements::parse_statement;
    use crate::parse::strings::test_parse_string::full_span;
    use komrad_ast::prelude::{
        Block, CallExpr, Expr, Handler, Number, Pattern, Statement, TypeExpr, Value,
    };

    #[test]
    fn test_parse_block_expression() {
        let input = full_span(
            r#"
        {
            x = 2
            foo bar
        }
        "#
            .trim(),
        );
        let (remaining, expr) = parse_value_expression(input).unwrap();
        assert_eq!(
            expr,
            Box::new(Expr::Block(
                Block::new(vec![
                    Statement::Assignment("x".into(), Expr::Value(Value::Number(Number::UInt(2))))
                        .into(),
                    Statement::Expr(Expr::Call(CallExpr::new(
                        Expr::Variable("foo".into()),
                        vec![Expr::Variable("bar".into()).into()]
                    ))),
                ])
                .into()
            ))
        );
        assert_eq!(*remaining.fragment(), "");
    }

    #[test]
    fn test_parse_agent_valid_but_wrong_expression() {
        let input = full_span(
            r#"
        agent Alice
        "#
            .trim(),
        );

        let result = parse_statement(input);
        let (_remaining, stmt) = result.unwrap().clone();

        assert_eq!(
            stmt,
            Statement::Expr(Expr::Call(CallExpr::new(
                Expr::Variable("agent".into()),
                vec![Expr::Variable("Alice".into()).into()]
            )))
        )
    }

    #[test]
    fn test_parse_agent_valid_but_wrong_block_expression() {
        let input = full_span(
            r#"
        agent {}
        "#
            .trim(),
        );

        let result = parse_statement(input);
        let (_remaining, stmt) = result.unwrap().clone();

        assert_eq!(
            stmt,
            Statement::Expr(Expr::Call(CallExpr::new(
                Expr::Variable("agent".into()),
                vec![Expr::Block(Block::new(vec![]).into()).into()]
            )))
        )
    }

    #[test]
    fn test_parse_agent_valid_but_wrong_two_name_expression() {
        let input = full_span(
            r#"
        agent Alice Bob
        "#
            .trim(),
        );

        let result = parse_statement(input);
        let (_remaining, stmt) = result.unwrap().clone();

        assert_eq!(
            stmt,
            Statement::Expr(Expr::Call(CallExpr::new(
                Expr::Variable("agent".into()),
                vec![
                    Expr::Variable("Alice".into()).into(),
                    Expr::Variable("Bob".into()).into()
                ]
            )))
        )
    }

    #[test]
    fn test_parse_agent_block_expression() {
        let input = full_span(
            r#"
        agent Alice {}
        "#
            .trim(),
        );

        let result = parse_statement(input);
        let (_remaining, stmt) = result.unwrap().clone();

        assert_eq!(
            stmt,
            Statement::Expr(Expr::Call(CallExpr::new(
                Expr::Variable("agent".into()),
                vec![
                    Expr::Variable("Alice".into()).into(),
                    Expr::Block(Block::new(vec![]).into()).into()
                ]
            )))
        )
    }

    #[test]
    fn test_parse_agent_block_expression_spaced() {
        let input = full_span(
            r#"
        agent Alice { }
        "#
            .trim(),
        );

        let result = parse_statement(input);
        let (_remaining, stmt) = result.unwrap().clone();

        assert_eq!(
            stmt,
            Statement::Expr(Expr::Call(CallExpr::new(
                Expr::Variable("agent".into()),
                vec![
                    Expr::Variable("Alice".into()).into(),
                    Expr::Block(Block::new(vec![]).into()).into()
                ]
            )))
        )
    }

    #[test]
    fn test_parse_agent_block_expression_newline() {
        let input = full_span(
            r#"
        agent Alice {
        }
        "#
            .trim(),
        );

        let result = parse_statement(input);
        let (_remaining, stmt) = result.unwrap().clone();

        assert_eq!(
            stmt,
            Statement::Expr(Expr::Call(CallExpr::new(
                Expr::Variable("agent".into()),
                vec![
                    Expr::Variable("Alice".into()).into(),
                    Expr::Block(Block::new(vec![]).into()).into()
                ]
            )))
        )
    }

    #[test]
    fn test_parse_agent_block_expression_assignment() {
        let input = full_span(
            r#"
        agent Alice {
            y = 55
        }
        "#
            .trim(),
        );

        let result = parse_statement(input);
        let (_remaining, stmt) = result.unwrap().clone();

        assert_eq!(
            stmt,
            Statement::Expr(Expr::Call(CallExpr::new(
                Expr::Variable("agent".into()),
                vec![
                    Expr::Variable("Alice".into()).into(),
                    Expr::Block(
                        Block::new(vec![Statement::Assignment(
                            "y".into(),
                            Expr::Value(Value::Number(Number::UInt(55)))
                        )])
                        .into()
                    )
                    .into()
                ]
            )))
        )
    }

    #[test]
    fn test_parse_agent_block_expression_with_handler() {
        let input = full_span(
            r#"
        agent Alice {
            [start] {}
        }
        "#
            .trim(),
        );

        let result = parse_statement(input);
        let (_remaining, stmt) = result.unwrap().clone();

        assert_eq!(
            stmt,
            Statement::Expr(Expr::Call(CallExpr::new(
                Expr::Variable("agent".into()),
                vec![
                    Expr::Variable("Alice".into()).into(),
                    Expr::Block(
                        Block::new(vec![Statement::Handler(
                            Handler::new(
                                Pattern::new(vec![TypeExpr::Word("start".into(),)]),
                                Block::new(vec![]),
                            )
                            .into()
                        )])
                        .into()
                    )
                    .into()
                ]
            )))
        )
    }
}
