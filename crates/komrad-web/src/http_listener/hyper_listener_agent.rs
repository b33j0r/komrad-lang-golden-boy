use crate::http_listener::config::ServerConfig;
use crate::request;
use crate::request::KomradRequest;
use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::combinators::BoxBody;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use komrad_ast::agent::{Agent, AgentLifecycle};
use komrad_ast::prelude::{AgentBehavior, Channel, ChannelListener, Message, Scope};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::select;
use tokio_util::sync::CancellationToken;

pub struct HyperListenerAgent {
    name: String,
    listener: Arc<ChannelListener>,
    channel: Channel,
    config: ServerConfig,
}

impl HyperListenerAgent {
    pub fn new(name: String, scope: &Scope) -> Self {
        let (channel, listener) = Channel::new(32);
        let config = crate::http_listener::config::parse_server_config_from_scope(scope);
        HyperListenerAgent {
            name,
            listener: Arc::new(listener),
            channel,
            config,
        }
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for HyperListenerAgent {
    async fn init(self: Arc<Self>, scope: &mut Scope) {
        // Initialize the agent
        let config = self.config.clone();
        tokio::spawn(async move {
            if let Err(err) = run_server(config).await {
                eprintln!("Error running server: {:?}", err);
            }
        });
    }

    async fn get_scope(&self) -> Arc<tokio::sync::Mutex<Scope>> {
        Arc::new(tokio::sync::Mutex::new(Scope::new()))
    }

    fn channel(&self) -> &Channel {
        &self.channel
    }

    fn listener(&self) -> Arc<ChannelListener> {
        self.listener.clone()
    }
}

async fn run_server(
    server_config: ServerConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = format!("{}:{}", server_config.address, server_config.port)
        .parse::<SocketAddr>()
        .expect("Invalid address");
    let graceful = CancellationToken::new();
    let listener = TcpListener::bind(addr).await?;

    // We start a loop to continuously accept incoming connections
    loop {
        select! {
            accept_result = listener.accept() => {
                if let Ok((stream, _)) = accept_result {
                    // Use an adapter to access something implementing `tokio::io` traits as if they implement
                    // `hyper::rt` IO traits.
                    let io = TokioIo::new(stream);

                    // Spawn a tokio task to serve multiple connections concurrently
                    tokio::task::spawn(async move {
                        // Finally, we bind the incoming connection to our `hello` service
                        if let Err(err) = http1::Builder::new()
                            // `service_fn` converts our function in a `Service`
                            .serve_connection(io, service_fn(echo))
                            .await
                        {
                            eprintln!("Error serving connection: {:?}", err);
                        }
                    });
                } else {
                    eprintln!("Failed to accept connection: {:?}", accept_result);
                }
            }
            // Catch ctrl-c to trigger graceful shutdown
            _ = tokio::signal::ctrl_c() => {
                println!("Ctrl-C received, shutting down...");
                graceful.cancel();
            }
            // If the graceful shutdown token is triggered, we break the loop
            _ = graceful.cancelled() => {
                println!("Shutting down gracefully...");
                break;
            }
        }
    }
    Ok(())
}

async fn echo(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let komrad_req = KomradRequest::from_request(req).await;
    let path = komrad_req.path.join("/");

    match komrad_req.method.as_str() {
        "GET" => {
            // Check for a delegate channel
            if komrad_req.delegate.is_none() {
                return Ok(Response::new(request::full("No delegate channel found")));
            }
            Ok(Response::new(request::full(format!("Hello, {}!", path))))
        }
        "POST" => {
            //
            Ok(Response::new(request::full(komrad_req.body)))
        }
        // Return 404 Not Found for other routes.
        _ => {
            let mut not_found = Response::new(request::empty());
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

#[async_trait::async_trait]
impl AgentBehavior for HyperListenerAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        return true;
    }
}

impl Agent for HyperListenerAgent {}

pub struct HyperListenerFactory;

impl komrad_ast::agent::AgentFactory for HyperListenerFactory {
    fn create_agent(&self, name: &str, initial_scope: Scope) -> Arc<dyn Agent> {
        Arc::new(HyperListenerAgent::new(name.to_string(), &initial_scope))
    }
}
