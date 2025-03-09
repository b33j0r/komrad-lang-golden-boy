use crate::operators::{BinaryOp, UnaryOp};
use crate::scope::Scope;
use crate::types::{literal, Expr};
use crate::Value;
use crate::{Msg, RuntimeError};

#[async_trait::async_trait]
pub trait Reducer {
    type State;
    type Output;

    async fn reduce(&self, state: &Self::State) -> Self::Output;
}

#[async_trait::async_trait]
impl Reducer for Expr {
    type State = Scope;
    type Output = Value;

    async fn reduce(&self, state: &Self::State) -> Self::Output {
        match self {
            Expr::Value(v) => v.clone(),
            Expr::Variable(name) => {
                if let Some(value) = state.get(name).await {
                    value
                } else {
                    Value::Word(name.clone())
                }
            }
            Expr::Ask {
                callee,
                args,
                reply,
            } => {
                let callee = callee.reduce(state).await;
                let mut new_args = Vec::new();
                for arg in args {
                    let arg = arg.reduce(state).await;
                    new_args.push(arg);
                }
                let reply = reply.reduce(state).await;
                Value::Msg(Msg {
                    callee: Box::new(callee),
                    command: None,
                    message: new_args,
                    reply: Some(Box::new(reply)),
                })
            }
            Expr::Tell { callee, args } => {
                let callee = callee.reduce(state).await;
                let mut new_args = Vec::new();
                for arg in args {
                    let arg = arg.reduce(state).await;
                    new_args.push(arg);
                }
                Value::Msg(Msg {
                    callee: Box::new(callee),
                    command: None,
                    message: new_args,
                    reply: None,
                })
            }
            Expr::Command {
                callee,
                command,
                args,
            } => {
                let callee = callee.reduce(state).await;
                let mut new_args = Vec::new();
                for arg in args {
                    let arg = arg.reduce(state).await;
                    new_args.push(arg);
                }
                Value::Msg(Msg {
                    callee: Box::new(callee),
                    command: Some(command.clone()),
                    message: new_args,
                    reply: None,
                })
            }
            Expr::Unary { op, expr } => {
                let expr = expr.reduce(state).await;
                match (op, expr) {
                    (UnaryOp::Not, Value::Bool(b)) => Value::Bool(!b),
                    (UnaryOp::Neg, Value::Int(i)) => Value::Int(-i),
                    (UnaryOp::Neg, Value::Float(f)) => Value::Float(-f),
                    (UnaryOp::Neg, Value::UInt(u)) => Value::Int((u as literal::Int) * -1),
                    (UnaryOp::Inc, Value::Int(i)) => Value::Int(i + 1),
                    (UnaryOp::Inc, Value::UInt(u)) => Value::UInt(u + 1),
                    (UnaryOp::Inc, Value::Float(f)) => Value::Float(f + 1.0),
                    (UnaryOp::Dec, Value::Int(i)) => Value::Int(i - 1),
                    (UnaryOp::Dec, Value::UInt(u)) => Value::UInt(u.saturating_sub(1)),
                    (UnaryOp::Dec, Value::Float(f)) => Value::Float(f - 1.0),
                    (_, other) => other, // No-op for unsupported unary operations
                }
            }
            Expr::Binary { left, op, right } => {
                let left = left.reduce(state).await;
                let right = right.reduce(state).await;

                match (op, left, right) {
                    // Arithmetic
                    (BinaryOp::Add, Value::Int(a), Value::Int(b)) => Value::Int(a + b),
                    (BinaryOp::Add, Value::UInt(a), Value::UInt(b)) => Value::UInt(a + b),
                    (BinaryOp::Add, Value::Float(a), Value::Float(b)) => Value::Float(a + b),
                    (BinaryOp::Sub, Value::Int(a), Value::Int(b)) => Value::Int(a - b),
                    (BinaryOp::Sub, Value::UInt(a), Value::UInt(b)) => {
                        Value::UInt(a.saturating_sub(b))
                    }
                    (BinaryOp::Sub, Value::Float(a), Value::Float(b)) => Value::Float(a - b),
                    (BinaryOp::Mul, Value::Int(a), Value::Int(b)) => Value::Int(a * b),
                    (BinaryOp::Mul, Value::UInt(a), Value::UInt(b)) => Value::UInt(a * b),
                    (BinaryOp::Mul, Value::Float(a), Value::Float(b)) => Value::Float(a * b),
                    (BinaryOp::Div, Value::Int(a), Value::Int(b)) => {
                        if b == 0 {
                            Value::Error(RuntimeError::DivisionByZero(Box::new(Value::Int(a))))
                        } else {
                            Value::Int(a / b)
                        }
                    }
                    (BinaryOp::Div, Value::UInt(a), Value::UInt(b)) => {
                        if b == 0 {
                            Value::Error(RuntimeError::DivisionByZero(Box::new(Value::UInt(a))))
                        } else {
                            Value::UInt(a / b)
                        }
                    }
                    (BinaryOp::Div, Value::Float(a), Value::Float(b)) => {
                        if b == 0.0 {
                            Value::Error(RuntimeError::DivisionByZero(Box::new(Value::Float(a))))
                        } else {
                            Value::Float(a / b)
                        }
                    }
                    (BinaryOp::Mod, Value::Int(a), Value::Int(b)) => Value::Int(a % b),
                    (BinaryOp::Mod, Value::UInt(a), Value::UInt(b)) => Value::UInt(a % b),

                    // Bitwise operations
                    (BinaryOp::And, Value::Int(a), Value::Int(b)) => Value::Int(a & b),
                    (BinaryOp::Or, Value::Int(a), Value::Int(b)) => Value::Int(a | b),
                    (BinaryOp::Xor, Value::Int(a), Value::Int(b)) => Value::Int(a ^ b),
                    (BinaryOp::Shl, Value::Int(a), Value::Int(b)) => Value::Int(a << b),
                    (BinaryOp::Shr, Value::Int(a), Value::Int(b)) => Value::Int(a >> b),

                    // Logical operations
                    (BinaryOp::And, Value::Bool(a), Value::Bool(b)) => Value::Bool(a && b),
                    (BinaryOp::Or, Value::Bool(a), Value::Bool(b)) => Value::Bool(a || b),
                    (BinaryOp::Xor, Value::Bool(a), Value::Bool(b)) => Value::Bool(a ^ b),

                    // String concatenation
                    (BinaryOp::Add, Value::String(a), Value::String(b)) => {
                        Value::String(format!("{}{}", a, b))
                    }

                    // String split with division operator
                    // e.g. "you can split strings" / " " => ("you", "can", "split", "strings")
                    (BinaryOp::Div, Value::String(a), Value::String(sep)) => {
                        // split by sep
                        let split = a
                            .split(&sep)
                            .map(|s| Value::String(s.to_string()))
                            .collect();
                        Value::List(split)
                    }

                    // String subtraction
                    // e.g. "you can remove by regex" - "can" => "you remove by regex"
                    (BinaryOp::Sub, Value::String(a), Value::String(b)) => {
                        let new_string = a.replace(&b, "");
                        Value::String(new_string)
                    }

                    // Fallback
                    (_, left, right) => Value::List(vec![left, right]), // Return as a list if unsupported
                }
            }
        }
    }
}

