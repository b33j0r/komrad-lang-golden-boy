use komrad_agent::scope::Scope;
use komrad_agent::{Agent, AgentBehavior, AgentControl, AgentFactory, AgentLifecycle, AgentState};
use komrad_ast::prelude::{Channel, ChannelListener, Message, Number, RuntimeError, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::select;
use tokio::sync::{mpsc, watch, Mutex};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use warp::Filter;

/// HTTP Listener Agent responsible for handling HTTP requests via Warp.
pub struct HttpListenerAgent {
    name: String,
    scope: Arc<Mutex<Scope>>,
    channel: Channel,
    listener: Mutex<ChannelListener>,

    control_tx: mpsc::Sender<AgentControl>,
    control_rx: Mutex<mpsc::Receiver<AgentControl>>,
    state_tx: watch::Sender<AgentState>,
    state_rx: watch::Receiver<AgentState>,

    warp_handle: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for HttpListenerAgent {
    fn drop(&mut self) {
        debug!("HttpListenerAgent {} is being dropped", self.name);
        self.control_tx.send(AgentControl::Stop);
    }
}

impl HttpListenerAgent {
    /// Creates a new HTTP Listener Agent.
    pub fn new(name: &str, initial_scope: Scope) -> Arc<Self> {
        let (channel, listener) = Channel::new(32);
        let (control_tx, control_rx) = mpsc::channel(8);
        let (state_tx, state_rx) = watch::channel(AgentState::Started);

        Arc::new(Self {
            name: name.to_string(),
            scope: Arc::new(Mutex::new(initial_scope)),
            channel,
            listener: Mutex::new(listener),
            control_tx,
            control_rx: Mutex::new(control_rx),
            state_tx,
            state_rx,
            warp_handle: Mutex::new(None),
        })
    }

    /// Starts the Warp server in a background task.
    fn start_server(&self, address: Value, port: Value, delegate: Value) -> JoinHandle<()> {
        info!("Starting Warp HTTP server");
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

        warn!("Starting Warp HTTP server at {}", socket_addr);

        let route = Self::build_route(delegate_channel);
        tokio::spawn(async move {
            warp::serve(route).run(socket_addr).await;
        })
    }

    fn build_route(
        delegate: Option<Channel>,
    ) -> impl Filter<Extract = (warp::reply::Html<String>,), Error = warp::Rejection> + Clone {
        warp::any().and_then(move || {
            let delegate = delegate.clone();
            async move {
                if let Some(delegate) = &delegate {
                    info!(
                        "Received request, forwarding to delegate channel {}:",
                        delegate.uuid(),
                    );
                    let (reply_tx, mut reply_rx) = Channel::new(1);
                    let message = Message::new(
                        vec![
                            Value::Word("http".to_string()),
                            Value::Word("GET".to_string()),
                            Value::String("/".to_string()),
                        ],
                        Some(reply_tx),
                    );

                    if let Err(e) = delegate.send(message).await {
                        error!("Failed to send message to delegate: {:?}", e);
                        return Ok::<_, warp::Rejection>(warp::reply::html("Error".to_string()));
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
        })
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for HttpListenerAgent {
    async fn init(self: Arc<Self>, scope: &mut Scope) -> Option<JoinHandle<()>> {
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
        Some(self.start_server(address, port, delegate))
    }

    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
        self.scope.clone()
    }

    async fn stop(&self) {
        let _ = self.control_tx.send(AgentControl::Stop).await;
    }

    fn channel(&self) -> &Channel {
        &self.channel
    }

    fn listener(&self) -> &Mutex<ChannelListener> {
        &self.listener
    }

    async fn recv_control(&self) -> Result<AgentControl, RuntimeError> {
        let mut rx = self.control_rx.lock().await;
        rx.recv().await.ok_or(RuntimeError::ReceiveControlError)
    }

    async fn notify_stopped(&self) {
        let _ = self.state_tx.send(AgentState::Stopped);
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
