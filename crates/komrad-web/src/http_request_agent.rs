use bytes::Bytes;
use http::{HeaderMap, Request};
use http_body_util::BodyExt;
use komrad_agent::{Agent, AgentBehavior, AgentLifecycle};
use komrad_ast::prelude::{Channel, ChannelListener, Message, Value};
use komrad_ast::scope::Scope;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct RequestData {
    pub url: String,
    pub method: String,
    pub body: Bytes,
    pub headers: HeaderMap,
    pub params: HashMap<String, String>,
    pub cookies: HashMap<String, String>,
}

/// HttpRequestAgent encapsulates an HTTP request so the delegate can query its URL, method,
/// body, headers, query parameters, and cookies.
pub struct HttpRequestAgent {
    name: String,
    channel: Channel,
    listener: Arc<ChannelListener>,
    data: RequestData,
}

#[allow(dead_code)]
impl HttpRequestAgent {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> Vec<String> {
        // split the URL into non-empty path segments
        self.data
            .url
            .split('/')
            .filter_map(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .collect()
    }

    pub fn url(&self) -> &str {
        &self.data.url
    }

    pub fn method(&self) -> &str {
        &self.data.method
    }

    pub fn body(&self) -> &[u8] {
        &self.data.body
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.data.headers
    }

    pub fn params(&self) -> &HashMap<String, String> {
        &self.data.params
    }

    pub fn cookies(&self) -> &HashMap<String, String> {
        &self.data.cookies
    }

    /// Consumes a Hyper Request and extracts its details.
    pub async fn new(name: &str, req: Request<hyper::body::Incoming>) -> Arc<Self> {
        let method = req.method().to_string();
        let url = req.uri().to_string();
        let headers = req.headers().clone();
        // Parse query parameters from the URI.
        let params = Self::parse_query_params(req.uri().query());
        // Aggregate the entire request body.
        let body = req.collect().await.unwrap_or_default().to_bytes();
        // Parse cookies from the "cookie" header.
        let cookies = Self::parse_cookies(&headers);

        let data = RequestData {
            url,
            method,
            body,
            headers,
            params,
            cookies,
        };

        let (channel, listener) = Channel::new(32);
        Arc::new(Self {
            name: name.to_string(),
            channel,
            listener: Arc::new(listener),
            data,
        })
    }

    /// Parse query parameters from an optional query string.
    fn parse_query_params(query: Option<&str>) -> HashMap<String, String> {
        let mut params = HashMap::new();
        if let Some(q) = query {
            for pair in q.split('&') {
                let mut iter = pair.splitn(2, '=');
                if let (Some(key), Some(value)) = (iter.next(), iter.next()) {
                    params.insert(key.to_string(), value.to_string());
                }
            }
        }
        params
    }

    /// Parse cookies from the "cookie" header.
    fn parse_cookies(headers: &HeaderMap) -> HashMap<String, String> {
        let mut cookies = HashMap::new();
        if let Some(cookie_header) = headers.get("cookie") {
            if let Ok(cookie_str) = cookie_header.to_str() {
                // Expect cookies in the form "key1=value1; key2=value2"
                for cookie_pair in cookie_str.split(';') {
                    let cookie_pair = cookie_pair.trim();
                    if let Some(eq_pos) = cookie_pair.find('=') {
                        let key = &cookie_pair[..eq_pos];
                        let value = &cookie_pair[eq_pos + 1..];
                        cookies.insert(key.to_string(), value.to_string());
                    }
                }
            }
        }
        cookies
    }

    /// Public getter that returns a Value for a given key.
    /// For composite keys like headers, params, or cookies, a subkey may be provided.
    pub fn get(&self, key: &str, subkey: Option<&str>) -> Value {
        match key {
            "url" => Value::String(self.data.url.clone()),
            "method" => Value::String(self.data.method.clone()),
            // Convert the Bytes to a Vec<u8> since Value::Bytes expects Vec<u8>
            "body" => Value::Bytes(self.data.body.to_vec()),
            "headers" => {
                if let Some(header_name) = subkey {
                    if let Some(val) = self.data.headers.get(header_name) {
                        Value::String(val.to_str().unwrap_or("").to_string())
                    } else {
                        Value::String("".to_string())
                    }
                } else {
                    let list = self
                        .data
                        .headers
                        .iter()
                        .map(|(k, v)| {
                            Value::List(vec![
                                Value::String(k.to_string()),
                                Value::String(v.to_str().unwrap_or("").to_string()),
                            ])
                        })
                        .collect();
                    Value::List(list)
                }
            }
            "params" => {
                if let Some(param_name) = subkey {
                    if let Some(val) = self.data.params.get(param_name) {
                        Value::String(val.clone())
                    } else {
                        Value::String("".to_string())
                    }
                } else {
                    let list = self
                        .data
                        .params
                        .iter()
                        .map(|(k, v)| {
                            Value::List(vec![Value::String(k.clone()), Value::String(v.clone())])
                        })
                        .collect();
                    Value::List(list)
                }
            }
            "cookie" => {
                if let Some(cookie_name) = subkey {
                    if let Some(val) = self.data.cookies.get(cookie_name) {
                        Value::String(val.clone())
                    } else {
                        Value::String("".to_string())
                    }
                } else {
                    let list = self
                        .data
                        .cookies
                        .iter()
                        .map(|(k, v)| {
                            Value::List(vec![Value::String(k.clone()), Value::String(v.clone())])
                        })
                        .collect();
                    Value::List(list)
                }
            }
            _ => Value::String("".to_string()),
        }
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for HttpRequestAgent {
    async fn init(self: Arc<Self>, _scope: &mut Scope) {
        // No additional initialization needed.
    }

    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
        Arc::new(Mutex::new(Scope::new()))
    }

    async fn stop(&self) {
        // Nothing special to do on stop.
    }

    fn channel(&self) -> &Channel {
        &self.channel
    }

    fn listener(&self) -> Arc<ChannelListener> {
        self.listener.clone()
    }
}

#[async_trait::async_trait]
impl AgentBehavior for HttpRequestAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        // This handles messages in the form: [ "get", <property>, (optional subkey), <reply_channel> ]
        let terms = msg.terms();
        if terms.is_empty() {
            return true;
        }
        if let Value::Word(action) = &terms[0] {
            if action == "get" {
                let key = if let Some(Value::Word(k)) = terms.get(1) {
                    k.as_str()
                } else if let Some(Value::String(k)) = terms.get(1) {
                    k.as_str()
                } else {
                    ""
                };
                let subkey = if terms.len() >= 4 {
                    match &terms[2] {
                        Value::Word(s) | Value::String(s) => Some(s.as_str()),
                        _ => None,
                    }
                } else {
                    None
                };
                let value = self.get(key, subkey);
                // Assume the last term is a reply channel.
                if let Some(Value::Channel(reply_chan)) = terms.last() {
                    let reply_msg = Message::new(vec![value], None);
                    let _ = reply_chan.send(reply_msg).await;
                }
            }
        }
        true
    }
}

impl Agent for HttpRequestAgent {}
