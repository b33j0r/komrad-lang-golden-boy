use async_trait::async_trait;
use komrad_agent::execute::Execute;
use komrad_agent::scope::Scope;
use komrad_agent::{Agent, AgentBehavior, AgentFactory, AgentLifecycle};
use komrad_ast::prelude::{Block, Channel, ChannelListener, Message, Number, Value};
use std::sync::Arc;
use tokio::select;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use warp::Filter;

pub struct HttpListener {
    _name: String,
    scope: Arc<Mutex<Scope>>,
    running: Mutex<bool>,
    channel: Channel,
    listener: Mutex<ChannelListener>,
}

impl HttpListener {
    pub fn new(name: &str, initial_scope: Scope) -> Self {
        let (chan, listener) = Channel::new(32);
        HttpListener {
            _name: name.to_string(),
            scope: Arc::new(Mutex::new(initial_scope)),
            running: Mutex::new(true),
            channel: chan,
            listener: Mutex::new(listener),
        }
    }
}

#[async_trait]
impl AgentLifecycle for HttpListener {
    async fn init(self: Arc<Self>, scope: &mut Scope) {
        warn!("Initializing HTTP server");
        let address = scope
            .get("host")
            .await
            .unwrap_or(Value::String("localhost".to_string()));
        let port = scope
            .get("port")
            .await
            .unwrap_or(Value::Number(Number::UInt(3033)));
        let delegate = scope.get("delegate").await.unwrap_or(Value::Empty);
        self.start_server(address, port, delegate).await;
    }

    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
        self.scope.clone()
    }

    async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
        // Here you would also stop the HTTP server
        // For example, if using hyper, you would call server.shutdown().await
        warn!("HTTP server stopped");
    }

    fn is_running(&self) -> bool {
        match self.running.try_lock() {
            Ok(guard) => *guard,
            Err(_) => false,
        }
    }

    fn channel(&self) -> &Channel {
        &self.channel
    }

    fn listener(&self) -> &Mutex<ChannelListener> {
        &self.listener
    }
}

#[async_trait]
impl AgentBehavior for HttpListener {
    async fn actor_loop(self: Arc<Self>, _chan: Channel) {
        {
            let scope = self.clone().get_scope().await;
            let mut scope = scope.lock().await;
            error!("HTTP scope {:}", scope);
            self.clone().init(&mut scope).await;
            error!("HTTP server started");
        }
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
        }
    }

    async fn handle_message(&self, _msg: Message) -> bool {
        true
    }
}

impl Agent for HttpListener {}

impl HttpListener {
    async fn start_server(&self, address: Value, port: Value, delegate: Value) {
        let delegate = if let Value::Channel(chan) = delegate {
            info!("Using delegate channel: {:?}", chan);
            chan
        } else {
            error!("Invalid delegate value: {:?}", delegate);
            warn!("Requests will go to a dead-letter channel");
            Channel::new(32).0
        };
        let address = if let Value::String(addr) = address {
            addr
        } else {
            "localhost".to_string()
        };

        let port = if let Value::Number(Number::UInt(p)) = port {
            p
        } else {
            3033
        };

        let delegate = delegate; // You can process the 'delegate' value if needed

        // Define a simple Warp filter
        let route = warp::any().map(|| warp::reply::html("Hello, World!"));

        // Combine the address and port
        let socket_addr = format!("{}:{}", address, port);
        info!("Starting HTTP server at {}", socket_addr);
        let socket_addr = socket_addr.parse::<std::net::SocketAddr>();

        match socket_addr {
            Ok(addr) => {
                // Start the Warp server
                tokio::spawn(async move { warp::serve(route).run(addr).await });
            }
            Err(e) => {
                error!("Invalid address or port: {:?}", e);
            }
        }
    }
}

pub struct HttpListenerFactory;

impl AgentFactory for HttpListenerFactory {
    fn create_agent(&self, name: &str, initial_scope: Scope) -> Arc<dyn Agent> {
        Arc::new(HttpListener::new(name, initial_scope))
    }
}
