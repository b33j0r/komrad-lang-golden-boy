use crate::span::{KResult, Span};
use komrad_ast::prelude::{ParserError, Value};
use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_till1};
use nom::combinator::map_res;
use nom::multi::fold_many0;

/// Parses a single, double, or triple quoted string.
pub fn parse_string(input: Span) -> KResult<Value> {
    alt((
        parse_triple_quoted_string,
        parse_double_quoted_string,
        parse_single_quoted_string,
    ))
    .map(Value::String)
    .parse(input)
}

/// Parses a single-quoted string.
pub fn parse_single_quoted_string(input: Span) -> KResult<String> {
    let (input, _) = tag("'").parse(input)?;
    let (input, string) = parse_string_inner('\'')(input)?;
    let (input, _) = tag("'").parse(input)?;
    Ok((input, string))
}

/// Parses a double-quoted string.
pub fn parse_double_quoted_string(input: Span) -> KResult<String> {
    let (input, _) = tag("\"").parse(input)?;
    let (input, string) = parse_string_inner('\"')(input)?;
    let (input, _) = tag("\"").parse(input)?;
    Ok((input, string))
}

/// Parses a triple-quoted string.
pub fn parse_triple_quoted_string(input: Span) -> KResult<String> {
    let (input, _) = tag("\"\"\"").parse(input)?;
    let (input, string) = parse_triple_string_inner(input)?;
    let (input, _) = tag("\"\"\"").parse(input)?;
    Ok((input, string))
}

/// Optimized function to parse a string until a delimiter.
pub fn parse_string_inner(delimiter: char) -> impl Fn(Span) -> KResult<String> {
    move |input: Span| {
        let (mut remaining_input, mut output) = (input, String::new());

        loop {
            let (next_input, chunk) = take_till1(|c| c == delimiter || c == '\\')(remaining_input)?;
            output.push_str(chunk.fragment()); // Bulk copy valid chars

            remaining_input = next_input;

            if remaining_input.fragment().starts_with(delimiter) {
                break; // Stop before consuming the closing delimiter
            }

            if remaining_input.fragment().starts_with('\\') {
                let (next_input, esc) = parse_escape_sequence(remaining_input)?;
                output.push_str(&esc);
                remaining_input = next_input;
            }
        }

        Ok((remaining_input, output))
    }
}

/// Optimized function to parse a triple-quoted string.
pub fn parse_triple_string_inner(input: Span) -> KResult<String> {
    fold_many0(
        alt((
            map_res(take_till1(|c| c == '\\' || c == '"'), |s: Span| {
                Ok::<_, ParserError>(s.fragment().to_string())
            }),
            parse_escape_sequence,
        )),
        String::new,
        |mut acc, item| {
            acc.push_str(&item);
            acc
        },
    )
    .parse(input)
}

/// Parses an escape sequence (e.g. `\n`, `\t`, `\\`).
pub fn parse_escape_sequence(input: Span) -> KResult<String> {
    let (input, _) = tag("\\").parse(input)?;
    let (input, esc) =
        alt((tag("n"), tag("t"), tag("r"), tag("\""), tag("'"), tag("\\"))).parse(input)?;

    let replaced = match *esc.fragment() {
        "n" => "\n",
        "t" => "\t",
        "r" => "\r",
        "\"" => "\"",
        "'" => "'",
        "\\" => "\\",
        _ => unreachable!(),
    };

    Ok((input, replaced.to_string()))
}

#[cfg(test)]
pub mod test_parse_string {
    use crate::parse::strings::{
        parse_double_quoted_string, parse_escape_sequence, parse_single_quoted_string,
        parse_triple_quoted_string,
    };
    use crate::span::Span;
    use miette::NamedSource;
    use nom::character::complete::anychar;
    use std::sync::Arc;

    pub fn full_span(input: &str) -> Span {
        Span::new_extra(
            input,
            Arc::new(NamedSource::new("<test>", input.to_string())),
        )
    }

    #[test]
    fn test_parse_single_quoted_string() {
        let input = full_span("'hello'");
        let result = parse_single_quoted_string(input);
        assert!(result.is_ok(), "result was not ok: {:?}", result);
        assert_eq!(result.unwrap().1, "hello");
    }

    #[test]
    fn test_parse_double_quoted_string() {
        let input = full_span("\"hello\"");
        let result = parse_double_quoted_string(input);
        assert!(result.is_ok(), "result was not ok: {:?}", result);
        assert_eq!(result.unwrap().1, "hello");
    }

    #[test]
    fn test_parse_triple_quoted_string() {
        let input = full_span("\"\"\"hello\"\"\"");
        let result = parse_triple_quoted_string(input);
        assert!(result.is_ok(), "result was not ok: {:?}", result);
        assert_eq!(result.unwrap().1, "hello");
    }

    #[test]
    fn test_parse_escape_sequence() {
        let input = full_span("\\n");
        let result = parse_escape_sequence(input);
        assert!(result.is_ok(), "result was not ok: {:?}", result);
        assert_eq!(result.unwrap().1, "\n");
    }

    #[test]
    fn test_parse_character() {
        let input = full_span("a");
        match anychar::<Span, ()>(input) {
            Ok((rest, ch)) => {
                assert_eq!(ch, 'a');
                assert_eq!(rest.fragment().to_string(), "");
            }
            Err(_) => panic!("Failed to parse character"),
        }
    }
}