#[cfg(test)]
pub mod test_reduce_expr_arithmetic {
    use super::*;
    use crate::scope::Scope;
    use crate::Value;

    #[tokio::test]
    async fn test_reduce_expr_arithmetic() {
        let scope = Scope::new();
        let expr = Expr::Binary {
            left: Box::new(Expr::Value(Value::Int(5))),
            op: BinaryOp::Add,
            right: Box::new(Expr::Value(Value::Int(3))),
        };
        let result = expr.reduce(&scope).await;
        assert_eq!(result, Value::Int(8));
    }

    #[tokio::test]
    async fn test_reduce_expr_arithmetic_float() {
        let scope = Scope::new();
        let expr = Expr::Binary {
            left: Box::new(Expr::Value(Value::Float(5.0))),
            op: BinaryOp::Add,
            right: Box::new(Expr::Value(Value::Float(3.0))),
        };
        let result = expr.reduce(&scope).await;
        assert_eq!(result, Value::Float(8.0));
    }

    #[tokio::test]
    async fn test_reduce_expr_arithmetic_nested() {
        let scope = Scope::new();
        let expr = Expr::Binary {
            left: Box::new(Expr::Binary {
                left: Box::new(Expr::Value(Value::Int(5))),
                op: BinaryOp::Add,
                right: Box::new(Expr::Value(Value::Int(3))),
            }),
            op: BinaryOp::Mul,
            right: Box::new(Expr::Value(Value::Int(2))),
        };
        let result = expr.reduce(&scope).await;
        assert_eq!(result, Value::Int(16));
    }
}

#[cfg(test)]
mod test_reduce_expr_string_operations {
    use super::*;
    use crate::scope::Scope;
    use crate::Value;

    #[tokio::test]
    async fn test_reduce_expr_string_concat() {
        let scope = Scope::new();
        let expr = Expr::Binary {
            left: Box::new(Expr::Value(Value::String("Hello".to_string()))),
            op: BinaryOp::Add,
            right: Box::new(Expr::Value(Value::String(" World".to_string()))),
        };
        let result = expr.reduce(&scope).await;
        assert_eq!(result, Value::String("Hello World".to_string()));
    }

