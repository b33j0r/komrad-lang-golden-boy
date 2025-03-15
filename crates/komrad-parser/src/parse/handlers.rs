use crate::parse::expressions::parse_value_expression;
use crate::parse::fields::parse_value_type;
use crate::parse::identifier::parse_identifier;
use crate::parse::strings::parse_string;
use crate::span::{KResult, Span};
use komrad_ast::prelude::{Block, Expr, Handler, Pattern, Statement, TypeExpr, Value};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{space0, space1};
use nom::multi::{many0, separated_list1};
use nom::sequence::{delimited, preceded, separated_pair};
use nom::Parser;
use std::sync::Arc;
use tracing::error;

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

pub fn parse_type_constraint(input: Span) -> KResult<TypeExpr> {
    separated_pair(parse_identifier, tag(":"), parse_value_type)
        .map(|(identifier, value_type)| TypeExpr::TypeHole(identifier.to_string(), value_type))
        .parse(input)
}

pub fn parse_binary_constraint(input: Span) -> KResult<TypeExpr> {
    (
        parse_identifier,
        alt((
            tag("=="),
            tag("!="),
            tag("<"),
            tag("<="),
            tag(">"),
            tag(">="),
        )),
        parse_value_expression,
    )
        .map(|(identifier, op, value)| {
            let op = match *op {
                "==" => komrad_ast::prelude::ComparisonOp::Eq,
                "!=" => komrad_ast::prelude::ComparisonOp::Ne,
                "<" => komrad_ast::prelude::ComparisonOp::Lt,
                "<=" => komrad_ast::prelude::ComparisonOp::Le,
                ">" => komrad_ast::prelude::ComparisonOp::Gt,
                ">=" => komrad_ast::prelude::ComparisonOp::Ge,
                _ => unreachable!(),
            };
            // the expr has to be a value or a variable

            if let Expr::Variable(name) = *value {
                TypeExpr::Binary(identifier.to_string(), op, Value::Word(name.to_string()))
            } else if let Expr::Value(value) = *value {
                TypeExpr::Binary(identifier.to_string(), op, value)
            } else {
                error!("Expected a value expression, but got: {:?}", value);
                TypeExpr::Empty
            }
        })
        .parse(input)
}

pub fn parse_type_expr_hole(input: Span) -> KResult<TypeExpr> {
    // parse _(identifier:ValueType)
    preceded(
        tag("_"),
        delimited(
            tag("("),
            alt((
                parse_boolean.map(|value: Value| TypeExpr::Value(value)),
                parse_type_constraint,
                parse_binary_constraint,
            )),
            tag(")"),
        ),
    )
    .parse(input)
}

pub fn parse_word(input: Span) -> KResult<TypeExpr> {
    // parse an identifier
    parse_identifier
        .map(|identifier| TypeExpr::Word(identifier.to_string()))
        .parse(input)
}

pub fn parse_string_type_expr(input: Span) -> KResult<TypeExpr> {
    parse_string
        .map(|string| TypeExpr::Value(string.into()))
        .parse(input)
}

pub fn parse_boolean(input: Span) -> KResult<Value> {
    tag("true")
        .or(tag("false"))
        .map(|boolean: Span| Value::Boolean(boolean.fragment() == &"true"))
        .parse(input)
}

/// Parse a handler pattern's parts, e.g. `foo do` -> `((foo) (do))`.
pub fn parse_handle_pattern_parts(input: Span) -> KResult<Vec<TypeExpr>> {
    separated_list1(
        space1,
        alt((
            parse_block_hole,
            parse_type_expr_hole,
            parse_named_hole,
            parse_word,
            parse_string_type_expr,
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
    let (input, _) = tag("[").parse(input)?;
    let (remaining, parts) = parse_handle_pattern.parse(input)?;
    let (input, _) = tag("]").parse(remaining)?;
    let (remaining, block) = parse_handler_block.parse(input)?;
    Ok((
        remaining,
        Statement::Handler(Arc::new(Handler::new(parts, block))),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::strings::test_parse_string::full_span;
    use komrad_ast::prelude::{Pattern, Statement, ValueType};

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
