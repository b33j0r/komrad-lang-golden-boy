use async_trait::async_trait;
use komrad_agent::{Agent, AgentBehavior, AgentFactory, AgentLifecycle};
use komrad_ast::prelude::{Channel, ChannelListener, Message};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct HttpListener {
    address: String,
    port: u16,
    running: Mutex<bool>,
    channel: Channel,
    listener: Mutex<ChannelListener>,
}

impl HttpListener {
    pub fn new() -> Self {
        let (chan, listener) = Channel::new(32);
        HttpListener {
            address: "0.0.0.0".to_string(),
            port: 8080,
            running: Mutex::new(true),
            channel: chan,
            listener: Mutex::new(listener),
        }
    }
}

#[async_trait]
impl AgentLifecycle for HttpListener {
    async fn init(self: Arc<Self>) {
        println!("HTTP server started at {}:{}", self.address, self.port);
    }

    async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
        // Here you would also stop the HTTP server
        // For example, if using hyper, you would call server.shutdown().await
        println!("HTTP server stopped");
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
    async fn handle_message(&self, msg: Message) -> bool {
        true
    }
}

impl Agent for HttpListener {}

pub struct HttpListenerFactory;

impl AgentFactory for HttpListenerFactory {
    fn create_agent(&self, name: String) -> Arc<dyn Agent> {
        Arc::new(HttpListener::new())
    }
}
