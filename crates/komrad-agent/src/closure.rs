use async_trait::async_trait;
use komrad_ast::prelude::{BinaryExpr, Block, CallExpr, Expr, RuntimeError, Statement, Value};
use komrad_ast::scope::Scope;

#[async_trait]
pub trait Closure {
    type Input;
    type Output;
    type Context = Scope;

    /// Binds the variables in the scope within the AST node and its children.
    async fn closure(&self, context: &mut Self::Context) -> Self::Output;
}

#[async_trait]
impl Closure for Block {
    type Input = Block;
    type Output = Value;
    type Context = Scope;

    async fn closure(&self, context: &mut Self::Context) -> Self::Output {
        let mut new_statements = self.statements().clone();

        for statement in &mut new_statements {
            let _ = statement.closure(context).await;
        }

        Value::Block(Block::new(new_statements).into())
    }
}

#[async_trait]
impl Closure for Statement {
    type Input = Statement;
    type Output = Statement;
    type Context = Scope;

    async fn closure(&self, context: &mut Self::Context) -> Self::Output {
        match self {
            Statement::NoOp => Statement::NoOp,
            statement @ Statement::Comment(_) => statement.clone(),
            statement @ Statement::Handler(_) => statement.clone(),
            Statement::Expr(expr) => Statement::Expr(expr.closure(context).await),
            Statement::Assignment(name, expr) => {
                Statement::Assignment(name.clone(), expr.closure(context).await)
            }
            Statement::Field(name, typ_expr, value_expr) => {
                if let Some(value_expr) = value_expr {
                    Statement::Field(
                        name.to_string(),
                        typ_expr.clone(),
                        Some(value_expr.closure(context).await),
                    )
                } else {
                    Statement::Field(name.to_string(), typ_expr.clone(), None)
                }
            }
            Statement::Expander(expr) => Statement::Expander(expr.closure(context).await),
        }
    }
}

#[async_trait]
impl Closure for Expr {
    type Input = Expr;
    type Output = Expr;
    type Context = Scope;

    async fn closure(&self, context: &mut Self::Context) -> Self::Output {
        match self {
            Expr::List(list) => {
                let mut new_list = Vec::new();
                for item in list {
                    new_list.push(item.closure(context).await);
                }
                Expr::List(new_list)
            }
            Expr::Variable(name) => {
                if let Some(val) = context.get(name) {
                    Expr::Value(val)
                } else {
                    self.clone()
                }
            }
            Expr::Member(path) => {
                if path.len() != 2 {
                    return Expr::Value(Value::Error(RuntimeError::InvalidArugments(
                        "Member path must be of length 2".to_string(),
                    )));
                }
                let target_value: String = path[0].clone();
                let target_member = path[1].clone();

                if let Some(Value::Channel(target)) = context.get(&target_value) {
                    let value = target.get(&target_member).await;
                    if let Ok(value) = value {
                        Expr::Value(value)
                    } else {
                        Expr::Value(Value::Error(RuntimeError::NameNotFound(target_member)))
                    }
                } else {
                    Expr::Value(Value::Error(RuntimeError::NameNotFound(target_value)))
                }
            }
            Expr::Block(block) => {
                let mut new_stmts = Vec::new();
                for stmt in block.statements() {
                    new_stmts.push(stmt.closure(context).await);
                }
                Expr::Block(Box::new(Block::new(new_stmts)))
            }
            Expr::Call(call) => Expr::Call(call.closure(context).await),
            Expr::Binary(bexpr) => Expr::Binary(bexpr.closure(context).await),
            Expr::Value(_) => self.clone(),
        }
    }
}

#[async_trait]
impl Closure for BinaryExpr {
    type Input = BinaryExpr;
    type Output = BinaryExpr;
    type Context = Scope;

    async fn closure(&self, context: &mut Self::Context) -> Self::Output {
        let left = self.left().closure(context).await;
        let right = self.right().closure(context).await;
        BinaryExpr::new(left, self.op.clone(), right)
    }
}

#[async_trait]
impl Closure for CallExpr {
    type Input = CallExpr;
    type Output = CallExpr;
    type Context = Scope;

    async fn closure(&self, context: &mut Self::Context) -> Self::Output {
        let target = self.target().closure(context).await;
        let mut new_args = Vec::new();
        for arg in self.args() {
            new_args.push(Box::new(arg.closure(context).await));
        }
        CallExpr::new(target, new_args)
    }
}
