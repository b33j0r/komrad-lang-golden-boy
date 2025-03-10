use komrad_ast::prelude::{BinaryExpr, BinaryOp, Expr, Number, Statement};
use komrad_ast::value::Value;
use komrad_vm::{ModuleCommand, System};
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_module_assignment_and_retrieval() {
    let system = System::spawn().await;
    let module = system.create_module("test_module").await;

    // Assignment: x = 100
    let assign_x = Statement::Assignment(
        "x".to_string(),
        Expr::Value(Value::Number(Number::from(100.0))),
    );

    module
        .send_command(ModuleCommand::ExecuteStatement(assign_x))
        .await;

    sleep(Duration::from_millis(1)).await;

    if let Some(scope) = module.get_scope().await {
        let x = scope.get("x").await;
        assert_eq!(x, Some(Value::Number(Number::from(100.0))));
    } else {
        panic!("Scope was inaccessible");
    }
}

#[tokio::test]
async fn test_module_modify_scope_directly() {
    let system = System::spawn().await;
    let module = system.create_module("scope_test_module").await;

    module
        .send_command(ModuleCommand::ModifyScope {
            key: "y".to_string(),
            value: Value::Number(Number::from(25.0)),
        })
        .await;

    sleep(Duration::from_millis(1)).await;

    if let Some(scope) = module.get_scope().await {
        let y = scope.get("y").await;
        assert_eq!(y, Some(Value::Number(Number::from(25.0))));
    } else {
        panic!("Scope was inaccessible");
    }
}

#[tokio::test]
async fn test_module_binary_operation_execution() {
    let system = System::spawn().await;
    let module = system.create_module("binary_op_module").await;

    // Set initial values
    module
        .send_command(ModuleCommand::ModifyScope {
            key: "a".to_string(),
            value: Value::Number(Number::from(10.0)),
        })
        .await;

    module
        .send_command(ModuleCommand::ModifyScope {
            key: "b".to_string(),
            value: Value::Number(Number::from(20.0)),
        })
        .await;

    // Perform binary addition: c = a + b
    let assign_c = Statement::Assignment(
        "c".to_string(),
        Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Variable("a".to_string())),
            op: BinaryOp::Add,
            right: Box::new(Expr::Variable("b".to_string())),
        }),
    );

    module
        .send_command(ModuleCommand::ExecuteStatement(assign_c))
        .await;

    sleep(Duration::from_millis(1)).await;

    if let Some(scope) = module.get_scope().await {
        let c = scope.get("c").await;
        assert_eq!(c, Some(Value::Number(Number::from(30.0))));
    } else {
        panic!("Scope was inaccessible");
    }
}

#[tokio::test]
async fn test_system_module_lookup() {
    let system = System::spawn().await;
    let module = system.create_module("lookup_module").await;

    let fetched_module = system.get_module_by_id(&module.id);
    assert!(fetched_module.is_some());
    assert_eq!(fetched_module.unwrap().name, "lookup_module");
}

#[tokio::test]
async fn test_module_stop_command() {
    let system = System::spawn().await;
    let module = system.create_module("stop_module").await;

    module.send_command(ModuleCommand::Stop).await;
    sleep(Duration::from_millis(1)).await;

    // Since the module is stopped, commands should fail silently or log warnings
    module
        .send_command(ModuleCommand::ModifyScope {
            key: "should_fail".to_string(),
            value: Value::Number(Number::from(999.0)),
        })
        .await;

    sleep(Duration::from_millis(1)).await;

    if let Some(scope) = module.get_scope().await {
        panic!("Scope should be empty after module stop: {:?}", scope);
    }
}
