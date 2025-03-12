use async_trait::async_trait;
use komrad_agent::scope::Scope;
use komrad_agent::{Agent, AgentBehavior, AgentFactory, AgentLifecycle};
use komrad_ast::prelude::{Channel, ChannelListener, Message, Number, Value};
use std::sync::Arc;
use tokio::select;
use tokio::sync::Mutex;
use tracing::{info, warn};

pub struct HttpListener {
    _name: String,
    scope: Arc<Mutex<Scope>>,
    running: Mutex<bool>,
    channel: Channel,
    listener: Mutex<ChannelListener>,
}

impl HttpListener {
    pub fn new(name: &str) -> Self {
        let (chan, listener) = Channel::new(32);
        HttpListener {
            _name: name.to_string(),
            scope: Arc::new(Mutex::new(Scope::new())),
            running: Mutex::new(true),
            channel: chan,
            listener: Mutex::new(listener),
        }
    }
}

#[async_trait]
impl AgentLifecycle for HttpListener {
    async fn init(self: Arc<Self>, scope: &mut Scope) {
        let address = scope
            .get("address")
            .await
            .unwrap_or(Value::String("localhost".to_string()));
        let port = scope
            .get("port")
            .await
            .unwrap_or(Value::Number(Number::UInt(3033)));
        warn!("HTTP server started at http://{}:{}", address, port);
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
    async fn handle_message(&self, _msg: Message) -> bool {
        true
    }

    async fn actor_loop(self: Arc<Self>, _chan: Channel) {
        {
            let scope = self.clone().get_scope().await;
            let mut scope = scope.lock().await;
            self.clone().init(&mut scope).await;
        }
        //
        // while self.is_running() {
        //     match Self::recv(&self).await {
        //         Ok(msg) => {
        //             if !Self::handle_message(&self, msg).await {
        //                 break;
        //             }
        //         }
        //         Err(_) => break,
        //     }
        // }
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
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                    info!("HTTP server is running");
                }
                else => {
                    break;
                }
            }
        }
    }
}

impl Agent for HttpListener {}

pub struct HttpListenerFactory;

impl AgentFactory for HttpListenerFactory {
    fn create_agent(&self, name: &str) -> Arc<dyn Agent> {
        Arc::new(HttpListener::new(name))
    }
}
