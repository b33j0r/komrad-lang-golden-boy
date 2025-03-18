use komrad_ast::prelude::{Number, Scope, Value};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub address: String,
    pub port: u16,
    pub delegate: Value,
    pub websocket_path: Option<String>,
}

pub fn parse_server_config_from_scope(scope: &Scope) -> ServerConfig {
    let address = scope
        .get("host")
        .unwrap_or(Value::String("0.0.0.0".to_string()));
    let port = scope
        .get("port")
        .unwrap_or(Value::Number(Number::UInt(3000)));
    let delegate = scope.get("delegate").unwrap_or(Value::Empty);
    let websocket_path = scope.get("websocket_path").unwrap_or(Value::Empty);
    ServerConfig {
        address: address.to_string(),
        port: match port {
            Value::Number(Number::UInt(p)) => p as u16,
            Value::Number(Number::Int(p)) => p as u16,
            _ => 3000,
        },
        delegate,
        websocket_path: match websocket_path {
            Value::String(path) => Some(path.to_string()),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use komrad_ast::prelude::{Channel, Scope};

    #[tokio::test]
    async fn test_parse_server_config_from_scope() {
        let mut scope = Scope::new();
        let channel = Channel::new(1).0;
        scope
            .set("host".to_string(), Value::String("1.0.0.0".to_string()))
            .await;
        scope
            .set("port".to_string(), Value::Number(Number::UInt(4300)))
            .await;
        scope
            .set("delegate".to_string(), Value::Channel(channel.clone()))
            .await;
        let config = parse_server_config_from_scope(&mut scope);
        assert_eq!(config.address, "1.0.0.0");
        assert_eq!(config.port, 4300);
        assert_eq!(config.delegate, Value::Channel(channel));
    }
}
