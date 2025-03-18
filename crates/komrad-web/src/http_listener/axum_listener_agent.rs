use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Bytes;
// Import axum's Bytes.
use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade}, Path,
        Query,
    }, http::{self, StatusCode},
    response::IntoResponse,
    routing::get,
    Extension,
    Router,
};
use futures::StreamExt;
use http_body_util::Full as Body;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::service::TowerToHyperService;
use tokio::net::TcpListener;
use tokio::select;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::http_listener::config;
use crate::http_listener::config::ServerConfig;
use crate::http_listener::http_response_agent::HttpResponseAgent;
use komrad_agent::{Agent, AgentBehavior, AgentFactory, AgentLifecycle};
use komrad_ast::prelude::{Channel, ChannelListener, Message, Number, Value, ValueType};
use komrad_ast::scope::Scope;

/// Converts a final message (expected to be a 4-element list: [status, headers, cookies, body])
/// into a full Axum response with proper headers and body. This mirrors the behavior in your
/// Warp listener and HttpResponseAgent.
fn axum_response_from_komrad(terms: &[Value]) -> http::Response<Body<axum::body::Bytes>> {
    if terms.is_empty() {
        return http::Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("Content-Type", "text/plain")
            .body(Body::from(Bytes::from("Empty response")))
            .unwrap();
    }
    match &terms[0] {
        Value::List(list_of_4) if list_of_4.len() == 4 => {
            // 1) Extract status code.
            let status_code = if let Value::Number(n) = &list_of_4[0] {
                let raw = match n {
                    Number::Int(i) => *i,
                    Number::UInt(u) => *u as i64,
                    Number::Float(_) => 200,
                };
                if raw < 100 || raw > 599 {
                    200
                } else {
                    raw as u16
                }
            } else {
                200
            };

            // 2) Extract headers.
            let mut header_map = http::HeaderMap::new();
            if let Value::List(header_list) = &list_of_4[1] {
                for hpair in header_list {
                    if let Value::List(pair) = hpair {
                        if pair.len() == 2 {
                            if let (Value::String(k), Value::String(v)) = (&pair[0], &pair[1]) {
                                if let Ok(header_name) =
                                    http::header::HeaderName::from_bytes(k.as_bytes())
                                {
                                    if let Ok(header_value) = http::header::HeaderValue::from_str(v)
                                    {
                                        header_map.insert(header_name, header_value);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 3) Extract cookies as "Set-Cookie" headers.
            if let Value::List(cookie_list) = &list_of_4[2] {
                for cpair in cookie_list {
                    if let Value::List(pair) = cpair {
                        if pair.len() == 2 {
                            if let (Value::String(k), Value::String(v)) = (&pair[0], &pair[1]) {
                                let cookie = format!("{}={}", k, v);
                                if let Ok(cookie_value) =
                                    http::header::HeaderValue::from_str(&cookie)
                                {
                                    header_map.append(http::header::SET_COOKIE, cookie_value);
                                }
                            }
                        }
                    }
                }
            }

            // 4) Extract body (as bytes).
            let body_bytes = match &list_of_4[3] {
                Value::Bytes(b) => b.clone(),
                Value::String(s) => s.clone().into_bytes(),
                _ => Vec::new(),
            };

            // 5) Build the final response.
            let mut resp_builder = http::Response::builder()
                .status(http::StatusCode::from_u16(status_code).unwrap_or(StatusCode::OK));
            for (key, value) in header_map.iter() {
                resp_builder = resp_builder.header(key, value);
            }
            resp_builder
                .body(Body::from(Bytes::from(body_bytes)))
                .unwrap()
        }
        other => {
            // Fallback: treat the message as a string.
            let text = match other {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                _ => format!("Unsupported response type: {:?}", other),
            };
            http::Response::builder()
                .status(StatusCode::OK)
                .header(http::header::CONTENT_TYPE, "text/html")
                .body(Body::from(Bytes::from(text.into_bytes())))
                .unwrap()
        }
    }
}

async fn http_handler_root(
    Query(_query): Query<std::collections::HashMap<String, String>>,
    Extension(delegate_opt): Extension<Option<Channel>>,
) -> impl IntoResponse {
    http_handler(Path("".to_string()), Query(_query), Extension(delegate_opt)).await
}

/// HTTP handler that forwards requests to the delegate agent.
async fn http_handler(
    Path(path): Path<String>,
    Query(_query): Query<std::collections::HashMap<String, String>>,
    Extension(delegate_opt): Extension<Option<Channel>>,
) -> impl IntoResponse {
    let delegate = match delegate_opt {
        Some(ch) => ch,
        None => {
            return http::Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "text/plain")
                .body(Body::from(Bytes::from("No delegate channel")))
                .unwrap();
        }
    };

    let path_segments: Vec<Value> = path
        .split('/')
        .filter(|p| !p.is_empty())
        .map(|s| Value::String(s.to_string()))
        .collect();
    let method_str = "GET".to_string(); // Adjust if needed

    // Spawn an ephemeral HttpResponseAgent.
    let (final_tx, final_rx) = Channel::new(1);
    let response_agent = HttpResponseAgent::new("Response", Some(final_tx));
    let ephemeral_chan = response_agent.spawn();

    let mut msg_terms = vec![
        Value::Word("http".into()),
        Value::Channel(ephemeral_chan),
        Value::Word(method_str),
    ];
    msg_terms.extend(path_segments);

    let msg_to_delegate = Message::new(msg_terms, None);
    if let Err(e) = delegate.send(msg_to_delegate).await {
        error!("Failed sending message to delegate: {:?}", e);
        return http::Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("Content-Type", "text/plain")
            .body(Body::from(Bytes::from("Error forwarding to delegate")))
            .unwrap();
    }

    match final_rx.recv().await {
        Ok(final_msg) => axum_response_from_komrad(final_msg.terms()),
        Err(e) => {
            error!("Failed receiving final reply: {:?}", e);
            http::Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "text/plain")
                .body(Body::from(Bytes::from("No final reply")))
                .unwrap()
        }
    }
}

/// WebSocket handler that upgrades the connection and then calls handle_websocket.
async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(delegate_opt): Extension<Option<Channel>>,
) -> impl IntoResponse {
    let delegate = delegate_opt;
    ws.on_upgrade(move |socket| handle_websocket(socket, delegate))
}

/// Handles a WebSocket connection.
async fn handle_websocket(socket: WebSocket, delegate_opt: Option<Channel>) {
    let delegate = match delegate_opt {
        Some(ch) => ch,
        None => return,
    };

    let (_sender, mut receiver) = socket.split();

    while let Some(Ok(msg)) = receiver.next().await {
        if let axum::extract::ws::Message::Text(utf8bytes) = msg {
            let text = utf8bytes.to_string();
            let msg_to_delegate = Message::new(
                vec![
                    Value::Word("ws".into()),
                    Value::String("id".into()),
                    Value::String(text),
                ],
                None,
            );
            let _ = delegate.send(msg_to_delegate).await;
        }
    }
}

/// AxumListenerAgent uses its own TcpListener and a manual accept loop.
pub struct AxumListenerAgent {
    name: String,
    scope: Arc<Mutex<Scope>>,
    channel: Channel,
    listener: Arc<ChannelListener>,
    shutdown_token: CancellationToken,
    server_handle: Mutex<Option<JoinHandle<()>>>,
}

impl AxumListenerAgent {
    pub fn new(name: &str, initial_scope: Scope) -> Arc<Self> {
        let (ch, rx) = Channel::new(32);
        Arc::new(Self {
            name: name.to_string(),
            scope: Arc::new(Mutex::new(initial_scope)),
            channel: ch,
            listener: Arc::new(rx),
            shutdown_token: CancellationToken::new(),
            server_handle: Mutex::new(None),
        })
    }

    /// Starts the Axum server using a manual TcpListener and a custom accept loop.
    async fn run_server(
        self: Arc<Self>,
        host: String,
        port: u16,
        delegate: Option<Channel>,
        shutdown_token: CancellationToken,
    ) {
        // Build the Axum router with HTTP and WebSocket routes.
        let app = Router::new()
            .route("/ws", get(ws_handler))
            .route("/", get(http_handler_root))
            .route("/{*path}", get(http_handler))
            .layer(Extension(delegate));

        let addr_str = format!("{}:{}", host, port);
        let addr: SocketAddr = addr_str.parse().unwrap_or_else(|_| {
            warn!(
                "Invalid address: {}. Falling back to 0.0.0.0:3000",
                addr_str
            );
            SocketAddr::from(([0, 0, 0, 0], 3000))
        });

        // Bind our own TcpListener.
        let listener = match TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                error!("Failed to bind to {}: {}", addr, e);
                return;
            }
        };

        info!("AxumListenerAgent is listening on {}", addr);

        loop {
            select! {
                _ = shutdown_token.cancelled() => {
                    info!("AxumListenerAgent shutdown signal received");
                    break;
                }
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, peer_addr)) => {
                            let tower_service = app.clone();
                            let hyper_service = TowerToHyperService::new(tower_service);
                            tokio::spawn(async move {
                                let io = TokioIo::new(stream);
                                if let Err(err) = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
                                    .serve_connection(io, hyper_service)
                                    .await
                                {
                                    error!("Error serving connection from {}: {}", peer_addr, err);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                        }
                    }
                }
            }
        }
        info!("AxumListenerAgent server loop exiting");
    }

    fn start_server(
        self: Arc<Self>,
        address: Value,
        port: Value,
        delegate_val: Value,
    ) -> JoinHandle<()> {
        // Parse host from Value.
        let host = if let Value::String(s) = address {
            s
        } else {
            "0.0.0.0".to_string()
        };

        // Parse port from Value.
        let port_num = if let Value::Number(Number::UInt(u)) = port {
            u as u16
        } else if let Value::Number(Number::Int(i)) = port {
            if i > 0 { i as u16 } else { 3000 }
        } else {
            3000
        };

        // Parse delegate channel.
        let delegate_chan = if let Value::Channel(ch) = delegate_val {
            Some(ch)
        } else {
            None
        };

        let shutdown = self.shutdown_token.clone();
        let self_clone = self.clone();
        let cancellation_token = self.shutdown_token.clone();
        tokio::spawn(async move {
            self_clone
                .run_server(host, port_num, delegate_chan, cancellation_token)
                .await;
            shutdown.cancelled().await;
        })
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for AxumListenerAgent {
    async fn init(self: Arc<Self>, scope: &mut Scope) {
        debug!("Initializing AxumListenerAgent: {}", self.name);
        let config = config::parse_server_config_from_scope(scope);
        let handle = self
            .clone()
            .start_server(config.address, config.port, config.delegate);
        self.server_handle.lock().await.replace(handle);
    }

    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
        self.scope.clone()
    }

    async fn stop(&self) {
        info!("Stopping AxumListenerAgent: {}", self.name);
        self.shutdown_token.cancel();
        if let Some(handle) = self.server_handle.lock().await.take() {
            if let Err(e) = handle.await {
                error!("Error stopping Axum server: {:?}", e);
            }
        }
        self.stop_in_scope().await;
    }

    fn channel(&self) -> &Channel {
        &self.channel
    }

    fn listener(&self) -> Arc<ChannelListener> {
        self.listener.clone()
    }
}

#[async_trait::async_trait]
impl AgentBehavior for AxumListenerAgent {
    async fn handle_message(&self, _msg: Message) -> bool {
        true
    }
}

impl Agent for AxumListenerAgent {}

pub struct AxumListenerFactory;

impl AgentFactory for AxumListenerFactory {
    fn create_agent(&self, name: &str, initial_scope: Scope) -> Arc<dyn Agent> {
        AxumListenerAgent::new(name, initial_scope)
    }
}
