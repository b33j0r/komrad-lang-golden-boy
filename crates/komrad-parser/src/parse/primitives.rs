use crate::parse::identifier::parse_identifier;
use crate::parse::{block, strings};
use crate::span::{KResult, Span};
use komrad_ast::prelude::{Expr, Number, Value};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{digit1, space0};
use nom::combinator::map;
use nom::multi::separated_list0;
use nom::sequence::delimited;
use nom::Parser;

pub fn parse_word(input: Span) -> KResult<Value> {
    // parse an identifier
    parse_identifier
        .map(|identifier| Value::Word(identifier.to_string()))
        .parse(input)
}

pub fn parse_boolean(input: Span) -> KResult<Value> {
    tag("true")
        .or(tag("false"))
        .map(|boolean: Span| Value::Boolean(boolean.fragment() == &"true"))
        .parse(input)
}

pub fn parse_number(input: Span) -> KResult<Number> {
    map(digit1, |digits: Span| {
        let txt = digits.fragment();
        let unsigned_value = txt.parse::<u64>().unwrap_or_default();
        Number::UInt(unsigned_value)
    })
    .parse(input)
}

pub fn parse_list_part(input: Span) -> KResult<Expr> {
    alt((
        parse_word.map(|w| Expr::Variable(w.to_string())),
        parse_boolean.map(|b| Expr::Value(b.into())),
        parse_number.map(|n| Expr::Value(n.into())),
        strings::parse_string.map(|s| Expr::Value(s.into())),
        parse_list.map(|list| Expr::List(list)),
        block::parse_block_expression,
    ))
    .parse(input)
}

pub fn parse_list(input: Span) -> KResult<Vec<Expr>> {
    delimited(tag("["), separated_list0(space0, parse_list_part), tag("]")).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::strings::test_parse_string::full_span;
    use komrad_ast::prelude::Value;

    #[test]
    fn test_parse_word() {
        // Example: hello
        let input = full_span("hello");
        let (remaining, word) = parse_word(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(word, Value::Word("hello".to_string()));
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

    #[test]
    fn test_parse_list() {
        // Example: [1, 2, 3]
        let input = full_span("[1 2 3]");
        let (remaining, list) = parse_list(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(
            list,
            vec![
                Expr::Value(Value::Number(Number::UInt(1))),
                Expr::Value(Value::Number(Number::UInt(2))),
                Expr::Value(Value::Number(Number::UInt(3)))
            ]
        );
    }

    #[test]
    fn test_parse_list_with_word() {
        // Example: [1, 2, 3]
        let input = full_span("[Io println]");
        let (remaining, list) = parse_list(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        let expected = vec![
            Expr::Variable("Io".to_string()),
            Expr::Variable("println".to_string()),
        ];
        assert_eq!(list, expected);
    }
}
