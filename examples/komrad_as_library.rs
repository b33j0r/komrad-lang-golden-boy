use komrad_ast::number::Number;
use komrad_ast::prelude::{BinaryExpr, BinaryOp, Expr};
use komrad_ast::value::Value;
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
    let scope = module.get_scope().await;
    // Retrieve the value of x
    let x_value = scope.get("x").await.unwrap();
    // Print the value of x
    info!("Value of x: {:?}", x_value);

    // Set y to 3.0 manually
    module
        .send_command(ModuleCommand::ModifyScope {
            key: "y".to_string(),
            value: Value::Number(Number::from(3.0)),
        })
        .await;

    // Get the module scope again
    let scope = module.get_scope().await;
    // Retrieve the value of y
    let y_value = scope.get("y").await.unwrap();
    // Print the value of y
    info!("Value of y: {:?}", y_value);

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
    let scope = module.get_scope().await;
    // Retrieve the updated value of x
    let updated_x_value = scope.get("x").await.unwrap();
    // Print the updated value of x
    info!("Updated value of x: {:?}", updated_x_value);

    // (Optional) Stop the module to cleanly end the actor task
    module.send_command(ModuleCommand::Stop).await;
}
