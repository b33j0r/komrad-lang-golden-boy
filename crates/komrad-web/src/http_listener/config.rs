use komrad_ast::prelude::{Number, Scope, Value};

pub fn parse_server_config_from_scope(scope: &mut Scope) -> (Value, Value, Value) {
    let address = scope
        .get("host")
        .unwrap_or(Value::String("0.0.0.0".to_string()));
    let port = scope
        .get("port")
        .unwrap_or(Value::Number(Number::UInt(3000)));
    let delegate = scope.get("delegate").unwrap_or(Value::Empty);
    (address, port, delegate)
}
