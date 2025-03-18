use async_trait::async_trait;
use komrad_agent::{Agent, AgentBehavior, AgentLifecycle};
use komrad_ast::prelude::{Channel, ChannelListener, ControlMessage, Message, Number, Value};
use komrad_ast::scope::Scope;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

pub enum CacheControl {
    NoCache,
    NoStore,
    Private,
    Public,
    MaxAge(u32),
    MustRevalidate,
    ProxyRevalidate,
    Immutable,
}

pub trait ResponseMetadataProtocol {
    fn set_status(&self, status: u16);
    fn set_cookie(&self, name: String, value: String);
    fn set_content_type(&self, content_type: String);
    fn set_header(&self, name: String, value: String);
    fn set_cache_control(&self, cache_control: CacheControl);
}

pub trait ResponseWriteProtocol {
    fn write_value(&self, value: Value);
}

pub trait ResponseFinalizerProtocol {
    fn finish(&self);
    fn redirect(&self, location: String);
    fn text(&self, body: String);
    fn html(&self, body: String);
    fn json(&self, body: String);
    fn binary(&self, body: Vec<u8>);
    fn websocket(&self, client: Value);
    fn error(&self, message: &str);
}

// Combines them
pub trait ResponseProtocol:
    ResponseMetadataProtocol + ResponseWriteProtocol + ResponseFinalizerProtocol
{
}

// ---------------------------------------------------------
// 2) Internal state to track status, headers, cookies, body
// ---------------------------------------------------------

#[derive(Debug, Clone)]
pub struct HttpResponseState {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub cookies: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub finished: bool,

    // NEW: If set, we know this response is a websocket upgrade, not a normal HTTP response.
    pub websocket_delegate: Option<Value>,
}

impl Default for HttpResponseState {
    fn default() -> Self {
        HttpResponseState {
            status: 200,
            headers: HashMap::new(),
            cookies: vec![],
            body: vec![],
            finished: false,
            websocket_delegate: None,
        }
    }
}

// ---------------------------------------------------------
// 3) The ephemeral agent that glues it all together
// ---------------------------------------------------------

#[derive(Debug)]
pub struct HttpResponseAgent {
    name: String,
    channel: Channel,
    listener: Arc<ChannelListener>,

    // Where we send the final message once we're done
    reply_to: Option<Channel>,

    // All actual response data is in here.
    state: Arc<Mutex<HttpResponseState>>,
}

impl HttpResponseAgent {
    /// Creates a new ephemeral HTTPResponseAgent.
    pub fn new(name: &str, reply_to: Option<Channel>) -> Arc<Self> {
        let (ch, listener) = Channel::new(32);
        Arc::new(Self {
            name: name.to_string(),
            channel: ch,
            listener: Arc::new(listener),
            reply_to,
            state: Arc::new(Mutex::new(HttpResponseState::default())),
        })
    }

    /// Called whenever we’re “done.”
    /// If this is a websocket, we send [ "websocket", <client> ].
    /// Otherwise, we send [status, headers, cookies, body].
    fn send_final(&self) {
        let mut st = self.state.lock().unwrap();
        if st.finished {
            return;
        }
        st.finished = true;

        if let Some(ref reply_chan) = self.reply_to {
            // Check if this is a websocket response.

            // Normal HTTP response
            let status_val = Value::Number(Number::UInt(st.status as u64));
            let headers_val = {
                let mut hv = vec![];
                for (k, v) in &st.headers {
                    hv.push(Value::List(vec![
                        Value::String(k.clone()),
                        Value::String(v.clone()),
                    ]));
                }
                Value::List(hv)
            };
            let cookies_val = {
                let mut cv = vec![];
                for (n, val) in &st.cookies {
                    cv.push(Value::List(vec![
                        Value::String(n.clone()),
                        Value::String(val.clone()),
                    ]));
                }
                Value::List(cv)
            };
            let body_val = Value::Bytes(st.body.clone());

            let websocket_delegate = if st.websocket_delegate.is_some() {
                st.websocket_delegate.clone().unwrap()
            } else {
                Value::Empty
            };

            let all = Value::List(vec![
                status_val,
                headers_val,
                cookies_val,
                body_val,
                websocket_delegate,
            ]);
            let msg = Message::new(vec![all], None);
            let _ = futures::executor::block_on(reply_chan.send(msg));
        }

        // Stop this agent’s loop
        let _ = futures::executor::block_on(self.channel.control(ControlMessage::Stop));
    }

