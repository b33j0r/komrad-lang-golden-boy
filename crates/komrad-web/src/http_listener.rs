use komrad_agent::scope::Scope;
use komrad_agent::{Agent, AgentBehavior, AgentFactory, AgentLifecycle};
use komrad_ast::prelude::{Channel, ChannelListener, Message, Number, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::select;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use warp::Filter;

/// A simple agent that can start a Warp server.
pub struct HttpListener {
    _name: String,
    scope: Arc<Mutex<Scope>>,
    running: Mutex<bool>,
    channel: Channel,
    listener: Mutex<ChannelListener>,
}

/// This trait holds the minimal server logic needed:
/// - Start the Warp server
/// - Build a route / filter
pub trait HttpListenerServer {
    /// Launch the server by spawning a Warp task.
    fn start_server(&self, address: Value, port: Value, delegate: Value);

    /// Build a route that just returns "Hello, World!"
    fn build_route(
        delegate: Option<Channel>,
    ) -> impl Filter<Extract = (warp::reply::Html<String>,), Error = warp::Rejection> + Clone;
}

impl HttpListener {
    /// Constructor. Accepts `initial_scope` if you want variables (like port, host).
    pub fn new(name: &str, initial_scope: Scope) -> Self {
        let (chan, lsn) = Channel::new(32);
        Self {
            _name: name.to_string(),
            scope: Arc::new(Mutex::new(initial_scope)),
            running: Mutex::new(true),
            channel: chan,
            listener: Mutex::new(lsn),
        }
    }
}

impl HttpListenerServer for HttpListener {
    fn start_server(&self, address: Value, port: Value, delegate: Value) {
        // Convert Komrad `Value` to concrete address, port
        let addr_str = match address {
            Value::String(s) => s,
            _ => "127.0.0.1".to_string(),
        };
        let port_num = match port {
            Value::Number(Number::UInt(n)) => n,
            _ => 3033,
        };

        // Get delegate channel if provided
        let delegate_channel = match delegate {
            Value::Channel(c) => Some(c),
            _ => None,
        };

        let socket_str = format!("{}:{}", addr_str, port_num);
        let Ok(socket_addr) = socket_str.parse::<SocketAddr>() else {
            error!("Invalid socket address: {socket_str}");
            return;
        };

        warn!("Starting Warp HTTP server at {socket_addr}");

        let route = Self::build_route(delegate_channel);
        // Spawn the Warp server in background without capturing `&self`
        tokio::spawn(async move {
            warp::serve(route).run(socket_addr).await;
        });
    }

    // Minimal route returning "Hello, World!" — no `&self` usage
    fn build_route(
        delegate: Option<Channel>,
    ) -> impl Filter<Extract = (warp::reply::Html<String>,), Error = warp::Rejection> + Clone {
        warp::any().and_then(move || {
            let delegate = delegate.clone();
            async move {
                if let Some(delegate) = &delegate {
                    info!(
                        "Received request, sending to delegate channel {}",
                        delegate.uuid()
                    );

                    let (reply_to, mut reply_rx) = Channel::new(1);
                    let message = Message::new(
                        vec![
                            Value::Word("http".to_string()),
                            Value::Word("GET".to_string()),
                            Value::String("/".to_string()),
                        ],
                        Some(reply_to),
                    );
                    if let Err(e) = delegate.send(message).await {
                        error!("Failed to send message to delegate: {:?}", e);
                        return Ok::<_, warp::Rejection>(warp::reply::html("Error".to_string()));
                    }
                    let response = reply_rx.recv().await;
                    let response_html = match response {
                        Ok(msg) => {
                            let response_body = msg.terms().get(0).unwrap_or(&Value::Empty);
                            match response_body {
                                Value::String(s) => s.clone(),
                                Value::Embedded(embedded_block) => {
                                    embedded_block.text().to_string()
                                }
                                Value::Empty => "<html><body>No response</body></html>".to_string(),
                                _ => {
                                    error!("Unexpected response type: {:?}", response_body);
                                    "<html><body>Error</body></html>".to_string()
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to receive message from delegate: {:?}", e);
                            "<html><body>Error</body></html>".to_string()
                        }
                    };

                    Ok::<_, warp::Rejection>(warp::reply::html(response_html))
                } else {
                    info!("Received request, no delegate channel");
                    Ok::<_, warp::Rejection>(warp::reply::html(
                        "<html><body>No delegate</body></html>".to_string(),
                    ))
                }
            }
        })
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for HttpListener {
    /// Called once before the main loop. We read `host`, `port`, and `delegate` from the scope,
    /// then start our Warp server.
    async fn init(self: Arc<Self>, scope: &mut Scope) {
        warn!("HttpListener init: reading scope & starting server...");

        let address = scope
            .get("host")
            .await
            .unwrap_or(Value::String("127.0.0.1".to_string()));
        let port = scope
            .get("port")
            .await
            .unwrap_or(Value::Number(Number::UInt(3033)));
        let delegate = scope.get("delegate").await.unwrap_or(Value::Empty);

        // Just start the server (non-async).
        self.start_server(address, port, delegate);
    }

    /// Return the scope so Komrad can store or retrieve variables later.
    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
        self.scope.clone()
    }

    /// Called if you want to stop the server gracefully.
    async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
        warn!("HttpListener stopping (TODO: graceful Warp shutdown).");
    }

    fn is_running(&self) -> bool {
        self.running.try_lock().map(|g| *g).unwrap_or(false)
    }

    fn channel(&self) -> &Channel {
        &self.channel
    }

    fn listener(&self) -> &Mutex<ChannelListener> {
        &self.listener
    }
}

#[async_trait::async_trait]
impl AgentBehavior for HttpListener {
    /// The main loop. Runs after `init()` completes.
    async fn actor_loop(self: Arc<Self>, _chan: Channel) {
        {
            let scope = self.get_scope().await;
            let mut scope = scope.lock().await;
            self.clone().init(&mut scope).await;
            info!("HttpListener: Warp server started in background.");
        }

        // Just loop until `stop()` is called or an error occurs.
        loop {
            select! {
                maybe_msg = Self::recv(&self) => {
                    match maybe_msg {
                        Ok(msg) => {
                            if !self.handle_message(msg).await {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                else => {
                    break;
                }
            }
            if !self.is_running() {
                break;
            }
        }

        warn!("HttpListener main loop exited.");
    }

    /// If you want special commands like "[shutdown]" => self.stop(), handle them here.
    async fn handle_message(&self, _msg: Message) -> bool {
        true
    }
}

/// This ensures we satisfy both Lifecycle + Behavior for Komrad
impl Agent for HttpListener {}

/// Factory so Komrad can create it dynamically with a scope
pub struct HttpListenerFactory;

impl AgentFactory for HttpListenerFactory {
    fn create_agent(&self, name: &str, initial_scope: Scope) -> Arc<dyn Agent> {
        Arc::new(HttpListener::new(name, initial_scope))
    }
}
