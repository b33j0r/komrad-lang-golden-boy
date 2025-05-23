use crate::closure::Closure;
use crate::stdlib_agent::ListAgent;
use crate::AgentBehavior;
use async_trait::async_trait;
use komrad_ast::prelude::{
    BinaryExpr, BinaryOp, Block, CallExpr, Channel, Expr, Message, RuntimeError, Statement,
    ToSexpr, Typed, Value,
};
use komrad_ast::scope::Scope;
use tracing::{debug, error, info};
// TODO
// pub enum ExecutionResult<T, E> {
//     Skip,
//     Ok(T),
//     Err(E),
// }

// Trait defining the execution behavior. Now it explicitly returns a pinned Future.
#[async_trait]
pub trait Execute {
    type Output;
    type Context;

    async fn execute(&self, ctx: &mut Self::Context) -> Self::Output;
}

#[async_trait]
pub trait ExecuteWithReply {
    type Output;
    type Context;

    async fn execute_with_reply(&self, ctx: &mut Self::Context) -> Self::Output;
}

#[async_trait]
impl Execute for Block {
    type Output = Value;
    type Context = Scope;

    // Boxing the future since this method involves recursion.
    async fn execute(&self, scope: &mut Self::Context) -> Self::Output {
        let mut last_value = Value::Empty;

        for statement in self.statements() {
            match statement {
                Statement::NoOp | Statement::Comment(_) => {
                    // Skip no-op and comment statements
                    continue;
                }
                _ => {
                    last_value = statement.execute(scope).await;
                }
            }
            if let Value::Boolean(b) = last_value {
                error!("Boolean value: {:}", b);
            }
            if let Value::Error(_) = last_value {
                error!("{:} -> {:}", statement.to_sexpr().format(0), last_value);
                // TODO: debateable whether to break. We don't have error
                //       handling constructs yet, so break for now.
                break;
            }
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
            Statement::Assignment(name, expr) => match expr {
                Expr::Call(call) => {
                    let value = call.execute_with_reply(scope).await;
                    scope.set(name.clone(), value.clone()).await;
                    return value;
                }
                _ => {
                    let value = expr.execute(scope).await;
                    scope.set(name.clone(), value.clone()).await;
                    value
                }
            },
            Statement::Expr(expr) => expr.execute(scope).await,
            Statement::NoOp => Value::Empty,
            Statement::Comment(_comment_text) => Value::Empty,
            Statement::Handler(_handler) => Value::Empty,
            Statement::Field(name, typ, expr) => {
                if let Some(expr) = expr {
                    // If a non-default value was provided for the field, use that.
                    let value = match scope.get(name) {
                        Some(value) => value.clone(),
                        None => expr.execute(scope).await,
                    };
                    let value_type = value.get_type_expr();
                    if !value_type.is_subtype_of(typ) {
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
            Statement::Expander(expr) => {
                let name = expr.execute(scope).await;
                match name {
                    Value::Word(name) => match scope.get(name.as_str()) {
                        Some(_value) => {
                            unreachable!(
                                "Expander should not be called with a bound variable: {:}",
                                name
                            );
                        }
                        None => {
                            // If the name is not found, return an error
                            Value::Error(RuntimeError::NameNotFound(name))
                        }
                    },
                    Value::Block(block) => {
                        // If an actual block is provided, execute it directly
                        block.execute(scope).await
                    }
                    Value::List(list) => {
                        // If a list is provided, treat it as a call
                        if let Some(Value::Channel(target)) = list.get(0) {
                            let mut args = Vec::new();
                            for arg in list.iter().skip(1) {
                                args.push(Expr::Value(arg.clone()).into());
                            }
                            let target = Expr::Value(Value::Channel(target.clone())).into();
                            let call = CallExpr::new(target, args);
                            call.execute_with_reply(scope).await
                        } else {
                            Value::Error(RuntimeError::TypeMismatch(format!(
                                "Expected a channel or word, found {:?}",
                                list.get(0)
                            )))
                        }
                    }
                    Value::Channel(channel) => {
                        // assume it has the List protocol, call `items` to get the value list
                        let (reply_chan, reply_chan_listener) = Channel::new(1);
                        let message = Message::new(
                            vec![Value::Word("items".to_string())],
                            Some(reply_chan.clone()),
                        );

                        let items = match channel.send(message).await {
                            Ok(_) => {
                                // Wait for the reply
                                let message = reply_chan_listener.recv().await;
                                match message {
                                    Ok(msg) => {
                                        assert_eq!(
                                            msg.terms().len(),
                                            1,
                                            "Expected a single term in reply"
                                        );
                                        msg.terms().get(0).unwrap().clone()
                                    }
                                    Err(_) => {
                                        // Handle receive error
                                        error!("Failed to receive message");
                                        Value::Error(RuntimeError::ReceiveError)
                                    }
                                }
                            }
                            Err(_) => {
                                // Handle send error
                                error!("Failed to send message");
                                Value::Error(RuntimeError::SendError)
                            }
                        };

                        // Convert the items to a call
                        if let Value::List(list) = items {
                            // If the first item is a channel, treat it as a call
                            if let Some(Value::Channel(target)) = list.get(0) {
                                let mut args = Vec::new();
                                for arg in list.iter().skip(1) {
                                    args.push(Expr::Value(arg.clone()).into());
                                }
                                let target = Expr::Value(Value::Channel(target.clone())).into();
                                let call = CallExpr::new(target, args);
                                return call.execute_with_reply(scope).await;
                            } else {
                                // If the first item is not a channel, return an error
                                Value::Error(RuntimeError::TypeMismatch(format!(
                                    "Expected a channel, found {:?}",
                                    list.get(0)
                                )))
                            }
                        } else {
                            Value::Error(RuntimeError::TypeMismatch(format!(
                                "Expected a list, found {:?}",
                                items
                            )))
                        }
                    }
                    _ => Value::Error(RuntimeError::TypeMismatch(format!(
                        "Expected a word or block, found {:?}",
                        name
                    ))),
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
            Expr::List(list) => {
                let mut new_list = Vec::new();
                for item in list {
                    new_list.push(item.execute(scope).await);
                }

                // spawn a ListAgent with the list, return a channel
                let list_agent = ListAgent::new(new_list);
                let list_channel = list_agent.spawn();
                // Return the channel as a value
                Value::Channel(list_channel)
            }
            Expr::Value(val) => val.clone(),
            Expr::Variable(name) => {
                if let Some(value) = scope.get(name) {
                    value.clone()
                } else {
                    // If not found, produce Word("x") or so
                    Value::Word(name.clone())
                }
            }

            Expr::Binary(b) => b.execute(scope).await,
            Expr::Call(call) => call.execute(scope).await,

            Expr::Block(_block) => {
                // We're using the outer expression instead of the capture

                // 1) closure transform
                let closed_expr = self.closure(scope).await;
                if let Expr::Block(new_block) = closed_expr {
                    // 2) Now actually run those statements
                    Value::Block(new_block)
                } else {
                    unreachable!("Expected a block after closure transform")
                }
            }
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
        debug!(
            "☎️ {:}",
            (target.clone(), args.clone()).to_sexpr().format(0)
        );

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
impl ExecuteWithReply for CallExpr {
    type Output = Value;
    type Context = Scope;

    async fn execute_with_reply(&self, scope: &mut Self::Context) -> Self::Output {
        let mut args = Vec::new();
        for arg in self.args() {
            args.push(arg.execute(scope).await);
        }
        let target = self.target().execute(scope).await;
        info!(
            "🔁 {:}",
            (target.clone(), args.clone()).to_sexpr().format(0)
        );

        if let Value::Channel(channel) = target {
            let (reply_chan, reply_chan_rx) = Channel::new(1);
            let message_with_reply_to = Message::new(args, Some(reply_chan.clone()));
            match channel.send(message_with_reply_to).await {
                Ok(_) => {
                    // Wait for the reply
                    let message = reply_chan_rx.recv().await;
                    match message {
                        Ok(msg) => {
                            assert_eq!(msg.terms().len(), 1, "Expected a single term in reply");
                            msg.terms().get(0).unwrap().clone()
                        }
                        Err(_) => {
                            // Handle receive error
                            error!("Failed to receive message");
                            Value::Error(RuntimeError::ReceiveError)
                        }
                    }
                }
                Err(_) => {
                    // Handle send error
                    error!("Failed to send message");
                    Value::Error(RuntimeError::SendError)
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
            BinaryOp::Add => match (left.clone(), right.clone()) {
                (Value::Number(l), Value::Number(r)) => Value::Number(l + r),
                (Value::String(l), Value::String(r)) => Value::String(format!("{}{}", l, r)),
                (Value::String(l), Value::Channel(ch)) => {
                    Value::String(format!("{}{}", l, ch.uuid().to_string()))
                }
                (Value::String(l), Value::Empty) => Value::String(format!("{}{}", l, "EMPTY")),
                (Value::String(l), Value::Number(r)) => {
                    let mut l = l.clone();
                    l = format!("{}{}", l, r);
                    Value::String(l)
                }
                (Value::Embedded(b), Value::String(r)) => {
                    let mut b = b.clone();
                    b.text = format!("{}{}", b.text(), r);
                    Value::Embedded(b)
                }
                (Value::String(l), Value::Embedded(b)) => {
                    let mut b = b.clone();
                    b.text = format!("{}{}", l, b.text());
                    Value::Embedded(b)
                }
                _ => {
                    error!("Unsupported binary operation: {:?} {:?}", left, right);
                    Value::Error(RuntimeError::TypeMismatch(format!(
                        "Unsupported binary operation: {:} {:}",
                        left, right
                    )))
                }
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
            BinaryOp::Eq => {
                if left == right {
                    Value::Boolean(true)
                } else {
                    Value::Boolean(false)
                }
            }
            BinaryOp::Ne => {
                if left != right {
                    Value::Boolean(true)
                } else {
                    Value::Boolean(false)
                }
            }
            BinaryOp::Access => {
                // Access operator e.g. `a.b` or `foo.bar`
                match (left.clone(), right.clone()) {
                    (Value::Channel(channel), Value::Word(word)) => {
                        // left is a channel, right is a word
                        match channel.get(word.as_str()).await {
                            Ok(value) => value,
                            Err(_) => Value::Error(RuntimeError::NameNotFound(word)),
                        }
                    }
                    (Value::Word(word), Value::Word(member)) => {
                        // left is a word, right is a word
                        if let Some(value) = scope.get(word.as_str()) {
                            match value {
                                Value::Channel(channel) => {
                                    channel.get(member.as_str()).await.unwrap_or(Value::Empty)
                                }
                                _ => Value::Error(RuntimeError::TypeMismatch(format!(
                                    "Expected a channel, found {:?}",
                                    value
                                ))),
                            }
                        } else {
                            Value::Error(RuntimeError::NameNotFound(word))
                        }
                    }
                    (_, _) => Value::Error(RuntimeError::TypeMismatch(format!(
                        "Expected a channel or word, found {:?} {:?}",
                        left, right
                    ))),
                }
            }
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
        let (channel, listener) = Channel::new(1);
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
        assert_eq!(result, Value::Word("undefined".to_string()));
    }

    #[tokio::test]
    async fn test_assign_list_to_variable() {
        let mut scope = Scope::default();

        // Assign a list to a variable.
        let assign_stmt = Statement::Assignment(
            "my_list".to_string(),
            Expr::List(vec![
                Expr::Value(Value::Number(Number::Int(1))).into(),
                Expr::Value(Value::Number(Number::Int(2))).into(),
                Expr::Value(Value::Number(Number::Int(3))).into(),
            ]),
        );
        assign_stmt.execute(&mut scope).await;

        // Now, evaluate the variable "my_list".
        let var_stmt = Statement::Expr(Expr::Variable("my_list".to_string()));
        let var_result = var_stmt.execute(&mut scope).await;

        // The result should be a channel to a ListAgent
        if let Value::Channel(channel) = var_result {
            // okay
        } else {
            panic!("Expected a channel to a ListAgent");
        }
    }
}
