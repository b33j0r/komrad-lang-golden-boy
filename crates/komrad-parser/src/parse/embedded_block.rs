use crate::parse::identifier::parse_identifier;
use crate::parse::strings::parse_escape_sequence;
use crate::span::KResult;
use komrad_ast::prelude::{EmbeddedBlock, ErrorKind, ParserError, Span, Value};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{anychar, space1};
use nom::multi::separated_list0;
use nom::Parser;

/// Parse a fenced block with arguments optionally on the first line
///    e.g. ```<tag1> <tag2> ...\n<text>```
pub fn parse_embedded_block(input: Span) -> KResult<EmbeddedBlock> {
    let (input, _) = tag("```").parse(input)?;
    let (input, tags) = separated_list0(alt((tag(","), space1)), parse_identifier).parse(input)?;
    let (input, _) = tag("\n").parse(input)?;
    let (input, text) = parse_embedded_block_body(input)?;
    let (input, _) = tag("```").parse(input)?;
    Ok((input, EmbeddedBlock { tags, text }))
}

/// Parse a fenced block as an expression
///    e.g. ```<tag1> <tag2> ...\n<text>```
pub fn parse_embedded_block_value(input: Span) -> KResult<Value> {
    let (input, block) = parse_embedded_block(input)?;
    Ok((input, Value::EmbeddedBlock(block)))
}

fn parse_embedded_block_body(input: Span) -> KResult<String> {
    let mut output = String::new();
    let mut remaining_input = input;

    while !remaining_input.fragment().is_empty() {
        // Stop parsing when the closing delimiter is found
        if remaining_input.fragment().starts_with("```") {
            return Ok((remaining_input, output));
        }

        // Parse escape sequences
        if remaining_input.fragment().starts_with('\\') {
            let (next_input, esc) = parse_escape_sequence(remaining_input)?;
            output.push_str(&esc);
            remaining_input = next_input;
        } else {
            // Parse a regular character
            let (next_input, ch) = anychar(remaining_input)?;
            output.push(ch);
            remaining_input = next_input;
        }
    }

    // If we reach here, the closing delimiter was not found
    Err(nom::Err::Error(ParserError::new(
        ErrorKind::UnexpectedEndOfEmbeddedBlock,
        remaining_input,
    )))
}

#[cfg(test)]
mod test_parse_embedded_block {
    use crate::parse::embedded_block::{parse_embedded_block, parse_embedded_block_value};
    use crate::parse::strings::test_parse_string::full_span;
    use komrad_ast::prelude::{EmbeddedBlock, Value};

    #[test]
    fn test_parse_embedded_block() {
        let input = full_span("```\nfoo bar\n```");
        let result = parse_embedded_block(input);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().1,
            EmbeddedBlock {
                tags: vec![],
                text: "foo bar\n".into()
            }
        );
    }

    #[test]
    fn test_parse_embedded_block_with_tags() {
        let input = full_span("```foo bar\nfoo bar\n```");
        let result = parse_embedded_block(input);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().1,
            EmbeddedBlock {
                tags: vec!["foo".into(), "bar".into()],
                text: "foo bar\n".into()
            }
        );
    }

    #[test]
    fn test_parse_embedded_block_value() {
        let input = full_span("```\nfoo bar\n```");
        let result = parse_embedded_block_value(input);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().1,
            Value::EmbeddedBlock(EmbeddedBlock {
                tags: vec![],
                text: "foo bar\n".into()
            })
        );
    }
}
