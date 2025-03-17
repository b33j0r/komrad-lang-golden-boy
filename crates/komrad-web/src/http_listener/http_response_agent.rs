use async_trait::async_trait;
use komrad_agent::{Agent, AgentBehavior, AgentLifecycle};
use komrad_ast::prelude::{Channel, ChannelListener, ControlMessage, Message, Number, Value};
use komrad_ast::scope::Scope;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

// ---------------------------------------------------------
// 1) The “internal” traits your system had defined
// ---------------------------------------------------------

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
}

#[allow(unused)]
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
}

impl Default for HttpResponseState {
    fn default() -> Self {
        HttpResponseState {
            status: 200,
            headers: HashMap::new(),
            cookies: vec![],
            body: vec![],
            finished: false,
        }
    }
}

// ---------------------------------------------------------
// 3) The ephemeral agent that glues it all together
// ---------------------------------------------------------

/// The `HttpResponseAgent` is a Komrad agent that listens on its channel for messages
/// like `[response set-cookie "k" "v"]`, `[response html "<html>...</html>"]`, etc.
/// Once it receives a final call (finish/html/text/etc.), it sends a final
/// `[status, headers, cookies, body]` message back to `reply_to`, then stops.
#[derive(Debug)]
pub struct HttpResponseAgent {
    name: String,
    channel: Channel,
    listener: Arc<ChannelListener>,

    // This is ephemeral for a single request in your design,
    // so we store the "final callback" to the original agent that created us.
    // That original agent (likely the HttpListenerAgent) is waiting for our final answer.
    reply_to: Option<Channel>,

    // All actual response data is in here.
    state: Arc<Mutex<HttpResponseState>>,
}

impl HttpResponseAgent {
    /// Creates a new ephemeral HTTPResponseAgent.
    /// Usually you pass in the `reply_to` that you want to eventually send
    /// `[status, headers, cookies, body]` back to.
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

    /// Helper: Send final `[statusVal, headersList, cookiesList, bodyBytes]` message.
    /// Then cause the agent to stop. (We do that by calling `self.channel.control(Stop)`,
    /// or by returning `false` in handle_message.)
    fn send_final(&self) {
        let mut st = self.state.lock().unwrap();

        // If already finished, do nothing.
        if st.finished {
            return;
        }
        st.finished = true;

        // If there is someone to reply to, send them a final Value::List
        if let Some(ref reply_chan) = self.reply_to {
            let status_val = Value::Number(Number::UInt(st.status as u64));

            // Convert headers to e.g. `List( [ ["Content-Type", "text/plain"], ... ] )`
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

            // Convert cookies likewise
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

            // Body is a Vec<u8>
            let body_val = Value::Bytes(st.body.clone());

            let all = Value::List(vec![status_val, headers_val, cookies_val, body_val]);

            // Wrap it in a message
            let msg = Message::new(vec![all], None);
            let _ = futures::executor::block_on(reply_chan.send(msg));
        }

        // Either kill ourselves or just mark done. We'll do a control(Stop).
        // That will cause our main loop to exit.
        let _ = futures::executor::block_on(self.channel.control(ControlMessage::Stop));
    }

    /// Helper to unify “set body bytes + content-type + finish”.
    fn set_body_and_finish(&self, content_type: &str, body: Vec<u8>) {
        {
            let mut st = self.state.lock().unwrap();
            st.headers
                .insert("Content-Type".to_string(), content_type.to_string());
            st.body = body;
        }
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
                // naive: join them with spaces
                for v in lst {
                    st.body
                        .extend_from_slice(format!("{} ", v.to_string()).as_bytes());
                }
            }
            other => {
                // fallback
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
        {
            let mut st = self.state.lock().unwrap();
            st.status = 302;
            st.headers.insert("Location".to_string(), location);
        }
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
        // Usually ephemeral agents might not need a real scope
        Arc::new(tokio::sync::Mutex::new(Scope::new()))
    }

    async fn stop(&self) {
        debug!("HttpResponseAgent stopping for {}", self.name);
        // If not finished, optionally finalize
        self.send_final();
        // Then call default behavior
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

        // The first term should be an action, e.g. "set-cookie" or "html" or "finish".
        let Some(Value::Word(action)) = terms.get(0) else {
            // Not a Word => ignore
            return true;
        };

        match action.as_str() {
            // For example: `[response set-cookie "session" "123"]`
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
                if let Some(val) = terms.get(1) {
                    let cc = match to_string(val).as_str() {
                        "no-cache" => CacheControl::NoCache,
                        "no-store" => CacheControl::NoStore,
                        "private" => CacheControl::Private,
                        "public" => CacheControl::Public,
                        "max-age" => {
                            if let Some(Value::Number(n)) = terms.get(2) {
                                let max_age = match n {
                                    Number::Int(i) => *i as u32,
                                    Number::UInt(u) => *u as u32,
                                    Number::Float(f) => *f as u32,
                                };
                                CacheControl::MaxAge(max_age)
                            } else {
                                CacheControl::MaxAge(0)
                            }
                        }
                        "must-revalidate" => CacheControl::MustRevalidate,
                        "proxy-revalidate" => CacheControl::ProxyRevalidate,
                        "immutable" => CacheControl::Immutable,
                        _ => {
                            warn!("Unrecognized cache control: {}", val);
                            return true;
                        }
                    };
                    self.set_cache_control(cc);
                }
            }
            // TODO: I think this was always meant to just be "write"?
            "write-value" | "write" => {
                // e.g. `[response write-value someValue]`
                if let Some(val) = terms.get(1) {
                    self.write_value(val.clone());
                }
            }
            "finish" => {
                self.finish();
                // Once finish is called, we are done.
                // Return false to exit the agent loop.
                return false;
            }
            "redirect" => {
                // `[response redirect "/somewhere"]`
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
                    // If it’s a Bytes, use them directly; otherwise fallback
                    match val {
                        Value::Bytes(bv) => self.binary(bv.clone()),
                        other => self.binary(other.to_string().into_bytes()),
                    }
                }
                return false;
            }
            other => {
                error!("Unrecognized response command: {}", other);
            }
        }

        // Keep listening for more instructions, unless the user told us to stop.
        true
    }
}

impl Agent for HttpResponseAgent {}

// A small helper for “turn any Value into string”
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
