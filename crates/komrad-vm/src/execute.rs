use crate::scope::Scope;
use async_trait::async_trait;
use komrad_ast::prelude::{
    BinaryExpr, BinaryOp, Block, CallExpr, Expr, Message, RuntimeError, Statement, TypeExpr, Typed,
    Value,
};
use tracing::{error, info};

pub enum ExecutionResult<T, E> {
    Skip,
    Ok(T),
    Err(E),
}

// Trait defining the execution behavior. Now it explicitly returns a pinned Future.
#[async_trait]
pub trait Execute {
    type Output;
    type Context;

    async fn execute(&self, ctx: &mut Self::Context) -> Self::Output;
}

#[async_trait]
impl Execute for Block {
    type Output = Value;
    type Context = Scope;

    // Boxing the future since this method involves recursion.
    async fn execute(&self, scope: &mut Self::Context) -> Self::Output {
        let mut last_value = Value::Empty;

        for statement in self.statements() {
            last_value = statement.execute(scope).await;
        }

        last_value
    }
}

#[async_trait]
impl Execute for Statement {
    type Output = Value;
    type Context = Scope;

    async fn execute(&self, scope: &mut Self::Context) -> Self::Output {
        match self {
            Statement::Assignment(name, expr) => {
                let value = expr.execute(scope).await;
                scope.set(name.clone(), value.clone()).await;
                value
            }
            Statement::Expr(expr) => expr.execute(scope).await,
            Statement::NoOp => Value::Empty,
            Statement::Comment(_comment_text) => Value::Empty,
            Statement::Handler(handler) => {
                scope.add_handler(handler.clone()).await;
                Value::Empty
            }
            Statement::Field(name, typ, expr) => {
                if let Some(expr) = expr {
                    let value = expr.execute(scope).await;
                    let value_type = value.get_type_expr();
                    if value_type.is_subtype_of(typ) {
                        return Value::Error(RuntimeError::TypeMismatch(format!(
                            "Expected type {:?}, found {:?}",
                            typ, value_type
                        )));
                    }
                    scope.set(name.clone(), value.clone()).await;
                    value
                } else {
                    Value::Empty
                }
            }
        }
    }
}

#[async_trait]
impl Execute for Expr {
    type Output = Value;
    type Context = Scope;

    async fn execute(&self, scope: &mut Self::Context) -> Self::Output {
        match self {
            Expr::Value(value) => value.clone(),
            Expr::Variable(name) => {
                if let Some(value) = scope.get(name).await {
                    value.clone()
                } else {
                    Value::Word(name.clone())
                }
            }
            Expr::Binary(b) => b.execute(scope).await,
            Expr::Call(call) => call.execute(scope).await,
            Expr::Block(block) => Value::Block(block.clone()),
        }
    }
}

#[async_trait]
impl Execute for CallExpr {
    type Output = Value;
    type Context = Scope;

    async fn execute(&self, scope: &mut Self::Context) -> Self::Output {
        let mut args = Vec::new();
        for arg in self.args() {
            args.push(arg.execute(scope).await);
        }
        let target = self.target().execute(scope).await;
        info!("Executing call: {:?}", target);

        if let Value::Channel(channel) = target {
            match channel.send(Message::new(args, None)).await {
                Ok(_) => {
                    // If the channel is a sender, we return an empty value.
                    // The receiver will handle the message.
                    return Value::Empty;
                }
                Err(_) => {
                    // Handle send error
                    error!("Failed to send message");
                    return Value::Error(RuntimeError::SendError);
                }
            }
        } else {
            Value::Error(RuntimeError::SendError)
        }
    }
}

#[async_trait]
impl Execute for BinaryExpr {
    type Output = Value;
    type Context = Scope;

