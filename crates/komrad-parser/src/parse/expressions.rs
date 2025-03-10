use crate::parse::{identifier, lines, statements};
use crate::span::{KResult, Span};
use komrad_ast::prelude::{Block, CallExpr, Expr, Value};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{digit1, multispace0, space0, space1};
use nom::combinator::map;
use nom::multi::{many0, separated_list1};
use nom::sequence::delimited;
use nom::Parser;

/// Parse an expression that is not a "call" — i.e. block, number, string, or variable
fn parse_value_expression(input: Span) -> KResult<Box<Expr>> {
    map(
        alt((
            parse_block_expression,
            parse_number_expression,
            parse_string_expression,
            map(identifier::parse_identifier, Expr::Variable),
        )),
        Box::new,
    )
    .parse(input)
}

/// Parse a block expression, i.e. `{ ...statements... }`
fn parse_block_expression(input: Span) -> KResult<Expr> {
    map(
        delimited(
            delimited(space0, tag("{"), multispace0),
            many0(alt((
                statements::parse_statement,
                lines::parse_blank_line,
                lines::parse_comment,
            ))),
            delimited(multispace0, tag("}"), space0),
        ),
        |statements| Expr::Block(Box::new(Block::new(statements))),
    )
    .parse(input)
}

/// Parse a call argument, which could be any "value expression" (block, number, string, variable).
/// You already have parse_value_expression.
fn parse_call_part_expression(input: Span) -> KResult<Box<Expr>> {
    // The simplest approach is just parse_value_expression (which includes blocks).
    parse_value_expression(input)
}

/// Parse a call expression like `foo bar { ... } baz`.
///
/// The first identifier is the target (`foo`).
/// Then we parse zero or more arguments, each preceded by *multispace1* so newlines are allowed.
fn parse_call_expression(input: Span) -> KResult<Expr> {
    let (remaining, receiver) = identifier::parse_identifier.parse(input)?;
    let (remaining, _) = space0.parse(remaining)?;
    let (remaining, parts) =
        separated_list1(space1, parse_call_part_expression).parse(remaining)?;
    Ok((
        remaining,
        Expr::Call(CallExpr::new(Expr::Variable(receiver), parts)),
    ))
}

/// Parse an expression (calls, block, number, string, variable).
pub fn parse_expression(input: Span) -> KResult<Expr> {
    alt((
        parse_call_expression,
        parse_number_expression,
        parse_string_expression,
        map(identifier::parse_identifier, Expr::Variable),
    ))
    .parse(input)
}

/// Minimal approach: parse digits as a number.
fn parse_number_expression(input: Span) -> KResult<Expr> {
    map(digit1, |digits: Span| {
        let txt = digits.fragment();
        let val = txt.parse::<u64>().unwrap_or_default();
        Expr::Value(Value::Number(val.into()))
    })
    .parse(input)
}

/// Minimal string parser (delegates to your strings module).
fn parse_string_expression(input: Span) -> KResult<Expr> {
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
                        Block::new(vec![Statement::Handler(Handler::new(
                            Pattern::new(vec![TypeExpr::Word("start".into(),)]),
                            Block::new(vec![]),
                        ))])
                        .into()
                    )
                    .into()
                ]
            )))
        )
    }
}
