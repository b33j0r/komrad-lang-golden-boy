use crate::parse::lines::{parse_blank_line, parse_comment};
use crate::parse::statements;
use crate::span::{KResult, Span};
use komrad_ast::prelude::{Block, Expr, Statement};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{line_ending, multispace0};
use nom::multi::{many1, separated_list0};
use nom::sequence::{delimited, preceded};
use nom::Parser;

/// Parse a handler block, e.g. `{ IO println "hello!" }` -> Block(statements)
pub fn parse_block(input: Span) -> KResult<Block> {
    let (remaining, block) = delimited(
        tag("{"),
        preceded(multispace0, parse_block_statements),
        preceded(multispace0, tag("}")), // allow spaces before and after '}'
    )
    .parse(input)?;

    Ok((remaining, Block::new(block)))
}

/// Parse a block expression, i.e. `{ ...statements... }`
pub fn parse_block_expression(input: Span) -> KResult<Expr> {
    parse_block
        .map(|block| Expr::Block(block.into()))
        .parse(input)
}

pub fn parse_block_statements(input: Span) -> KResult<Vec<Statement>> {
    separated_list0(
        many1(line_ending),
        alt((statements::parse_statement, parse_blank_line, parse_comment)),
    )
    .parse(input)
}
