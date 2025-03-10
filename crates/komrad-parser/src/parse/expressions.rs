use crate::parse::{identifier, lines, statements};
use crate::span::{KResult, Span};
use komrad_ast::prelude::{Block, CallExpr, Expr, Value};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{multispace0, space0, space1};
use nom::combinator::map;
use nom::multi::{many0, separated_list1};
use nom::sequence::{delimited, preceded};
use nom::Parser;

/// Parse a value expression, e.g. `2`, `"hello"`, or `foo`. Not a call expression.
fn parse_value_expression(input: Span) -> KResult<Box<Expr>> {
    map(
        alt((
            parse_block_expression,
            parse_number_expression,
            parse_string_expression,
            map(identifier::parse_identifier, Expr::Variable),
        )),
        |expr| Box::new(expr) as Box<Expr>,
    )
    .parse(input)
}

/// Parse a block expression, e.g. `{\n statements ... \n }`.
fn parse_block_expression(input: Span) -> KResult<Expr> {
    map(
        delimited(
            delimited(space0, tag("{"), multispace0),
            many0(alt((
                statements::parse_statement,
                lines::parse_blank_line,
                lines::parse_comment,
            ))),
            preceded(multispace0, tag("}")),
        ),
        |statements| Expr::Value(Value::Block(Box::new(Block::new(statements)))),
    )
    .parse(input)
}

/// Parse the argument list parts of a call expression.
fn parse_call_part_expression(input: Span) -> KResult<Box<Expr>> {
    let (remaining, expr) = alt((
        parse_value_expression,                             // Existing handling
        map(parse_block_expression, |expr| Box::new(expr)), // Accept blocks as arguments
    ))
    .parse(input)?;

    Ok((remaining, expr))
}

/// Parse a call expression, e.g. `foo do 2 to 5`.
fn parse_call_expression(input: Span) -> KResult<Expr> {
    // get the receiver
    let (remaining, receiver) = identifier::parse_identifier.parse(input)?;
    let (remaining, _) = space0.parse(remaining)?;
    let (remaining, parts) =
        separated_list1(space1, parse_call_part_expression).parse(remaining)?;
    Ok((
        remaining,
        Expr::Call(CallExpr::new(Expr::Variable(receiver), parts)),
    ))
}

/// Parse an expression
pub(crate) fn parse_expression(input: Span) -> KResult<Expr> {
    alt((
        parse_call_expression,
        parse_number_expression,
        parse_string_expression,
        map(identifier::parse_identifier, Expr::Variable),
    ))
    .parse(input)
}

/// Minimal approach: parse a “number” and wrap it in an Expr::Value(Number).
pub fn parse_number_expression(input: Span) -> KResult<Expr> {
    use nom::character::complete::digit1;

    map(digit1, |digits: Span| {
        let txt = digits.fragment();
        let val = txt.parse::<u64>().unwrap_or_default();
        Expr::Value(Value::Number(val.into()))
    })
    .parse(input)
}

/// Parse a string expression, e.g. "hello world".
fn parse_string_expression(input: Span) -> KResult<Expr> {
    crate::parse::strings::parse_string(input)
        .map(|(remaining, expr)| (remaining, Expr::Value(expr)))
}

#[cfg(test)]
mod test_parse_expression {
    use crate::parse::expressions::parse_value_expression;
    use crate::parse::strings::test_parse_string::full_span;
    use komrad_ast::prelude::{Block, CallExpr, Expr, Number, Statement, Value};

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
            Box::new(Expr::Value(Value::Block(
                Block::new(vec![
                    Statement::Assignment("x".into(), Expr::Value(Value::Number(Number::UInt(2))))
                        .into(),
                    Statement::Expr(Expr::Call(CallExpr::new(
                        Expr::Variable("foo".into()),
                        vec![Expr::Variable("bar".into()).into()]
                    ))),
                ])
                .into()
            )))
        );
        assert_eq!(*remaining.fragment(), "");
    }

    #[test]
    fn test_parse_agent_block_expression() {
        let input = full_span(
            r#"
        agent Alice {
            [start] {}
        }
        "#
            .trim(),
        );

        let result = parse_value_expression(input);
        let (_remaining, expr) = result.unwrap().clone();

        assert_eq!(
            expr,
            Box::new(Expr::Value(Value::Block(Box::from(Block::new(vec![
                Statement::Expr(Expr::Call(CallExpr::new(
                    Expr::Variable("start".into()),
                    vec![]
                ))),
            ])))))
        );
    }
}
