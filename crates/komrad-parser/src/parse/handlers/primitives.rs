use crate::parse::identifier::parse_identifier;
use crate::span::{KResult, Span};
use komrad_ast::prelude::{TypeExpr, Value};
use nom::bytes::complete::tag;
use nom::Parser;

pub fn parse_word(input: Span) -> KResult<TypeExpr> {
    // parse an identifier
    parse_identifier
        .map(|identifier| TypeExpr::Word(identifier.to_string()))
        .parse(input)
}

pub fn parse_boolean(input: Span) -> KResult<Value> {
    tag("true")
        .or(tag("false"))
        .map(|boolean: Span| Value::Boolean(boolean.fragment() == &"true"))
        .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::strings::test_parse_string::full_span;
    use komrad_ast::prelude::{TypeExpr, Value};

    #[test]
    fn test_parse_word() {
        // Example: hello
        let input = full_span("hello");
        let (remaining, word) = parse_word(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(word, TypeExpr::Word("hello".to_string()));
    }

    #[test]
    fn test_parse_boolean() {
        // Example: true
        let input = full_span("true");
        let (remaining, boolean) = parse_boolean(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(boolean, Value::Boolean(true));
    }

    #[test]
    fn test_parse_boolean_false() {
        // Example: false
        let input = full_span("false");
        let (remaining, boolean) = parse_boolean(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(boolean, Value::Boolean(false));
    }

    #[test]
    fn test_parse_boolean_invalid() {
        // Example: invalid
        let input = full_span("invalid");
        let result = parse_boolean(input);
        assert!(
            result.is_err(),
            "Expected parse to fail for invalid boolean"
        );
    }
}
