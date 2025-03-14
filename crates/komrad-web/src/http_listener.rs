use komrad_agent::scope::Scope;
use komrad_agent::{Agent, AgentBehavior, AgentControl, AgentFactory, AgentLifecycle, AgentState};
use komrad_ast::prelude::{Channel, ChannelListener, Message, Number, RuntimeError, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::select;
use tokio::sync::{mpsc, watch, Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use warp::Filter;

/// HTTP Listener Agent responsible for handling HTTP requests via Warp.
pub struct HttpListenerAgent {
    name: String,
    scope: Arc<Mutex<Scope>>,
    channel: Channel,
    listener: Arc<ChannelListener>,

    warp_handle: Mutex<Option<JoinHandle<()>>>,
    warp_shutdown: CancellationToken,
}

impl HttpListenerAgent {
    /// Creates a new HTTP Listener Agent.
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

    /// Starts the Warp server in a background task.
    fn start_server(&self, address: Value, port: Value, delegate: Value) -> JoinHandle<()> {
        error!("Starting Warp HTTP server");
        let addr_str = match address {
            Value::String(s) => s,
            _ => "127.0.0.1".to_string(),
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

        let route = Self::build_route(delegate_channel);
        let warp_shutdown = self.warp_shutdown.clone();
        let (addr, server) =
            warp::serve(route).bind_with_graceful_shutdown(socket_addr, async move {
                warp_shutdown.cancelled().await;
            });
        warn!("Starting Warp HTTP server at http://{}", addr);
        tokio::spawn(server)
    }

    fn build_route(
        delegate: Option<Channel>,
    ) -> impl Filter<Extract = (warp::reply::Html<String>,), Error = warp::Rejection> + Clone {
        warp::path::full().and(warp::method()).and_then(
            move |path: warp::filters::path::FullPath, method: warp::http::Method| {
                let delegate = delegate.clone();
                async move {
                    if let Some(delegate) = &delegate {
                        info!(
                            "Received {} request for {}, forwarding to delegate channel {}:",
                            method,
                            path.as_str(),
                            delegate.uuid(),
                        );

                        let path_segments: Vec<Value> = path
                            .as_str()
                            .split('/')
                            .filter(|s| !s.is_empty())
                            .map(|s| Value::String(s.to_string()))
                            .collect();

                        let mut message_terms = vec![
                            Value::Word("http".to_string()),
                            Value::Word(method.to_string().to_uppercase()),
                        ];
                        message_terms.extend(path_segments);

                        let (reply_tx, mut reply_rx) = Channel::new(1);
                        let message = Message::new(message_terms, Some(reply_tx));

                        if let Err(e) = delegate.send(message).await {
                            error!("Failed to send message to delegate: {:?}", e);
                            return Ok::<_, warp::Rejection>(warp::reply::html(
                                "Error".to_string(),
                            ));
                        }

                        match reply_rx.recv().await {
                            Ok(reply_msg) => {
                                let body = reply_msg.terms().get(0);
                                let body = match body {
                                    Some(Value::String(s)) => s.clone(),
                                    Some(Value::Number(n)) => n.to_string(),
                                    Some(Value::Embedded(e)) => format!("{}", e.text),
                                    _ => "Unsupported response type".to_string(),
                                };
                                Ok::<_, warp::Rejection>(warp::reply::html(body))
                            }
                            Err(e) => {
                                error!(
                                    "Failed to receive reply from delegate {}: {:?}",
                                    delegate.uuid(),
                                    e
                                );
                                Ok::<_, warp::Rejection>(warp::reply::html("Error".to_string()))
                            }
                        }
                    } else {
                        Ok::<_, warp::Rejection>(warp::reply::html("Hello, World!".to_string()))
                    }
                }
            },
        )
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
