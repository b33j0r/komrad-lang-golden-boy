use komrad_ast::prelude::{BinaryExpr, BinaryOp, Expr, Number, Value};
use komrad_vm::ModuleCommand;
use tracing::info;

#[tokio::main]
pub async fn main() {
    // Initialize logging so you can see debug/warn messages
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .without_time()
        .init();

    // Create the global "System"
    let system = komrad_vm::System::spawn().await;

    // Spin up a "main" module
    let module = system.create_module("main").await;

    // Create a simple statement:  x = 42.0
    let assignment_statement = komrad_ast::prelude::Statement::Assignment(
        "x".to_string(),
        Expr::Value(Value::Number(Number::from(42.0))),
    );

    // Execute that statement in the module
    module
        .send_command(ModuleCommand::ExecuteStatement(assignment_statement))
        .await;

    // Get the module scope
    if let Some(scope) = module.get_scope().await {
        // Print the scope
        info!("Module scope: {:?}", scope);
    } else {
        info!("Failed to get module scope");
    }

    // Set y to 3.0 manually
    module
        .send_command(ModuleCommand::ModifyScope {
            key: "y".to_string(),
            value: Value::Number(Number::from(3.0)),
        })
        .await;

    // Get the module scope again
    if let Some(scope) = module.get_scope().await {
        // Print the scope
        info!("Module scope: {:?}", scope);
    } else {
        info!("Failed to get module scope");
    }

    // Send a multiply by 2 command
    let multiply_statement = komrad_ast::prelude::Statement::Assignment(
        "x".to_string(),
        Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Variable("x".to_string())),
            op: BinaryOp::Mul,
            right: Box::new(Expr::Variable("y".to_string())),
        }),
    );

    // Execute the multiply statement
    module
        .send_command(ModuleCommand::ExecuteStatement(multiply_statement))
        .await;

    // Get the module scope
    if let Some(scope) = module.get_scope().await {
        // Print the scope
        info!("Module scope: {:?}", scope);
    } else {
        info!("Failed to get module scope");
    }

    // (Optional) Stop the module to cleanly end the actor task
    module.send_command(ModuleCommand::Stop).await;
}