    fn set_body_and_finish(&self, content_type: &str, body: Vec<u8>) {
        let mut st = self.state.lock().unwrap();
        st.headers
            .insert("Content-Type".to_string(), content_type.to_string());
        st.body = body;
        drop(st);
        self.send_final();
    }
}

// ---------------------------------------------------------
// 4) Implementation of the "Response" traits
// ---------------------------------------------------------

impl ResponseMetadataProtocol for HttpResponseAgent {
    fn set_status(&self, status: u16) {
        let mut st = self.state.lock().unwrap();
        st.status = status;
    }

    fn set_cookie(&self, name: String, value: String) {
        let mut st = self.state.lock().unwrap();
        st.cookies.push((name, value));
    }

    fn set_content_type(&self, content_type: String) {
        let mut st = self.state.lock().unwrap();
        st.headers.insert("Content-Type".to_string(), content_type);
    }

    fn set_header(&self, name: String, value: String) {
        let mut st = self.state.lock().unwrap();
        st.headers.insert(name, value);
    }

    fn set_cache_control(&self, cc: CacheControl) {
        let cc_str = match cc {
            CacheControl::NoCache => "no-cache".to_string(),
            CacheControl::NoStore => "no-store".to_string(),
            CacheControl::Private => "private".to_string(),
            CacheControl::Public => "public".to_string(),
            CacheControl::MaxAge(n) => format!("max-age={}", n),
            CacheControl::MustRevalidate => "must-revalidate".to_string(),
            CacheControl::ProxyRevalidate => "proxy-revalidate".to_string(),
            CacheControl::Immutable => "immutable".to_string(),
        };
        let mut st = self.state.lock().unwrap();
        st.headers.insert("Cache-Control".to_string(), cc_str);
    }
}

impl ResponseWriteProtocol for HttpResponseAgent {
    fn write_value(&self, value: Value) {
        let mut st = self.state.lock().unwrap();
        match value {
            Value::Bytes(b) => st.body.extend_from_slice(&b),
            Value::String(s) => st.body.extend_from_slice(s.as_bytes()),
            Value::Number(n) => st.body.extend_from_slice(n.to_string().as_bytes()),
            Value::Boolean(b) => st
                .body
                .extend_from_slice(if b { b"true" } else { b"false" }),
            Value::List(lst) => {
                for v in lst {
                    st.body
                        .extend_from_slice(format!("{} ", v.to_string()).as_bytes());
                }
            }
            other => {
                st.body.extend_from_slice(format!("{:?}", other).as_bytes());
            }
        }
    }
}

impl ResponseFinalizerProtocol for HttpResponseAgent {
    fn finish(&self) {
        self.send_final();
    }

    fn redirect(&self, location: String) {
        let mut st = self.state.lock().unwrap();
        st.status = 302;
        st.headers.insert("Location".to_string(), location);
        drop(st);
        self.send_final();
    }

    fn text(&self, body: String) {
        self.set_body_and_finish("text/plain", body.into_bytes());
    }

    fn html(&self, body: String) {
        self.set_body_and_finish("text/html", body.into_bytes());
    }

    fn json(&self, body: String) {
        self.set_body_and_finish("application/json", body.into_bytes());
    }

    fn binary(&self, body: Vec<u8>) {
        self.set_body_and_finish("application/octet-stream", body);
    }

    fn websocket(&self, client: Value) {
        let mut st = self.state.lock().unwrap();
        st.websocket_delegate = Some(client);
        self.set_status(101);
        self.set_header("Upgrade".to_string(), "websocket".to_string());
        self.set_header("Connection".to_string(), "Upgrade".to_string());
        drop(st);
        self.send_final();
    }

    fn error(&self, message: &str) {
        let mut st = self.state.lock().unwrap();
        st.status = 500;
        st.body.extend_from_slice(message.as_bytes());
        drop(st);
        self.send_final();
    }
}

