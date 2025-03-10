use crate::parse::fields::parse_value_type;
use crate::parse::identifier::parse_identifier;
use crate::span::{KResult, Span};
use komrad_ast::prelude::{Block, Handler, Pattern, Statement, TypeExpr};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{space0, space1};
use nom::multi::{many0, separated_list1};
use nom::sequence::{delimited, preceded, separated_pair};
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

pub fn parse_named_hole(input: Span) -> KResult<TypeExpr> {
    // parse and underscore and an identifier
    preceded(tag("_"), crate::parse::identifier::parse_identifier)
        .map(|identifier| TypeExpr::Hole(identifier.to_string()))
        .parse(input)
}

pub fn parse_block_hole(input: Span) -> KResult<TypeExpr> {
    // parse _ { identifier }
    preceded(
        tag("_"),
        delimited(
            tag("{"),
            crate::parse::identifier::parse_identifier,
            tag("}"),
        ),
    )
    .map(|identifier| TypeExpr::BlockHole(identifier.to_string()))
    .parse(input)
}

pub fn parse_type_hole(input: Span) -> KResult<TypeExpr> {
    // parse _(identifier:ValueType
    preceded(
        tag("_"),
        delimited(
            tag("("),
            separated_pair(
                parse_identifier,
                preceded(space1, tag(":")),
                parse_value_type,
            ),
            tag(")"),
        ),
    )
    .map(|(identifier, value_type)| TypeExpr::TypeHole(identifier.to_string(), value_type))
    .parse(input)
}

/// Parse a handler pattern's parts, e.g. `foo do` -> `((foo) (do))`.
pub fn parse_handle_pattern_parts(input: Span) -> KResult<Vec<TypeExpr>> {
    separated_list1(
        space1,
        alt((parse_named_hole, parse_block_hole, parse_type_hole)),
    )
    .parse(input)
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
