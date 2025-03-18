use crate::http_listener::config::{parse_server_config_from_scope, ServerConfig};
use crate::http_listener::http_response_agent::HttpResponseAgent;
use crate::request::{full, KomradRequest};
use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::combinators::BoxBody;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use komrad_agent::{Agent, AgentBehavior, AgentFactory, AgentLifecycle};
use komrad_ast::prelude::{Channel, ChannelListener, Message, Number, Scope, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::select;
use tokio::sync::Mutex;
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
                                        echo(req, delegate_value.clone())
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

async fn echo(
    req: Request<hyper::body::Incoming>,
    delegate_value: Value,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let komrad_req = KomradRequest::from_request(req).await;
    let method = komrad_req.method.to_uppercase();
    let path_segments = komrad_req.path.clone();

    let delegate = match delegate_value {
        Value::Channel(ref chan) => Some(chan.clone()),
        _ => None,
    };

    if delegate.is_none() {
        return Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(full("No delegate channel found"))
            .unwrap());
    }
    let delegate_chan = delegate.unwrap();

    // Spawn an ephemeral response agent so that the delegate knows where to send its reply.
    let (final_tx, final_rx) = Channel::new(1);
    let response_agent = HttpResponseAgent::new("Response", Some(final_tx.clone()));
    let ephemeral_chan = response_agent.spawn();

    // Build message with ephemeral channel as the second term.
    let mut msg_terms = vec![
        Value::Word("http".into()),
        Value::Channel(ephemeral_chan),
        Value::Word(method),
    ];
    msg_terms.extend(path_segments.into_iter().map(Value::String));

    let msg = Message::new(msg_terms, None);
    if let Err(e) = delegate_chan.send(msg).await {
        error!("Failed to send message to delegate: {:?}", e);
        return Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(full("Error sending request"))
            .unwrap());
    }

    match final_rx.recv().await {
        Ok(final_msg) => Ok(build_response_from_komrad(final_msg.terms())),
        Err(e) => {
            error!("Failed to receive delegate response: {:?}", e);
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(full("Error receiving response"))
                .unwrap())
        }
    }
}

/// Converts a final 4-element Komrad response to an HTTP Response:
/// Expected format: [status, headers, cookies, body]
fn build_response_from_komrad(terms: &[Value]) -> Response<BoxBody<Bytes, hyper::Error>> {
    if terms.is_empty() {
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("Content-Type", "text/plain")
            .body(full("Empty response"))
            .unwrap();
    }

    match &terms[0] {
        Value::List(list_of_4) if list_of_4.len() == 4 => {
            let status_code = match &list_of_4[0] {
                Value::Number(n) => match n {
                    Number::Int(i) => *i as u16,
                    Number::UInt(i) => *i as u16,
                    Number::Float(f) => *f as u16,
                },
                _ => 200,
            };

            let mut builder = Response::builder().status(status_code);
            if let Value::List(header_list) = &list_of_4[1] {
                for hpair in header_list {
                    if let Value::List(pair) = hpair {
                        if pair.len() == 2 {
                            if let (Value::String(k), Value::String(v)) = (&pair[0], &pair[1]) {
                                builder = builder.header(k.as_str(), v.as_str());
                            }
                        }
                    }
                }
            }

            if let Value::List(cookie_list) = &list_of_4[2] {
                for cpair in cookie_list {
                    if let Value::List(pair) = cpair {
                        if pair.len() == 2 {
                            if let (Value::String(k), Value::String(v)) = (&pair[0], &pair[1]) {
                                builder = builder.header("Set-Cookie", format!("{}={}", k, v));
                            }
                        }
                    }
                }
            }

            let body_bytes = match &list_of_4[3] {
                Value::Bytes(b) => b.clone(),
                Value::String(s) => s.as_bytes().to_vec(),
                other => format!("{:?}", other).into_bytes(),
            };
            builder
                .body(full(Bytes::from(body_bytes)))
                .unwrap_or_else(|_| {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(full("Error building response"))
                        .unwrap()
                })
        }
        other => {
            let text = match other {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                _ => format!("Unsupported response type: {:?}", other),
            };
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/plain")
                .body(full(Bytes::from(text)))
                .unwrap()
        }
    }
}

//
// Implementing the agent traits
//

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
