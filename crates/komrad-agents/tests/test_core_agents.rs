use komrad_ast::prelude::{
    CallExpr, Expr, Number, Statement, ToBlock, ToBoxedExpr, Value,
};

fn create_alice_agent_definition() -> Statement {
    Statement::Expr(Expr::Call(CallExpr::new(
        Expr::Variable("agent".into()),
        vec![
            Expr::Variable("Alice".into()).into(),
            vec![
                Statement::Assignment("x".into(), Expr::Value(Value::Number(Number::UInt(2))))
                    .into(),
                Statement::Expr(Expr::Call(CallExpr::new(
                    Expr::Variable("foo".into()),
                    vec![Expr::Variable("bar".into()).into()],
                ))),
            ]
            .to_block()
            .to_boxed_expr(),
        ],
    )))
}

#[tokio::test]
async fn test_registry_agent() {}