    #[tokio::test]
    async fn test_reduce_expr_string_subtract() {
        let scope = Scope::new();
        let expr = Expr::Binary {
            left: Box::new(Expr::Value(Value::String("Hello World".to_string()))),
            op: BinaryOp::Sub,
            right: Box::new(Expr::Value(Value::String(" World".to_string()))),
        };
        let result = expr.reduce(&scope).await;
        assert_eq!(result, Value::String("Hello".to_string()));
    }

    #[tokio::test]
    async fn test_reduce_expr_string_split_with_div_operator() {
        let scope = Scope::new();
        let expr = Expr::Binary {
            left: Box::new(Expr::Value(Value::String("Hello World".to_string()))),
            op: BinaryOp::Div,
            right: Box::new(Expr::Value(Value::String(" ".to_string()))),
        };
        let result = expr.reduce(&scope).await;
        assert_eq!(
            result,
            Value::List(vec![
                Value::String("Hello".to_string()),
                Value::String("World".to_string())
            ])
        );
    }
}

#[cfg(test)]
mod test_reduce_expr_logical_operations {
    use super::*;
    use crate::scope::Scope;
    use crate::Value;

    #[tokio::test]
    async fn test_reduce_expr_logical_and() {
        let scope = Scope::new();
        let expr = Expr::Binary {
            left: Box::new(Expr::Value(Value::Bool(true))),
            op: BinaryOp::And,
            right: Box::new(Expr::Value(Value::Bool(false))),
        };
        let result = expr.reduce(&scope).await;
        assert_eq!(result, Value::Bool(false));
    }

    #[tokio::test]
    async fn test_reduce_expr_logical_or() {
        let scope = Scope::new();
        let expr = Expr::Binary {
            left: Box::new(Expr::Value(Value::Bool(true))),
            op: BinaryOp::Or,
            right: Box::new(Expr::Value(Value::Bool(false))),
        };
        let result = expr.reduce(&scope).await;
        assert_eq!(result, Value::Bool(true));
    }
}

#[cfg(test)]
mod test_reduce_expr_scope_substitution {
    use super::*;
    use crate::scope::Scope;
    use crate::Value;

    #[tokio::test]
    async fn test_reduce_expr_scope_substitution() {
        let mut scope = Scope::new();
        scope.set("x".to_string(), Value::Int(5)).await;

        let expr = Expr::Variable("x".to_string());
        let result = expr.reduce(&scope).await;
        assert_eq!(result, Value::Int(5));
    }

    #[tokio::test]
    async fn test_reduce_expr_scope_substitution_not_found() {
        let scope = Scope::new();
        let expr = Expr::Variable("y".to_string());
        let result = expr.reduce(&scope).await;
        assert_eq!(result, Value::Word("y".to_string()));
    }

    #[tokio::test]
    async fn test_reduce_expr_scope_substitution_nested() {
        let mut parent_scope = Scope::new();
        parent_scope.set("x".to_string(), Value::Int(5)).await;

        let mut child_scope = Scope::with_parent(parent_scope);
        child_scope.set("y".to_string(), Value::Int(10)).await;

        let expr = Expr::Variable("x".to_string());
        let result = expr.reduce(&child_scope).await;
        assert_eq!(result, Value::Int(5));
    }
}

#[cfg(test)]
mod test_reduce_expr_unary_operations {
    use super::*;
    use crate::scope::Scope;
    use crate::Value;

    #[tokio::test]
    async fn test_reduce_expr_unary_not() {
        let scope = Scope::new();
        let expr = Expr::Unary {
            op: UnaryOp::Not,
            expr: Box::new(Expr::Value(Value::Bool(true))),
        };
        let result = expr.reduce(&scope).await;
        assert_eq!(result, Value::Bool(false));
    }

    #[tokio::test]
    async fn test_reduce_expr_unary_negate() {
        let scope = Scope::new();
        let expr = Expr::Unary {
            op: UnaryOp::Neg,
            expr: Box::new(Expr::Value(Value::Int(5))),
        };
        let result = expr.reduce(&scope).await;
        assert_eq!(result, Value::Int(-5));
    }

    #[tokio::test]
    async fn test_reduce_expr_unary_inc() {
        let scope = Scope::new();
        let expr = Expr::Unary {
            op: UnaryOp::Inc,
            expr: Box::new(Expr::Value(Value::Int(5))),
        };
        let result = expr.reduce(&scope).await;
        assert_eq!(result, Value::Int(6));
    }

    #[tokio::test]
    async fn test_reduce_expr_unary_dec() {
        let scope = Scope::new();
        let expr = Expr::Unary {
            op: UnaryOp::Dec,
            expr: Box::new(Expr::Value(Value::Int(5))),
        };
        let result = expr.reduce(&scope).await;
        assert_eq!(result, Value::Int(4));
    }
}
