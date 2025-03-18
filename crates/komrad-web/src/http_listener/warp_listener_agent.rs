use crate::config::parse_server_config_from_scope;
use crate::http_response_agent::HttpResponseAgent;
use komrad_agent::{Agent, AgentBehavior, AgentFactory, AgentLifecycle};
use komrad_ast::prelude::{Channel, ChannelListener, Message, Number, Value};
use komrad_ast::scope::Scope;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use warp::http::{self, Response};
use warp::{hyper, Filter, Rejection, Reply};

/// Converts the final list-based response (expected [status, headers, cookies, body])
/// into a Warp `Response<hyper::Body>`. All branches return the same type.
fn warp_response_from_komrad(terms: &[Value]) -> Response<hyper::Body> {
    // If empty, respond with an error response.
    if terms.is_empty() {
        return http::Response::builder()
            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
            .header(http::header::CONTENT_TYPE, "text/plain")
            .body(hyper::Body::from("Empty response".as_bytes().to_vec()))
            .unwrap();
    }
    match &terms[0] {
        // Expect 4-element list: [status, headers, cookies, body]
        Value::List(list_of_4) if list_of_4.len() == 4 => {
            // 1) Extract status code.
            let status_code = if let Value::Number(n) = &list_of_4[0] {
                let raw = match n {
                    Number::Int(i) => *i,
                    Number::UInt(u) => *u as i64,
                    Number::Float(_) => 200,
                };
                if raw < 100 || raw > 599 {
                    error!("Invalid status code: {}", raw);
                    200
                } else {
                    raw as u16
                }
            } else {
                200
            };

            // 2) Extract normal headers.
            let mut header_map = http::HeaderMap::new();
            if let Value::List(header_list) = &list_of_4[1] {
                for hpair in header_list {
                    if let Value::List(pair) = hpair {
                        if pair.len() == 2 {
                            if let (Value::String(k), Value::String(v)) = (&pair[0], &pair[1]) {
                                header_map.insert(
                                    http::header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                                    http::header::HeaderValue::from_str(v).unwrap(),
                                );
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
                                header_map.append(
                                    http::header::SET_COOKIE,
                                    http::header::HeaderValue::from_str(&format!("{}={}", k, v))
                                        .unwrap(),
                                );
                            }
                        }
                    }
                }
            }

            // 4) Extract body without altering binary data.
            let body: Vec<u8> = match &list_of_4[3] {
                Value::Bytes(b) => b.clone(),
                Value::String(s) => s.clone().into_bytes(),
                _ => Vec::new(),
            };

            // Build the response.
            let mut resp_builder = http::Response::builder()
                .status(http::StatusCode::from_u16(status_code).unwrap_or(http::StatusCode::OK));
            for (key, value) in header_map.iter() {
                resp_builder = resp_builder.header(key, value);
            }
            resp_builder.body(hyper::Body::from(body)).unwrap()
        }
        // Fallback for non 4-element final messages: treat as string.
        other => {
            let text = match other {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Embedded(e) => e.text.clone(),
                _ => format!("Unsupported response type: {:?}", other),
            };
            http::Response::builder()
                .status(http::StatusCode::OK)
                .header(http::header::CONTENT_TYPE, "text/html")
                .body(hyper::Body::from(text.into_bytes()))
                .unwrap()
        }
    }
}

