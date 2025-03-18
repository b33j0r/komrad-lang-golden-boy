use crate::parse::expressions::parse_expression;
use crate::parse::identifier::parse_identifier;
use crate::parse::{block, embedded_block};
use crate::span::KResult;
use komrad_ast::prelude::{BinaryExpr, BinaryOp, Expr, Span};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::space0;
use nom::combinator::{map, opt};
use nom::sequence::delimited;
use nom::Parser;

/// Returns the precedence value of a given binary operator.
/// Higher numbers bind more tightly.
fn precedence(op: &BinaryOp) -> u8 {
    match op {
        BinaryOp::Or => 1,
        BinaryOp::And => 2,
        BinaryOp::Add | BinaryOp::Sub => 3,
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 4,
    }
}

/// Parses a binary operator from the input.
/// This now supports both arithmetic and logical operators.
fn parse_binary_operator(input: Span) -> KResult<BinaryOp> {
    alt((
        map(tag("||"), |_| BinaryOp::Or),
        map(tag("&&"), |_| BinaryOp::And),
        map(tag("+"), |_| BinaryOp::Add),
        map(tag("-"), |_| BinaryOp::Sub),
        map(tag("*"), |_| BinaryOp::Mul),
        map(tag("/"), |_| BinaryOp::Div),
        map(tag("%"), |_| BinaryOp::Mod),
    ))
    .parse(input)
}

/// Parses a primary (non-binary) expression.
/// This combines call expressions, blocks, numbers, strings, embedded values, and identifiers.
fn parse_primary(input: Span) -> KResult<Expr> {
    alt((
        //expressions::parse_call_expression,
        block::parse_block_expression,
        parse_expression::parse_number_expression,
        parse_expression::parse_string_expression,
        map(embedded_block::parse_embedded_block_value, Expr::Value),
        map(parse_identifier, Expr::Variable),
    ))
    .parse(input)
}

/// Parses binary expressions using a precedence climbing algorithm.
///
/// This function first parses a primary expression and then, as long as the next operator
/// has a precedence higher than or equal to `min_prec`, it consumes the operator and recursively
/// parses the right-hand side expression with a higher minimum precedence (to enforce left associativity).
fn parse_binary_expr_prec(input: Span, min_prec: u8) -> KResult<Expr> {
    let (mut input, mut lhs) = parse_primary(input)?;

    loop {
        let (next_input, op_opt) =
            opt(delimited(space0, parse_binary_operator, space0)).parse(input.clone())?;
        match op_opt {
            Some(op) => {
                let op_prec = precedence(&op);
                if op_prec < min_prec {
                    break;
                }
                input = next_input;
                let next_min_prec = op_prec + 1;
                let (after_rhs, rhs) = parse_binary_expr_prec(input, next_min_prec)?;
                lhs = Expr::Binary(BinaryExpr::new(lhs, op, rhs));
                input = after_rhs;
            }
            None => break,
        }
    }
    Ok((input, lhs))
}

/// Public entry point for parsing binary expressions using the precedence climber.
///
/// This simply calls the recursive function with a minimum precedence of 0.
pub fn parse_binary_expression(input: Span) -> KResult<Expr> {
    parse_binary_expr_prec(input, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::new_span;
    use komrad_ast::prelude::{BinaryExpr, BinaryOp, Expr, Number, Value};

    #[test]
    fn test_simple_addition() {
        let input = new_span("1+2");
        let (remaining, expr) = parse_binary_expression(input).expect("parse failed");
        // Ensure all input was consumed.
        assert!(remaining.fragment().is_empty());

        // Expected AST: 1 + 2
        let expected = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Value(Value::Number(Number::UInt(1)))),
            op: BinaryOp::Add,
            right: Box::new(Expr::Value(Value::Number(Number::UInt(2)))),
        });
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_variable_addition() {
        let input = new_span("count+2");
        let (remaining, expr) = parse_binary_expression(input).expect("parse failed");
        // Ensure all input was consumed.
        assert!(remaining.fragment().is_empty());

        // Expected AST: count + 2
        let expected = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Variable("count".to_string())),
            op: BinaryOp::Add,
            right: Box::new(Expr::Value(Value::Number(Number::UInt(2)))),
        });
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_precedence_addition_and_multiplication() {
        let input = new_span("1+2*3");
        let (remaining, expr) = parse_binary_expression(input).expect("parse failed");
        assert!(remaining.fragment().is_empty());

        // Expected AST: 1 + (2 * 3)
        let expected = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Value(Value::Number(Number::UInt(1)))),
            op: BinaryOp::Add,
            right: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Value(Value::Number(Number::UInt(2)))),
                op: BinaryOp::Mul,
                right: Box::new(Expr::Value(Value::Number(Number::UInt(3)))),
            })),
        });
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_left_associativity_subtraction() {
        let input = new_span("1-2-3");
        let (remaining, expr) = parse_binary_expression(input).expect("parse failed");
        assert!(remaining.fragment().is_empty());

        // Expected AST: ((1 - 2) - 3)
        let expected = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Value(Value::Number(Number::UInt(1)))),
                op: BinaryOp::Sub,
                right: Box::new(Expr::Value(Value::Number(Number::UInt(2)))),
            })),
            op: BinaryOp::Sub,
            right: Box::new(Expr::Value(Value::Number(Number::UInt(3)))),
        });
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_logical_and_with_addition() {
        // Test mixed logical and arithmetic operators:
        // Expected AST: (1+2) && (3+4)
        let input = new_span("1+2&&3+4");
        let (remaining, expr) = parse_binary_expression(input).expect("parse failed");
        assert!(remaining.fragment().is_empty());

        let expected = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Value(Value::Number(Number::UInt(1)))),
                op: BinaryOp::Add,
                right: Box::new(Expr::Value(Value::Number(Number::UInt(2)))),
            })),
            op: BinaryOp::And,
            right: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Value(Value::Number(Number::UInt(3)))),
                op: BinaryOp::Add,
                right: Box::new(Expr::Value(Value::Number(Number::UInt(4)))),
            })),
        });
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_whitespace_handling_in_between() {
        let input = new_span("1 +  2 *   3");
        let (remaining, expr) = parse_binary_expression(input).expect("parse failed");
        assert!(remaining.fragment().is_empty());

        // Expected AST: 1 + (2 * 3)
        let expected = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Value(Value::Number(Number::UInt(1)))),
            op: BinaryOp::Add,
            right: Box::new(Expr::Binary(BinaryExpr {
                left: Box::new(Expr::Value(Value::Number(Number::UInt(2)))),
                op: BinaryOp::Mul,
                right: Box::new(Expr::Value(Value::Number(Number::UInt(3)))),
            })),
        });
        assert_eq!(expr, expected);
    }
}
