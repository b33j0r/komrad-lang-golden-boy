use crate::config::{parse_server_config_from_scope, ServerConfig};
use crate::http_request_agent::HttpRequestAgent;
use crate::http_response_agent::HttpResponseAgent;
use crate::response::{self, full};
use crate::websocket_agent::WebSocketAgent;
use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use hyper::body;
use hyper::body::Body;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use komrad_agent::{Agent, AgentBehavior, AgentFactory, AgentLifecycle};
use komrad_ast::prelude::{Channel, ChannelListener, Message, MessageBuilder, Scope, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::select;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::WebSocketStream;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use tungstenite::protocol::Message as WsMessage;

/// Computes the Sec-WebSocket-Accept header value as specified in RFC 6455.
fn compute_accept_key(key: &str) -> String {
    use base64::Engine as _;
    use base64::engine::general_purpose;
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    let result = hasher.finalize();
    general_purpose::STANDARD.encode(result)
}

/// Checks if the request contains the required headers for a WebSocket upgrade.
fn is_websocket_request(req: &Request<body::Incoming>) -> bool {
    // Check the "upgrade" header.
    if let Some(upgrade) = req.headers().get("upgrade") {
        if upgrade
            .to_str()
            .map(|s| !s.eq_ignore_ascii_case("websocket"))
            .unwrap_or(true)
        {
            return false;
        }
    } else {
        return false;
    }
    // Check the "connection" header for "Upgrade".
    if let Some(conn) = req.headers().get("connection") {
        if let Ok(conn_str) = conn.to_str() {
            if !conn_str.to_ascii_lowercase().contains("upgrade") {
                return false;
            }
        } else {
            return false;
        }
    } else {
        return false;
    }
    // Check for the presence of the Sec-WebSocket-Key and Version headers.
    req.headers().contains_key("sec-websocket-key")
        && req
            .headers()
            .get("sec-websocket-version")
            .map_or(false, |v| v == "13")
}

pub struct HyperListenerAgent {
    name: String,
    scope: Arc<Mutex<Scope>>,
    channel: Channel,
    listener: Arc<ChannelListener>,
    server_handle: Mutex<Option<JoinHandle<()>>>,
    shutdown_token: CancellationToken,
    config: ServerConfig,
}

impl HyperListenerAgent {
    pub fn new(name: &str, initial_scope: &Scope) -> Arc<Self> {
        let (channel, listener) = Channel::new(32);
        let config = parse_server_config_from_scope(initial_scope);
        Arc::new(Self {
            name: name.to_string(),
            scope: Arc::new(Mutex::new(initial_scope.clone())),
            channel,
            listener: Arc::new(listener),
            server_handle: Mutex::new(None),
            shutdown_token: CancellationToken::new(),
            config,
        })
    }

    async fn run_server(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr: SocketAddr = format!("{}:{}", self.config.address, self.config.port).parse()?;
        let listener = TcpListener::bind(addr).await?;
        info!("Hyper HTTP server listening on http://{}", addr);
        let delegate_value = self.config.delegate.clone();
        let shutdown = self.shutdown_token.clone();
        loop {
            select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, _)) => {
                            let io = TokioIo::new(stream);
                            let delegate_value = delegate_value.clone();
                            tokio::spawn(async move {
                                if let Err(err) = http1::Builder::new()
                                    .serve_connection(io, service_fn(move |req| {
                                        handle_request(req, delegate_value.clone())
                                    })).with_upgrades()
                                    .await {
                                    error!("Error serving connection: {:?}", err);
                                }
                            });
                        },
                        Err(e) => {
                            error!("Failed to accept connection: {:?}", e);
                        }
                    }
                },
                _ = shutdown.cancelled() => {
                    warn!("HyperListenerAgent shutting down gracefully...");
                    break;
                }
            }
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for HyperListenerAgent {
    async fn init(self: Arc<Self>, _scope: &mut Scope) {
        debug!("Initializing HyperListenerAgent: {}", self.name);
        let this = self.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = this.run_server().await {
                error!("Error in run_server: {:?}", e);
            }
        });
        *self.server_handle.lock().await = Some(handle);
    }

    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
        self.scope.clone()
    }

    async fn stop(&self) {
        warn!("Stopping HyperListenerAgent: {}", self.name);
        self.shutdown_token.cancel();
        if let Some(handle) = self.server_handle.lock().await.take() {
            let _ = handle.await;
        }
    }

    fn channel(&self) -> &Channel {
        &self.channel
    }

    fn listener(&self) -> Arc<ChannelListener> {
        self.listener.clone()
    }
}

#[async_trait::async_trait]
impl AgentBehavior for HyperListenerAgent {
    async fn handle_message(&self, _msg: Message) -> bool {
        true
    }
}

impl Agent for HyperListenerAgent {}

pub struct HyperListenerFactory;

impl AgentFactory for HyperListenerFactory {
    fn create_agent(&self, name: &str, initial_scope: Scope) -> Arc<dyn Agent> {
        HyperListenerAgent::new(name, &initial_scope)
    }
}

