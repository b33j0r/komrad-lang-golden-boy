use crate::parse::identifier::parse_identifier;
use crate::parse::{block, strings};
use crate::span::{KResult, Span};
use komrad_ast::prelude::{Expr, Number, Value};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{char, digit1, one_of, space0};
use nom::combinator::{opt, recognize};
use nom::multi::{many1, separated_list0};
use nom::sequence::{delimited, pair, preceded};
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
    alt((float, signed_decimal, unsigned_decimal)).parse(input)
}

fn float(input: Span) -> KResult<Number> {
    alt((
        // Case one: .42
        recognize((
            char('.'),
            unsigned_decimal,
            opt((one_of("eE"), opt(one_of("+-")), unsigned_decimal)),
        )), // Case two: 42e42 and 42.42e42
        recognize((
            unsigned_decimal,
            opt(preceded(char('.'), unsigned_decimal)),
            one_of("eE"),
            opt(one_of("+-")),
            unsigned_decimal,
        )), // Case three: 42. and 42.42
        recognize((unsigned_decimal, char('.'), opt(unsigned_decimal))),
    ))
    .map(|s: Span| s.fragment().to_string())
    .map(|s| Number::Float(s.parse().unwrap()))
    .parse(input)
}

fn unsigned_decimal(input: Span) -> KResult<Number> {
    recognize(many1(one_of("0123456789")))
        .map(|s: Span| s.to_string())
        .map(|s| Number::Int(s.parse().unwrap()))
        .parse(input)
}

fn signed_decimal(input: Span) -> KResult<Number> {
    recognize(pair(one_of("+-"), many1(digit1)))
        .map(|s: Span| s.to_string())
        .map(|s| Number::Int(s.parse().unwrap()))
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
