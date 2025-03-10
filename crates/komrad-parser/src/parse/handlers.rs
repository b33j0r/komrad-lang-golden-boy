use crate::error::{KResult, Span};
use komrad_runtime::prelude::{Block, Expr, Handler, Pattern, Statement};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{space0, space1};
use nom::multi::{many0, separated_list1};
use nom::sequence::delimited;
use nom::Parser;

/// Parse a handler block, e.g. `{ IO println "hello!" }` -> Block(statements)
pub fn parse_handler_block(input: Span) -> KResult<Block> {
    let (remaining, block) = delimited(
        delimited(space0, tag("{"), space0), // allow spaces before and after '{'
        many0(alt((
            crate::parse::statements::parse_statement,
            crate::parse::lines::parse_blank_line,
            crate::parse::lines::parse_comment,
        ))),
        delimited(space0, tag("}"), space0), // allow spaces before and after '}'
    )
    .parse(input)?;

    Ok((remaining, Block::new(block)))
}

/// Parse a handler pattern's parts, e.g. `foo do` -> `((foo) (do))`.
pub fn parse_handle_pattern_parts(input: Span) -> KResult<Vec<Expr>> {
    let (remaining, parts) =
        separated_list1(space1, crate::parse::identifier::parse_identifier).parse(input)?;

    // Convert the identifiers into Expr::Variable
    let exprs = parts.into_iter().map(|name| Expr::Variable(name)).collect();

    Ok((remaining, exprs))
}

/// Parse a handler pattern, e.g. `foo do` -> `((foo) (do))`.
pub fn parse_handle_pattern(input: Span) -> KResult<Pattern> {
    let (remaining, parts) = parse_handle_pattern_parts.parse(input)?;
    Ok((remaining, Pattern::new(parts)))
}

/// Parse a handler statement, e.g. `[foo do] {\n  IO println "hello!"\n}`.
pub fn parse_handler_statement(input: Span) -> KResult<Statement> {
    let (input, _) = tag("[").parse(input)?;
    let (remaining, parts) = parse_handle_pattern.parse(input)?;
    let (input, _) = tag("]").parse(remaining)?;
    let (remaining, block) = parse_handler_block.parse(input)?;
    Ok((remaining, Statement::Handler(Handler::new(parts, block))))
}