    async fn execute(&self, scope: &mut Self::Context) -> Self::Output {
        let left = self.left().execute(scope).await;
        let right = self.right().execute(scope).await;

        match self.operator() {
            // Math
            BinaryOp::Add => match (left, right) {
                (Value::Number(l), Value::Number(r)) => Value::Number(l + r),
                (Value::String(l), Value::String(r)) => Value::String(format!("{}{}", l, r)),
                _ => Value::Empty,
            },
            BinaryOp::Sub => {
                if let (Value::Number(l), Value::Number(r)) = (left, right) {
                    Value::Number(l - r)
                } else {
                    Value::Empty
                }
            }
            BinaryOp::Mul => {
                if let (Value::Number(l), Value::Number(r)) = (left, right) {
                    Value::Number(l * r)
                } else {
                    Value::Empty
                }
            }
            BinaryOp::Div => match (left, right) {
                (Value::Number(l), Value::Number(r)) => {
                    if !r.is_zero() {
                        Value::Number(l / r)
                    } else {
                        Value::Error(RuntimeError::DivisionByZero)
                    }
                }
                _ => Value::Empty,
            },

            // Mod
            BinaryOp::Mod => match (left, right) {
                (Value::Number(l), Value::Number(r)) => {
                    if !r.is_zero() {
                        Value::Number(l % r)
                    } else {
                        Value::Error(RuntimeError::DivisionByZero)
                    }
                }
                _ => Value::Empty,
            },

            // Logical
            BinaryOp::And => {
                if let (Value::Boolean(l), Value::Boolean(r)) = (left, right) {
                    Value::Boolean(l && r)
                } else {
                    Value::Empty
                }
            }

            BinaryOp::Or => {
                if let (Value::Boolean(l), Value::Boolean(r)) = (left, right) {
                    Value::Boolean(l || r)
                } else {
                    Value::Empty
                }
            } // Bitwise
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use komrad_ast::prelude::*;

    #[tokio::test]
    async fn test_variable_is_not_empty() {
        let mut scope = Scope::default();

        // Execute the assignment to bind "x" with 42.
        let assign_stmt =
            Statement::Assignment("x".to_string(), Expr::Value(Value::Number(Number::Int(42))));
        assign_stmt.execute(&mut scope).await;

        // Now, evaluate the variable "x".
        let var_stmt = Statement::Expr(Expr::Variable("x".to_string()));
        let var_result = var_stmt.execute(&mut scope).await;
        assert_ne!(var_result, Value::Empty);
    }

    #[tokio::test]
    async fn test_block_return_value() {
        // Create a dummy scope.
        let mut scope = Scope::default();

        // Create a statement that assigns 42 to variable "x".
        let assign_stmt =
            Statement::Assignment("x".to_string(), Expr::Value(Value::Number(Number::Int(42))));
        // Then evaluate the variable "x".
        let var_stmt = Statement::Expr(Expr::Variable("x".to_string()));

        // Create a block with the two statements.
        let block = Block::new(vec![assign_stmt, var_stmt]);

        // Execute the block.
        let result = block.execute(&mut scope).await;

        // The block should return the value of the last statement.
        assert_eq!(result, Value::Number(Number::Int(42)));
    }

    #[tokio::test]
    async fn test_binary_arithmetic() {
        let mut scope = Scope::default();

        // Test addition: 3 + 4 = 7
        let add_expr = Expr::Binary(BinaryExpr::new(
            Expr::Value(Value::Number(Number::Int(3))),
            BinaryOp::Add,
            Expr::Value(Value::Number(Number::Int(4))),
        ));
        let add_result = add_expr.execute(&mut scope).await;
        assert_eq!(add_result, Value::Number(Number::Int(7)));

        // Test subtraction: 10 - 3 = 7
        let sub_expr = Expr::Binary(BinaryExpr::new(
            Expr::Value(Value::Number(Number::Int(10))),
            BinaryOp::Sub,
            Expr::Value(Value::Number(Number::Int(3))),
        ));
        let sub_result = sub_expr.execute(&mut scope).await;
        assert_eq!(sub_result, Value::Number(Number::Int(7)));

        // Test multiplication: 2 * 5 = 10
        let mul_expr = Expr::Binary(BinaryExpr::new(
            Expr::Value(Value::Number(Number::Int(2))),
            BinaryOp::Mul,
            Expr::Value(Value::Number(Number::Int(5))),
        ));
        let mul_result = mul_expr.execute(&mut scope).await;
        assert_eq!(mul_result, Value::Number(Number::Int(10)));

        // Test division: 20 / 4 = 5
        let div_expr = Expr::Binary(BinaryExpr::new(
            Expr::Value(Value::Number(Number::Int(20))),
            BinaryOp::Div,
            Expr::Value(Value::Number(Number::Int(4))),
        ));
        let div_result = div_expr.execute(&mut scope).await;
        assert_eq!(div_result, Value::Number(Number::Int(5)));

        // Test division by zero: 10 / 0 -> DivisionByZero error.
        let div_zero_expr = Expr::Binary(BinaryExpr::new(
            Expr::Value(Value::Number(Number::Int(10))),
            BinaryOp::Div,
            Expr::Value(Value::Number(Number::Int(0))),
        ));
        let div_zero_result = div_zero_expr.execute(&mut scope).await;
        assert_eq!(div_zero_result, Value::Error(RuntimeError::DivisionByZero));
    }

    #[tokio::test]
    async fn test_binary_logical() {
        let mut scope = Scope::default();

        // Test logical AND: true && false = false
        let and_expr = Expr::Binary(BinaryExpr::new(
            Expr::Value(Value::Boolean(true)),
            BinaryOp::And,
            Expr::Value(Value::Boolean(false)),
        ));
        let and_result = and_expr.execute(&mut scope).await;
        assert_eq!(and_result, Value::Boolean(false));

        // Test logical OR: true || false = true
        let or_expr = Expr::Binary(BinaryExpr::new(
            Expr::Value(Value::Boolean(true)),
            BinaryOp::Or,
            Expr::Value(Value::Boolean(false)),
        ));
        let or_result = or_expr.execute(&mut scope).await;
        assert_eq!(or_result, Value::Boolean(true));
    }

    #[tokio::test]
    async fn test_call_expression() {
        let mut scope = Scope::default();

        // Create a channel with capacity 1.
        let (channel, mut listener) = Channel::new(1);
        // Create a call expression whose target is the channel.
        // The argument is a single number value (e.g. 100).
        let call_expr = Expr::Call(CallExpr::new(
            Expr::Value(Value::Channel(channel.clone())),
            vec![Expr::Value(Value::Number(Number::Int(100))).into()],
        ));

        // Execute the call expression.
        // According to your implementation, if the target is a channel the call returns Value::Empty.
        let call_result = call_expr.execute(&mut scope).await;
        assert_eq!(call_result, Value::Empty);

        // Now, receive the message on the channel listener.
        // (In tests we have access to the private receiver field.)
        let received = listener.recv().await;
        assert!(received.is_ok(), "Expected a message to be sent");
        let message = received.unwrap();

        // Verify that the sent message’s terms match the evaluated arguments.
        assert_eq!(message.terms(), &vec![Value::Number(Number::Int(100))]);
    }

    #[tokio::test]
    async fn test_variable_not_found() {
        let mut scope = Scope::default();

        // Evaluating a variable that was never assigned should yield Value::Empty.
        let var_expr = Expr::Variable("undefined".to_string());
        let result = var_expr.execute(&mut scope).await;
        assert_eq!(result, Value::Empty);
    }
}