// Combine them
impl ResponseProtocol for HttpResponseAgent {}

// ---------------------------------------------------------
// 5) Implement AgentLifecycle + AgentBehavior
// ---------------------------------------------------------

#[async_trait]
impl AgentLifecycle for HttpResponseAgent {
    async fn init(self: Arc<Self>, _scope: &mut Scope) {
        info!("HttpResponseAgent init for {}", self.name);
    }

    async fn get_scope(&self) -> Arc<tokio::sync::Mutex<Scope>> {
        Arc::new(tokio::sync::Mutex::new(Scope::new()))
    }

    async fn stop(&self) {
        debug!("HttpResponseAgent stopping for {}", self.name);
        // If not finished, finalize now.
        self.send_final();
        self.stop_in_scope().await;
    }

    fn channel(&self) -> &Channel {
        &self.channel
    }

    fn listener(&self) -> Arc<ChannelListener> {
        self.listener.clone()
    }
}

#[async_trait]
impl AgentBehavior for HttpResponseAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        let terms = msg.terms();
        if terms.is_empty() {
            return true;
        }
        let Some(Value::Word(action)) = terms.get(0) else {
            return true;
        };

        match action.as_str() {
            "set-cookie" => {
                if terms.len() >= 3 {
                    let name = to_string(&terms[1]);
                    let val = to_string(&terms[2]);
                    self.set_cookie(name, val);
                }
            }
            "set-status" => {
                if let Some(Value::Number(n)) = terms.get(1) {
                    let status = match n {
                        Number::Int(i) => *i as u16,
                        Number::UInt(u) => *u as u16,
                        Number::Float(f) => *f as u16,
                    };
                    self.set_status(status);
                }
            }
            "set-header" => {
                if terms.len() >= 3 {
                    let key = to_string(&terms[1]);
                    let val = to_string(&terms[2]);
                    self.set_header(key, val);
                }
            }
            "set-content-type" => {
                if let Some(val) = terms.get(1) {
                    self.set_content_type(to_string(val));
                }
            }
            "set-content-disposition" => {
                if let Some(val) = terms.get(1) {
                    self.set_header("Content-Disposition".to_string(), to_string(val));
                }
            }
            "set-cache-control" => {
                // ...
                // (unchanged logic for your cache-control handling)
            }
            "write-value" | "write" => {
                if let Some(val) = terms.get(1) {
                    self.write_value(val.clone());
                }
            }
            "finish" => {
                self.finish();
                return false;
            }
            "redirect" => {
                if let Some(val) = terms.get(1) {
                    self.redirect(to_string(val));
                }
                return false;
            }
            "text" => {
                if let Some(val) = terms.get(1) {
                    self.text(to_string(val));
                }
                return false;
            }
            "html" => {
                if let Some(val) = terms.get(1) {
                    self.html(to_string(val));
                }
                return false;
            }
            "json" => {
                if let Some(val) = terms.get(1) {
                    self.json(to_string(val));
                }
                return false;
            }
            "binary" => {
                if let Some(val) = terms.get(1) {
                    match val {
                        Value::Bytes(bv) => self.binary(bv.clone()),
                        other => self.binary(other.to_string().into_bytes()),
                    }
                }
                return false;
            }
            "websocket" => {
                if let Some(ws_client) = terms.get(1) {
                    match ws_client {
                        delegate_value @ Value::Channel(_delegate) => {
                            error!("Websocket client: {:?}", delegate_value);
                            self.websocket(delegate_value.clone());
                        }
                        other => {
                            error!("Invalid websocket client (expected channel): {:?}", other);
                            self.error("Invalid websocket client");
                        }
                    }
                }
                return false;
            }
            "error" => {
                if let Some(val) = terms.get(1) {
                    self.error(val.to_string().as_str());
                }
                return false;
            }
            other => {
                error!("Unrecognized response command: {}", other);
            }
        }

        true
    }
}

impl Agent for HttpResponseAgent {}

// Helper to turn a Value into a String.
fn to_string(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Word(w) => w.clone(),
        Value::Embedded(e) => e.text().clone(),
        other => format!("{:?}", other),
    }
}
