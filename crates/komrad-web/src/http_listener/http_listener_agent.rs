use crate::http_listener::http_response_agent::HttpResponseAgent;
use komrad_agent::scope::Scope;
use komrad_agent::{Agent, AgentBehavior, AgentFactory, AgentLifecycle};
use komrad_ast::prelude::{Channel, ChannelListener, Message, Number, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use warp::http::Response;
use warp::{http, Filter, Rejection, Reply};

/// Converts the final list-based response (expected [status, headers, cookies, body])
/// into a Warp `Response<Body>`.
fn warp_response_from_komrad(terms: &[Value]) -> warp::reply::Response {
    if terms.is_empty() {
        return warp::reply::with_status("Empty response", http::StatusCode::INTERNAL_SERVER_ERROR)
            .into_response();
    }
    match &terms[0] {
        // Expect 4-element list: [status, headers, cookies, body]
        Value::List(list_of_4) if list_of_4.len() == 4 => {
            // 1) Extract status code
            let status_code = if let Value::Number(n) = &list_of_4[0] {
                let raw = match n {
                    Number::Int(i) => *i,
                    Number::UInt(u) => *u as i64,
                    Number::Float(_) => 200, // fallback
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

            // 2) Extract headers
            let mut header_map = http::HeaderMap::new();
            if let Value::List(header_list) = &list_of_4[1] {
                for hpair in header_list {
                    if let Value::List(pair) = hpair {
                        if pair.len() == 2 {
                            if let (Value::String(k), Value::String(v)) = (&pair[0], &pair[1]) {
                                header_map.insert(
                                    http::header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                                    v.parse().unwrap(),
                                );
                            }
                        }
                    }
                }
            }

            // 3) Extract cookies => "Set-Cookie"
            if let Value::List(cookie_list) = &list_of_4[2] {
                for cpair in cookie_list {
                    if let Value::List(pair) = cpair {
                        if pair.len() == 2 {
                            if let (Value::String(k), Value::String(v)) = (&pair[0], &pair[1]) {
                                let ck_line = format!("{}={}", k, v);
                                header_map
                                    .append(http::header::SET_COOKIE, ck_line.parse().unwrap());
                            }
                        }
                    }
                }
            }

            // 4) Body
            let body_str = match &list_of_4[3] {
                Value::Bytes(b) => String::from_utf8_lossy(b).to_string(),
                Value::String(s) => s.clone(),
                _ => "".to_string(),
            };

            // Build final response
            let mut response = Response::new(body_str.into());
            *response.status_mut() =
                http::StatusCode::from_u16(status_code).unwrap_or(http::StatusCode::OK);

            // Insert headers
            let resp_headers = response.headers_mut();
            for (k, v) in header_map {
                resp_headers.insert(k.unwrap(), v);
            }
            response
        }
        // Fallback if only a single element: e.g. "Empty", "Unsupported", etc.
        other => {
            let text = match other {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Embedded(e) => e.text.clone(),
                _ => format!("Unsupported response type: {:?}", other),
            };
            warp::reply::html(text).into_response()
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
                    info!(
                        "Received {} request for {}, forwarding to delegate channel {}",
                        method,
                        path.as_str(),
                        delegate.uuid()
                    );

                    // Split path into segments
                    let path_segments: Vec<Value> = path
                        .as_str()
                        .split('/')
                        .filter(|s| !s.is_empty())
                        .map(|s| Value::String(s.to_string()))
                        .collect();

                    // 1) The final, ultimate channel we want to read from:
                    //    We'll read ephemeral agent's final `[status, headers, cookies, body]`
                    let (final_tx, mut final_rx) = Channel::new(1);

                    // 2) Create ephemeral response agent with `reply_to = final_tx`.
                    let response_agent = HttpResponseAgent::new("Response", Some(final_tx.clone()));
                    let ephemeral_chan = response_agent.spawn();

                    // 3) Instead of giving the dynamic agent the warp route's reply channel,
                    //    we give it `ephemeral_chan` so the dynamic agent's fallback
                    //    goes to ephemeral agent (which can ignore it).
                    //    -> ephemeral agent alone sends final message to `final_tx`.
                    let mut message_terms = vec![
                        Value::Word("http".into()),
                        Value::Channel(ephemeral_chan), // `_response`
                        Value::Word(method.to_string().to_uppercase()), // GET, POST, etc.
                    ];
                    // Then path segments
                    message_terms.extend(path_segments);

                    // The final message has `Some(final_tx)` in the ephemeral agent, not here
                    let msg_to_delegate = Message::new(message_terms, None);
                    if let Err(e) = delegate.send(msg_to_delegate).await {
                        error!("Failed to send message to delegate: {:?}", e);
                        return Ok::<_, Rejection>(
                            warp::reply::with_status(
                                "Error",
                                http::StatusCode::INTERNAL_SERVER_ERROR,
                            )
                            .into_response(),
                        );
                    }

                    // 4) Wait for ephemeral agent's final
                    match final_rx.recv().await {
                        Ok(final_msg) => {
                            let resp = warp_response_from_komrad(final_msg.terms());
                            Ok::<_, Rejection>(resp)
                        }
                        Err(e) => {
                            error!("Failed final reply from ephemeral agent: {:?}", e);
                            Ok::<_, Rejection>(
                                warp::reply::with_status(
                                    "Error receiving final reply",
                                    http::StatusCode::INTERNAL_SERVER_ERROR,
                                )
                                .into_response(),
                            )
                        }
                    }
                } else {
                    Ok::<_, Rejection>(
                        warp::reply::with_status(
                            "No delegate was found to return a response",
                            http::StatusCode::INTERNAL_SERVER_ERROR,
                        )
                        .into_response(),
                    )
                }
            }
        },
    )
}

/// HTTP Listener Agent ...
pub struct HttpListenerAgent {
    name: String,
    scope: Arc<Mutex<Scope>>,
    channel: Channel,
    listener: Arc<ChannelListener>,
    warp_handle: Mutex<Option<JoinHandle<()>>>,
    warp_shutdown: CancellationToken,
}

impl HttpListenerAgent {
    /// ...
    pub fn new(name: &str, initial_scope: Scope) -> Arc<Self> {
        error!("Creating HttpListenerAgent");
        let (channel, listener) = Channel::new(32);
        Arc::new(Self {
            name: name.to_string(),
            scope: Arc::new(Mutex::new(initial_scope)),
            channel,
            listener: Arc::new(listener),
            warp_handle: Mutex::new(None),
            warp_shutdown: CancellationToken::new(),
        })
    }

    fn start_server(&self, address: Value, port: Value, delegate: Value) -> JoinHandle<()> {
        error!("Starting Warp HTTP server");
        let addr_str = match address {
            Value::String(s) => s,
            _ => "0.0.0.0".to_string(),
        };
        let port_num = match port {
            Value::Number(Number::UInt(n)) => n,
            _ => 3033,
        };

        let delegate_channel = match delegate {
            Value::Channel(c) => Some(c),
            _ => None,
        };

        let socket_str = format!("{}:{}", addr_str, port_num);
        let socket_addr: SocketAddr = match socket_str.parse() {
            Ok(addr) => addr,
            Err(_) => {
                error!("Invalid socket address: {}", socket_str);
                return tokio::spawn(async {});
            }
        };

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
impl AgentLifecycle for HttpListenerAgent {
    async fn init(self: Arc<Self>, scope: &mut Scope) {
        info!("Initializing HttpListenerAgent");
        let (address, port, delegate) = {
            let address = scope
                .get("host")
                .unwrap_or(Value::String("0.0.0.0".to_string()));
            let port = scope
                .get("port")
                .unwrap_or(Value::Number(Number::UInt(3033)));
            let delegate = scope.get("delegate").unwrap_or(Value::Empty);
            (address, port, delegate)
        };
        self.warp_handle
            .lock()
            .await
            .replace(self.start_server(address, port, delegate));
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
impl AgentBehavior for HttpListenerAgent {
    async fn handle_message(&self, _msg: Message) -> bool {
        true
    }
}

impl Agent for HttpListenerAgent {}

pub struct HttpListenerFactory;

impl AgentFactory for HttpListenerFactory {
    fn create_agent(&self, name: &str, initial_scope: Scope) -> Arc<dyn Agent> {
        HttpListenerAgent::new(name, initial_scope)
    }
}
