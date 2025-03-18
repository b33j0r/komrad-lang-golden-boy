use crate::config::{parse_server_config_from_scope, ServerConfig};
use crate::http_request_agent::HttpRequestAgent;
use crate::http_response_agent::HttpResponseAgent;
use crate::response;
use crate::websocket_agent::WebSocketAgent;
use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::combinators::BoxBody;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::upgrade::OnUpgrade;
use hyper_util::rt::TokioIo;
use komrad_agent::{Agent, AgentBehavior, AgentFactory, AgentLifecycle};
use komrad_ast::prelude::{Channel, ChannelListener, Message, Scope, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::select;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::WebSocketStream;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

pub struct HyperListenerAgent {
    name: String,
    scope: Arc<Mutex<Scope>>,
    channel: Channel,
    listener: Arc<ChannelListener>,
    server_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
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
        let addr = format!("{}:{}", self.config.address, self.config.port).parse::<SocketAddr>()?;
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

                            tokio::task::spawn(async move {
                                if let Err(err) = http1::Builder::new()
                                    .serve_connection(io, service_fn(move |req| {
                                        handle_request(req, delegate_value.clone())
                                    }))
                                    .await
                                {
                                    error!("Error serving connection: {:?}", err);
                                }
                            });
                        }
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

async fn handle_request(
    mut req: Request<hyper::body::Incoming>,
    delegate_value: Value,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    // Pre-emptively get the upgrade handle, because the req is consumed when we call collect()
    // in the HttpRequestAgent
    let on_upgrade: OnUpgrade = hyper::upgrade::on(&mut req);

    // Build the ephemeral HttpRequestAgent
    let request_agent = HttpRequestAgent::new("Request", req).await;
    let ephemeral_request_chan = request_agent.clone().spawn();

    // Extract the method and path values
    let method = request_agent.method().to_string();
    let path_values = request_agent
        .path()
        .iter()
        .map(|s| Value::String(s.to_string()))
        .collect::<Vec<_>>();

    // Extract the server's delegate channel
    let delegate_chan = match delegate_value {
        Value::Channel(chan) => chan,
        _ => {
            error!("Delegate is not a channel");
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(response::full("Delegate is not a channel"))
                .unwrap());
        }
    };

    // Build ephemeral HttpResponseAgent
    let (final_tx, final_rx) = Channel::new(1);
    let response_agent = HttpResponseAgent::new("Response", Some(final_tx));
    let ephemeral_response_chan = response_agent.spawn();

    // Send [http requestChan responseChan method path...]
    let mut msg_terms = vec![
        Value::Word("http".into()),
        Value::Channel(ephemeral_request_chan),
        Value::Channel(ephemeral_response_chan),
        Value::Word(method.into()),
    ];
    // Add the rest of the path components
    msg_terms.extend(path_values);

    // Send the message to the delegate
    if let Err(e) = delegate_chan.send(Message::new(msg_terms, None)).await {
        error!("Failed sending msg to delegate: {:?}", e);
        return Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(response::full("Error sending request"))
            .unwrap());
    }

    // Wait for the final [status, headers, cookies, body, websocketDelegate]
    let final_msg = match final_rx.recv().await {
        Ok(m) => m,
        Err(e) => {
            error!("Delegate recv error: {:?}", e);
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(response::full("Error receiving delegate response"))
                .unwrap());
        }
    };

    // Pull the terms from the final message
    let terms = final_msg.terms();

    // Check for a websocket upgrade
    let ws_delegate_value = match terms.get(4) {
        Some(Value::Channel(chan)) => Some(chan.clone()),
        _ => None,
    };

    // Build an HTTP response from the 4 main fields
    let resp = response::build_hyper_response_from_komrad(terms);

    // If 5th element is a channel, we treat it as "websocket delegate"
    if let Some(ws_delegate_value) = ws_delegate_value {
        // We do the actual Hyper "upgrade" in a background task
        tokio::spawn(async move {
            match on_upgrade.await {
                Ok(upgraded) => {
                    // Wrap upgraded with .compat() so it implements AsyncRead + AsyncWrite
                    let compat_stream = TokioIo::new(upgraded);

                    // Use tungstenite to form a WebSocket
                    let ws_stream =
                        WebSocketStream::from_raw_socket(compat_stream, Role::Server, None).await;

                    // Create the WebSocketAgent that will forward frames
                    let ws_agent =
                        WebSocketAgent::new("WsAgent", ws_stream, ws_delegate_value.clone());
                    ws_agent.spawn();
                    error!("WebSocket upgrade successful");
                }
                Err(err) => {
                    error!("WebSocket upgrade failed: {:?}", err);
                }
            }
        });
    }

    Ok(resp)
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