/// Main request handler.
/// If the request is a WebSocket upgrade request, it is handled in handle_websocket_upgrade;
/// otherwise, normal HTTP request processing is performed.
async fn handle_request(
    mut req: Request<body::Incoming>,
    delegate_value: Value,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    if is_websocket_request(&req) {
        error!("WebSocket upgrade request detected on {}", req.uri());
        let response = handle_websocket_upgrade(req, delegate_value).await;
        error!("WebSocket upgrade request handled: {:?}", response);
        response
    } else {
        let request_agent = HttpRequestAgent::new("Request", req).await;
        let ephemeral_request_chan = request_agent.clone().spawn();
        let method = request_agent.method().to_string();
        let path_values = request_agent
            .path()
            .iter()
            .map(|s| Value::String(s.to_string()))
            .collect::<Vec<_>>();
        let delegate_chan = match delegate_value {
            Value::Channel(chan) => chan,
            _ => {
                error!("Delegate is not a channel");
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(full("Delegate is not a channel"))
                    .unwrap());
            }
        };
        let (final_tx, final_rx) = Channel::new(1);
        let response_agent = HttpResponseAgent::new("Response", Some(final_tx));
        let ephemeral_response_chan = response_agent.spawn();
        let mut msg_terms = vec![
            Value::Word("http".into()),
            Value::Channel(ephemeral_request_chan),
            Value::Channel(ephemeral_response_chan),
            Value::Word(method.into()),
        ];
        msg_terms.extend(path_values);
        if let Err(e) = delegate_chan.send(Message::new(msg_terms, None)).await {
            error!("Failed sending msg to delegate: {:?}", e);
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(full("Error sending request"))
                .unwrap());
        }
        let final_msg = match final_rx.recv().await {
            Ok(m) => m,
            Err(e) => {
                error!("Delegate recv error: {:?}", e);
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(full("Error receiving delegate response"))
                    .unwrap());
            }
        };
        let terms = final_msg.terms();
        let resp = response::build_hyper_response_from_komrad(&terms);
        Ok(resp)
    }
}

/// Handles WebSocket upgrade requests using tokio-tungstenite directly.
/// This function checks the required headers, builds the 101 response,
/// captures the upgrade future before constructing the response,
/// spawns a task to await the upgrade, and then creates a WebSocketStream.
async fn handle_websocket_upgrade(
    mut req: Request<body::Incoming>,
    delegate_value: Value,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let delegate_channel = match delegate_value {
        Value::Channel(chan) => chan,
        _ => {
            error!("Delegate is not a channel");
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(full("Delegate is not a channel"))
                .unwrap());
        }
    };

    if !is_websocket_request(&req) {
        error!("Invalid WebSocket upgrade request");
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(full("Invalid WebSocket upgrade request"))
            .unwrap());
    }

    // Extract the Sec-WebSocket-Key header and compute the accept key.
    let key = req
        .headers()
        .get("sec-websocket-key")
        .and_then(|val| val.to_str().ok())
        .unwrap_or("");
    let accept_key = compute_accept_key(key);

    // Capture the upgrade future BEFORE constructing the response.
    let upgrade_future = hyper::upgrade::on(&mut req);

    // Build the WebSocket handshake response.
    let response = Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .header("Sec-WebSocket-Accept", accept_key)
        .body(full(""))
        .unwrap();

    // Spawn a task to handle the upgrade once the connection is upgraded.
    tokio::spawn(async move {
        match upgrade_future.await {
            Ok(upgraded) => {
                let upgraded = TokioIo::new(upgraded);
                let ws_stream =
                    WebSocketStream::from_raw_socket(upgraded, Role::Server, None).await;
                let ws_agent = WebSocketAgent::new("WebSocket", ws_stream);
                let ws_channel = ws_agent.spawn();
                let ws_channel_clone = ws_channel.clone();

                let (ephemeral_onboarding_channel, ephemeral_onboarding_listener) = Channel::new(1);

                let msg = Message::new(
                    vec![
                        Value::Word("ws".into()),
                        Value::Channel(ws_channel),
                        Value::Word("connect".into()),
                    ],
                    Some(ephemeral_onboarding_channel),
                );
                if let Err(e) = delegate_channel.send(msg).await {
                    error!("Failed to send message to delegate: {:?}", e);
                } else {
                    // THe onboarding message is expected to be a delegate channel for the connection
                    // to send messages to the delegate.
                    match ephemeral_onboarding_listener.recv().await {
                        Ok(onboarding_msg) => {
                            let onboarding_terms = onboarding_msg.terms();
                            if onboarding_terms.len() == 1 {
                                if let Some(Value::Channel(websocket_delegate_channel)) =
                                    onboarding_terms.get(0)
                                {
                                    match ws_channel_clone
                                        .send(Message::default().with_terms(vec![
                                            Value::Word("set-delegate".into()),
                                            Value::Channel(websocket_delegate_channel.clone()),
                                        ]))
                                        .await
                                    {
                                        Ok(_) => {
                                            info!("WebSocket agent connected to delegate channel");
                                        }
                                        Err(e) => {
                                            error!("Failed to set delegate channel: {:?}", e);
                                        }
                                    }
                                }
                            } else {
                                error!("Invalid onboarding message: {:?}", onboarding_terms);
                            }
                        }
                        Err(e) => {
                            error!("Failed to receive onboarding message: {:?}", e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("WebSocket upgrade error: {:?}", e);
            }
        }
    });

    // Convert the handshake response body to a BoxBody.
    let converted_response =
        response.map(|body| BoxBody::new(body.map_err(|_| panic!("Infallible error occurred"))));
    Ok(converted_response)
}
