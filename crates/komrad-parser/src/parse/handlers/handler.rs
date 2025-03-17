use crate::parse::handlers::holes;
use crate::parse::strings::parse_string;
use crate::parse::{block, identifier, primitives};
use crate::span::{KResult, Span};
use komrad_ast::prelude::{Handler, Number, Pattern, Statement, TypeExpr, Value};
use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{space0, space1};
use nom::multi::separated_list1;
use nom::sequence::{delimited, pair, preceded};
use std::sync::Arc;

/// Parse a handler pattern's parts, e.g. `foo do` -> `((foo) (do))`.
pub fn parse_handle_pattern_parts(input: Span) -> KResult<Vec<TypeExpr>> {
    separated_list1(
        space1,
        alt((
            holes::parse_block_hole,
            holes::parse_type_expr_hole,
            holes::parse_named_hole,
            parse_string.map(|string: Value| TypeExpr::Value(string)),
            primitives::parse_number.map(|number: Number| TypeExpr::Value(Value::Number(number))),
            primitives::parse_boolean.map(|boolean: Value| TypeExpr::Value(boolean)),
            identifier::parse_identifier.map(|identifier: String| TypeExpr::Word(identifier)),
        )),
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
    pair(
        delimited(
            tag("["),
            preceded(space0, parse_handle_pattern),
            preceded(space0, tag("]")),
        ),
        preceded(space0, block::parse_block),
    )
    .map(|(pattern, block)| Statement::Handler(Arc::new(Handler::new(pattern, block))))
    .parse(input)
}

#[cfg(test)]
mod tests {
    use crate::parse::handlers::handler::{parse_handle_pattern, parse_handler_statement};
    use crate::parse::strings::test_parse_string::full_span;
    use komrad_ast::prelude::{Pattern, Statement, TypeExpr, ValueType};

    #[test]
    fn test_parse_simple_handler_pattern() {
        // Example: [hello world]
        let input = full_span("hello world");
        let (remaining, pattern) = parse_handle_pattern(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(
            pattern,
            Pattern::new(vec![
                TypeExpr::Word("hello".to_string()),
                TypeExpr::Word("world".to_string()),
            ])
        );
    }

    #[test]
    fn test_parse_complex_handler_pattern() {
        // Example: [custom-when _(x:Int) _y do something with this block _{block}]
        let input = full_span("custom-when _(x:Number) _y do something with this block _{block}");
        let (remaining, pattern) = parse_handle_pattern(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(
            pattern,
            Pattern::new(vec![
                TypeExpr::Word("custom-when".to_string()),
                // typehole: _(x:Number) -> TypeHole("x", ValueType::Number)
                TypeExpr::TypeHole("x".to_string(), ValueType::Number),
                // named hole: _y -> Hole("y")
                TypeExpr::Hole("y".to_string()),
                TypeExpr::Word("do".to_string()),
                TypeExpr::Word("something".to_string()),
                TypeExpr::Word("with".to_string()),
                TypeExpr::Word("this".to_string()),
                TypeExpr::Word("block".to_string()),
                // block hole: _{block} -> BlockHole("block")
                TypeExpr::BlockHole("block".to_string()),
            ])
        );
    }

    #[test]
    fn test_parse_handler_pattern_with_holes() {
        // Example: [_x _y]
        let input = full_span("_x _y");
        let (remaining, pattern) = parse_handle_pattern(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(
            pattern,
            Pattern::new(vec![
                TypeExpr::Hole("x".to_string()),
                TypeExpr::Hole("y".to_string()),
            ])
        );
    }

    #[test]
    fn test_parse_full_handler_statement() {
        // Test a full handler statement including a block.
        // Example: [hello world] {}
        let input = full_span("[hello world] {}");
        let (remaining, stmt) = parse_handler_statement(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        // Expect a handler statement with pattern [hello, world] and an empty block.
        if let Statement::Handler(handler) = stmt {
            assert_eq!(
                *handler.pattern(),
                Pattern::new(vec![
                    TypeExpr::Word("hello".to_string()),
                    TypeExpr::Word("world".to_string())
                ])
            );
            // The block should have no statements.
            assert!(handler.block().statements().is_empty());
        } else {
            panic!("Expected a handler statement");
        }
    }

    #[test]
    fn test_parse_handler_pattern_with_type_hole() {
        // Example: [_x _y]
        let input = full_span("_(x:Number)");
        let (remaining, pattern) = parse_handle_pattern(input).unwrap();
        assert_eq!(*remaining.fragment(), "");
        assert_eq!(
            pattern,
            Pattern::new(vec![TypeExpr::TypeHole("x".to_string(), ValueType::Number),])
        );
    }
}
