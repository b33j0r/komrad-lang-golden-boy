// file: http_listener_agent.rs

use std::net::SocketAddr;
use std::sync::Arc;

use actix_web::{
    http::header::{HeaderMap, HeaderName, HeaderValue}, web, App, HttpRequest, HttpResponse,
    HttpServer,
    Responder,
};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use komrad_agent::{Agent, AgentBehavior, AgentFactory, AgentLifecycle};
use komrad_ast::prelude::{Channel, ChannelListener, Message, Number, Value};
use komrad_ast::scope::Scope;

use crate::http_listener::http_response_agent::HttpResponseAgent;

/// Convert `[ [status, headers, cookies, body] ]` into an Actix `HttpResponse`.
fn actix_response_from_komrad(terms: &[Value]) -> HttpResponse {
    if terms.is_empty() {
        return HttpResponse::InternalServerError()
            .content_type("text/plain")
            .body("Empty response");
    }
    match &terms[0] {
        // Expect [ status, headersList, cookiesList, bodyBytes ]
        Value::List(list_of_4) if list_of_4.len() == 4 => {
            let status_code = match &list_of_4[0] {
                Value::Number(Number::UInt(u)) => *u as u16,
                Value::Number(Number::Int(i)) if *i >= 100 && *i <= 599 => *i as u16,
                _ => 200,
            };
            let mut builder = HttpResponse::build(
                actix_web::http::StatusCode::from_u16(status_code)
                    .unwrap_or(actix_web::http::StatusCode::OK),
            );

            // headers
            if let Value::List(hlist) = &list_of_4[1] {
                for hpair in hlist {
                    if let Value::List(pair) = hpair {
                        if pair.len() == 2 {
                            if let (Value::String(k), Value::String(v)) = (&pair[0], &pair[1]) {
                                builder.append_header((k.as_str(), v.as_str()));
                            }
                        }
                    }
                }
            }
            // cookies
            if let Value::List(clist) = &list_of_4[2] {
                for cpair in clist {
                    if let Value::List(pair) = cpair {
                        if pair.len() == 2 {
                            if let (Value::String(k), Value::String(v)) = (&pair[0], &pair[1]) {
                                // Just set "Set-Cookie" manually:
                                let set_cookie_val = format!("{}={}", k, v);
                                builder.append_header(("Set-Cookie", set_cookie_val));
                            }
                        }
                    }
                }
            }
            // body
            let body_bytes = match &list_of_4[3] {
                Value::Bytes(b) => b.clone(),
                Value::String(s) => s.clone().into_bytes(),
                _ => Vec::new(),
            };
            builder.body(body_bytes)
        }
        // Fallback => treat it as a string
        other => {
            let text = match other {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                _ => format!("Unsupported response type: {:?}", other),
            };
            HttpResponse::Ok().content_type("text/plain").body(text)
        }
    }
}

/// A single Actix handler that catches all requests (except websockets).
/// We spawn an ephemeral `HttpResponseAgent`, forward a message to the `delegate`,
/// wait for the final `[status, headers, cookies, body]`, then build the Actix response.
async fn all_http_handler(
    req: HttpRequest,
    body: web::Bytes,
    delegate_opt: web::Data<Option<Channel>>,
) -> impl Responder {
    let delegate = match delegate_opt.get_ref() {
        Some(chan) => chan.clone(),
        None => {
            return HttpResponse::InternalServerError()
                .content_type("text/plain")
                .body("No delegate channel found");
        }
    };

    // Build path segments
    let path_str = req.path().trim_start_matches('/');
    let path_segments: Vec<Value> = if path_str.is_empty() {
        vec![]
    } else {
        path_str
            .split('/')
            .map(|s| Value::String(s.to_string()))
            .collect()
    };
    let method_str = req.method().as_str().to_uppercase();

    // Create ephemeral agent + final channel
    let (final_tx, final_rx) = Channel::new(1);
    let response_agent = HttpResponseAgent::new("Response", Some(final_tx));
    let ephemeral_chan = response_agent.spawn();

    // e.g.: [ "http", ephemeralChan, "GET", "foo", "bar" ]
    let mut msg_terms = vec![
        Value::Word("http".into()),
        Value::Channel(ephemeral_chan),
        Value::Word(method_str),
    ];
    msg_terms.extend(path_segments);

    // Optionally, if you want the request body, you can add it:
    //   msg_terms.push(Value::Bytes(body.to_vec()));

    let msg_to_delegate = Message::new(msg_terms, None);
    if let Err(e) = delegate.send(msg_to_delegate).await {
        error!("Failed sending message to delegate: {:?}", e);
        return HttpResponse::InternalServerError()
            .content_type("text/plain")
            .body("Error forwarding to delegate");
    }

    // Wait for ephemeral response
    match final_rx.recv().await {
        Ok(final_msg) => {
            // Convert to Actix HttpResponse
            actix_response_from_komrad(final_msg.terms())
        }
        Err(e) => {
            error!("Failed receiving final reply: {:?}", e);
            HttpResponse::InternalServerError()
                .content_type("text/plain")
                .body("Error receiving final reply from ephemeral")
        }
    }
}

