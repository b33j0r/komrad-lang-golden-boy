use nom::bytes::complete::{take_while, take_while1};
use nom::combinator::recognize;
use nom::sequence::pair;
use nom::Parser;

/// Parse an identifier, e.g. `[a-zA-Z_][a-zA-Z0-9_]*`.
pub(crate) fn parse_identifier(input: Span) -> KResult<String> {
    let first = |c: char| c.is_alphabetic() || c == '_';
    let rest = |c: char| c.is_alphanumeric() || c == '_' || c == '-';

    let (remaining, matched_span) = recognize(pair(
        take_while1(first), // Must start with an alphabetic character or `_`
        take_while(rest),   // Then allow numbers, letters, `_`
    ))
    .parse(input)?;

    // valid identifiers can contain `-` in the middle, but not at the start or end
    let identifier = matched_span.fragment().trim().to_string();

    if identifier.starts_with('-') || identifier.ends_with('-') {
        return Err(nom::Err::Error(ParseError::new(
            ErrorKind::InvalidAtom,
            matched_span,
        )));
    }
    Ok((remaining, identifier))
}

#[cfg(test)]
mod test_parse_identifier {
    use crate::error::Span;
    use crate::parse::identifier::parse_identifier;
    use miette::NamedSource;
    use std::sync::Arc;

    #[test]
    fn test_parse_identifier() {
        let input = Span::new_extra(
            "foo_bar",
            Arc::new(NamedSource::new("<test>", "foo_bar".to_string())),
        );
        let (remaining, identifier) = parse_identifier(input).unwrap();
        assert_eq!(identifier, "foo_bar");
        assert_eq!(remaining.fragment().to_string(), "");
    }

    #[test]
    fn test_parse_invalid_identifier() {
        let input = Span::new_extra(
            "-foo_bar",
            Arc::new(NamedSource::new("<test>", "-foo_bar".to_string())),
        );
        let _ = parse_identifier(input).unwrap_err();
    }

    #[test]
    fn test_parse_identifier_with_dash() {
        let input = Span::new_extra(
            "foo-bar",
            Arc::new(NamedSource::new("<test>", "foo-bar".to_string())),
        );
        let (remaining, identifier) = parse_identifier(input).unwrap();
        assert_eq!(identifier, "foo-bar");
        assert_eq!(remaining.fragment().to_string(), "");
    }
}
