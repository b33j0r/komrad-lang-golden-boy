use crate::parse::expressions::parse_expression;
use crate::parse::identifier;
use crate::span::{KResult, Span};
use komrad_ast::prelude::{CallExpr, Expr};
use nom::character::complete::space1;
use nom::multi::separated_list1;
use nom::sequence::{pair, preceded};
use nom::Parser;

/// Parse a call expression like `foo bar { ... } baz`.
///
/// The first identifier is the target (`foo`).
/// Then we parse zero or more arguments, each preceded by *multispace1* so newlines are allowed.
pub fn parse_call_expression(input: Span) -> KResult<Expr> {
    pair(
        identifier::parse_identifier.map(|name| Expr::Variable(name)),
        preceded(
            space1,
            separated_list1(space1, parse_expression::parse_value_expression),
        ),
    )
    .map(|(target, args)| {
        Expr::Call(CallExpr::new(
            target,
            args.into_iter().map(|arg| arg.into()).collect(),
        ))
    })
    .parse(input)
}