fn build_route(
    delegate: Option<Channel>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path::full().and(warp::method()).and_then(
        move |path: warp::filters::path::FullPath, method: warp::http::Method| {
            let delegate = delegate.clone();
            async move {
                if let Some(delegate) = &delegate {
                    debug!("{} {} -> {}", method, path.as_str(), delegate.uuid(),);

                    let path_segments: Vec<Value> = path
                        .as_str()
                        .split('/')
                        .filter(|s| !s.is_empty())
                        .map(|s| Value::String(s.to_string()))
                        .collect();

                    // Create a final reply channel for the ephemeral agent.
                    let (final_tx, final_rx) = Channel::new(1);
                    let response_agent = HttpResponseAgent::new("Response", Some(final_tx.clone()));
                    let ephemeral_chan = response_agent.spawn();

                    // Build message terms in the order: [ "http", <response>, <METHOD>, ... ]
                    let mut message_terms = vec![
                        Value::Word("http".into()),
                        Value::Channel(ephemeral_chan), // Binds to _response in user code.
                        Value::Word(method.to_string().to_uppercase()),
                    ];
                    message_terms.extend(path_segments);

                    // No reply_to is passed here because the ephemeral agent sends final output to final_tx.
                    let msg_to_delegate = Message::new(message_terms, None);
                    if let Err(e) = delegate.send(msg_to_delegate).await {
                        error!("Failed to send message to delegate: {:?}", e);
                        return Ok::<_, Rejection>(
                            http::Response::builder()
                                .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                                .header(http::header::CONTENT_TYPE, "text/plain")
                                .body(hyper::Body::from("Error".as_bytes().to_vec()))
                                .unwrap(),
                        );
                    }

                    // Wait for the final response from the ephemeral agent.
                    match final_rx.recv().await {
                        Ok(final_msg) => {
                            let resp = warp_response_from_komrad(final_msg.terms());
                            Ok::<_, Rejection>(resp)
                        }
                        Err(e) => {
                            error!(
                                "Failed to receive final reply from ephemeral agent: {:?}",
                                e
                            );
                            Ok::<_, Rejection>(
                                http::Response::builder()
                                    .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                                    .header(http::header::CONTENT_TYPE, "text/plain")
                                    .body(hyper::Body::from(
                                        "Error receiving final reply".as_bytes().to_vec(),
                                    ))
                                    .unwrap(),
                            )
                        }
                    }
                } else {
                    Ok::<_, Rejection>(
                        http::Response::builder()
                            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                            .header(http::header::CONTENT_TYPE, "text/plain")
                            .body(hyper::Body::from(
                                "No delegate was found to return a response"
                                    .as_bytes()
                                    .to_vec(),
                            ))
                            .unwrap(),
                    )
                }
            }
        },
    )
}

pub struct WarpListenerAgent {
    _name: String,
    scope: Arc<Mutex<Scope>>,
    channel: Channel,
    listener: Arc<ChannelListener>,
    warp_handle: Mutex<Option<JoinHandle<()>>>,
    warp_shutdown: CancellationToken,
}

impl WarpListenerAgent {
    pub fn new(name: &str, initial_scope: Scope) -> Arc<Self> {
        error!("Creating HttpListenerAgent");
        let (channel, listener) = Channel::new(32);
        Arc::new(Self {
            _name: name.to_string(),
            scope: Arc::new(Mutex::new(initial_scope)),
            channel,
            listener: Arc::new(listener),
            warp_handle: Mutex::new(None),
            warp_shutdown: CancellationToken::new(),
        })
    }

    fn start_server(&self, addr_str: String, port_num: u16, delegate: Value) -> JoinHandle<()> {
        error!("Starting Warp HTTP server");
        let delegate_channel = match delegate {
            Value::Channel(c) => Some(c),
            _ => None,
        };

        // Construct the socket address
        let socket_str = format!("{}:{}", addr_str, port_num);
        let socket_addr: SocketAddr = match socket_str.parse() {
            Ok(addr) => addr,
            Err(_) => {
                error!("Invalid socket address: {}", socket_str);
                return tokio::spawn(async {});
            }
        };

        // Start the Warp server
        let route = build_route(delegate_channel);
        let warp_shutdown = self.warp_shutdown.clone();
        let (addr, server) =
            warp::serve(route).bind_with_graceful_shutdown(socket_addr, async move {
                warp_shutdown.cancelled().await;
            });
        warn!("Starting Warp HTTP server at http://{}", addr);
        tokio::spawn(server)
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for WarpListenerAgent {
    async fn init(self: Arc<Self>, scope: &mut Scope) {
        debug!("Initializing HttpListenerAgent");
        let config = parse_server_config_from_scope(scope);
        self.warp_handle.lock().await.replace(self.start_server(
            config.address,
            config.port,
            config.delegate,
        ));
    }

    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
        self.scope.clone()
    }

    async fn stop(&self) {
        info!("Stopping HttpListenerAgent");
        if let Some(handle) = self.warp_handle.lock().await.take() {
            info!("Stopping warp server");
            self.warp_shutdown.cancel();
            if let Err(e) = handle.await {
                error!("Error stopping Warp server: {:?}", e);
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
impl AgentBehavior for WarpListenerAgent {
    async fn handle_message(&self, _msg: Message) -> bool {
        true
    }
}

impl Agent for WarpListenerAgent {}

pub struct WarpListenerFactory;

impl AgentFactory for WarpListenerFactory {
    fn create_agent(&self, name: &str, initial_scope: Scope) -> Arc<dyn Agent> {
        WarpListenerAgent::new(name, initial_scope)
    }
}