/// The actual Komrad “listener agent”, but using Actix Web under the hood
pub struct ActixListenerAgent {
    name: String,
    scope: Arc<Mutex<Scope>>,
    channel: Channel,
    listener: Arc<ChannelListener>,

    actix_handle: Mutex<Option<JoinHandle<()>>>,
    actix_shutdown: CancellationToken,
}

impl ActixListenerAgent {
    pub fn new(name: &str, scope: Scope) -> Arc<Self> {
        let (ch, rx) = Channel::new(32);
        Arc::new(Self {
            name: name.to_string(),
            scope: Arc::new(Mutex::new(scope)),
            channel: ch,
            listener: Arc::new(rx),
            actix_handle: Mutex::new(None),
            actix_shutdown: CancellationToken::new(),
        })
    }

    fn start_server(
        self: &Arc<Self>,
        host: String,
        port: u16,
        delegate: Option<Channel>,
    ) -> JoinHandle<()> {
        let actix_shutdown = self.actix_shutdown.clone();

        // We spawn Actix in a tokio task so we can run it “in the background.”
        // We must call `actix_web::rt::System::run_in_tokio()` or an equivalent approach.
        tokio::spawn(async move {
            // create an actix System
            // Clone stuff to move into HttpServer
            let data_delegate = web::Data::new(delegate);
            let bind_addr = format!("{}:{}", host, port);

            // Build the server in an Actix closure
            let server = HttpServer::new(move || {
                App::new()
                    // store the Option<Channel> so our handler can see it
                    .app_data(data_delegate.clone())
                    // If we want a /ws route for websockets, see below
                    //.route("/ws", web::get().to(ws_handler))
                    // All other routes => fallback
                    .default_service(web::route().to(all_http_handler))
            })
            .bind(bind_addr)
            .expect("Could not bind to address")
            .shutdown_timeout(0) // so it stops quickly
            .run();

            // Listen for shutdown
            let mut srv_handle = tokio::spawn(server);

            // if our token is canceled, we stop the Actix system
            tokio::select! {
                _ = actix_shutdown.cancelled() => {
                    info!("Actix shutdown requested");
                },
                res = &mut srv_handle => {
                    error!("Server ended unexpectedly: {:?}", res);
                }
            }
        })
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for ActixListenerAgent {
    async fn init(self: Arc<Self>, scope: &mut Scope) {
        debug!("Initializing HttpListenerAgent with Actix Web");
        let host_val = scope.get("host").unwrap_or(Value::String("0.0.0.0".into()));
        let port_val = scope
            .get("port")
            .unwrap_or(Value::Number(Number::UInt(8080)));
        let delegate_val = scope.get("delegate").unwrap_or(Value::Empty);

        let host_str = match host_val {
            Value::String(ref s) => s.clone(),
            _ => "0.0.0.0".to_string(),
        };
        let port_u16 = match port_val {
            Value::Number(Number::UInt(u)) => u as u16,
            Value::Number(Number::Int(i)) if i > 0 => i as u16,
            _ => 8080,
        };
        let delegate_chan = if let Value::Channel(c) = delegate_val {
            Some(c)
        } else {
            None
        };

        let handle = self.start_server(host_str, port_u16, delegate_chan);
        self.actix_handle.lock().await.replace(handle);
    }

    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
        self.scope.clone()
    }

    async fn stop(&self) {
        info!("Stopping HttpListenerAgent");
        if let Some(h) = self.actix_handle.lock().await.take() {
            info!("Requesting Actix to shut down");
            self.actix_shutdown.cancel();
            let _ = h.await;
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
impl AgentBehavior for ActixListenerAgent {
    async fn handle_message(&self, _msg: Message) -> bool {
        // Typically do nothing
        true
    }
}

impl Agent for ActixListenerAgent {}

/// A simple factory if you need it
pub struct ActixListenerFactory;

impl AgentFactory for ActixListenerFactory {
    fn create_agent(&self, name: &str, scope: Scope) -> Arc<dyn Agent> {
        ActixListenerAgent::new(name, scope)
    }
}
