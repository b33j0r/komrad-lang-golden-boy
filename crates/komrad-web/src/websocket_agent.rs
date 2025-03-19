// websocket_agent.rs

use async_trait::async_trait;
use futures::stream::{SplitSink, SplitStream};
use futures::SinkExt;
use futures::StreamExt;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use komrad_agent::{Agent, AgentBehavior, AgentLifecycle};
use komrad_ast::prelude::{Channel, ChannelListener, ControlMessage, Message, Scope, Value};
use std::sync::Arc;
use tokio::select;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tokio_tungstenite::WebSocketStream;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

pub struct WebSocketAgent {
    name: String,
    channel: Channel,
    listener: Arc<ChannelListener>,
    // Writer half for sending messages concurrently.
    ws_sink: Arc<Mutex<SplitSink<WebSocketStream<TokioIo<Upgraded>>, WsMessage>>>,
    // Reader half for processing incoming messages.
    ws_stream: Arc<Mutex<SplitStream<WebSocketStream<TokioIo<Upgraded>>>>>,
    // Delegate channel for forwarding ws events.
    delegate: Arc<Mutex<Option<Channel>>>,
    // Cancellation token for graceful shutdown.
    cancellation_token: CancellationToken,
}

impl WebSocketAgent {
    pub fn new(name: &str, ws_stream_full: WebSocketStream<TokioIo<Upgraded>>) -> Arc<Self> {
        // Use futures_util's split so that both halves come from the same crate.
        let (sink, stream) = ws_stream_full.split();
        let (channel, listener) = Channel::new(32);
        Arc::new(Self {
            name: name.to_string(),
            channel,
            listener: Arc::new(listener),
            ws_sink: Arc::new(Mutex::new(sink)),
            ws_stream: Arc::new(Mutex::new(stream)),
            delegate: Arc::new(Mutex::new(None)),
            cancellation_token: CancellationToken::new(),
        })
    }
}

#[async_trait]
impl AgentLifecycle for WebSocketAgent {
    async fn init(self: Arc<Self>, _scope: &mut Scope) {
        error!("WebSocketAgent init: {}", self.name);
        // Immediately notify the delegate that we are "connected".
        let msg = Message::new(
            vec![
                Value::Word("ws".into()),
                Value::Channel(self.channel.clone()),
                Value::Word("connected".into()),
            ],
            None,
        );
        if let Some(delegate) = self.delegate.lock().await.as_ref() {
            let _ = delegate.send(msg).await;
        } else {
            warn!(
                "No delegate set for WebSocketAgent {}. `connected` not sent.",
                self.name
            );
        }
        // Spawn the read loop task.
        let this = self.clone();
        let cancellation_token = self.cancellation_token.clone();
        tokio::spawn(async move {
            this.read_loop(cancellation_token).await;
        });
    }

    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
        Arc::new(Mutex::new(Scope::new()))
    }

    async fn stop(&self) {
        debug!("WebSocketAgent stopping: {}", self.name);
        self.cancellation_token.cancel();
    }

    fn channel(&self) -> &Channel {
        &self.channel
    }

    fn listener(&self) -> Arc<ChannelListener> {
        self.listener.clone()
    }
}

#[async_trait]
impl AgentBehavior for WebSocketAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        let terms = msg.terms();
        if terms.is_empty() {
            return true;
        }
        let Some(Value::Word(cmd)) = terms.get(0) else {
            return true;
        };

        match cmd.as_str() {
            "send" => {
                // E.g. [ send "Hello there!" ]
                if let Some(text_val) = terms.get(1) {
                    let text_str = text_val.to_string();
                    let mut sink = self.ws_sink.lock().await;
                    // Convert the String into the type expected by WsMessage::Text
                    if let Err(e) = sink.send(WsMessage::Text(text_str.clone().into())).await {
                        error!("Error sending text message via WebSocket: {:?}", e);
                    } else {
                        info!("Sent message via WebSocket: {:?}", text_str);
                    }
                }
            }
            "set-delegate" => {
                // E.g. [ set-delegate channel ]
                error!("WebSocketAgent delegate received {}", self.name);
                if let Some(Value::Channel(channel)) = terms.get(1) {
                    self.delegate.lock().await.replace(channel.clone());
                    info!("WebSocketAgent {} set delegate to {:?}", self.name, channel);
                }
            }
            _ => {
                debug!("WebSocketAgent ignoring unknown command: {:?}", cmd);
            }
        }
        true
    }
}

impl Agent for WebSocketAgent {}

impl WebSocketAgent {
    async fn read_loop(self: Arc<Self>, cancellation_token: CancellationToken) {
        loop {
            select! {
                _ = cancellation_token.cancelled() => {
                    info!("WebSocketAgent {} cancelled", self.name);
                    let msg = Message::new(
                        vec![
                            Value::Word("ws".into()),
                            Value::Channel(self.channel.clone()),
                            Value::Word("disconnected".into()),
                        ],
                        None,
                    );
                    if let Some(delegate) = self.delegate.lock().await.as_ref() {
                        let _ = delegate.send(msg).await;
                    } else {
                        warn!("No delegate set for WebSocketAgent {}. `disconnected` not sent.", self.name);
                    }
                    break;
                }
                result = async {
                    let mut stream_guard = self.ws_stream.lock().await;
                    stream_guard.next().await
                } => {
                    match result {
                        Some(Ok(WsMessage::Text(text))) => {
                            // Convert the Utf8Bytes into a Rust String.
                            let msg = Message::new(
                                vec![
                                    Value::Word("ws".into()),
                                    Value::Channel(self.channel.clone()),
                                    Value::Word("text".into()),
                                    Value::String(text.to_string()),
                                ],
                                None,
                            );
                            if let Some(delegate) = self.delegate.lock().await.as_ref() {
                                let _ = delegate.send(msg).await;
                                info!("WebSocketAgent {} sent message to delegate: {:?}", self.name, text);
                            } else {
                                warn!("No delegate set for WebSocketAgent {}. `ws _ text _` not sent.", self.name);
                            }
                        }
                        Some(Ok(WsMessage::Binary(_bin))) => {
                            // Optionally handle binary messages here.
                        }
                        Some(Ok(WsMessage::Close(_frame))) => {
                            let msg = Message::new(
                                vec![
                                    Value::Word("ws".into()),
                                    Value::Channel(self.channel.clone()),
                                    Value::Word("disconnected".into()),
                                ],
                                None,
                            );
                            if let Some(delegate) = self.delegate.lock().await.as_ref() {
                                let _ = delegate.send(msg).await;
                            } else {
                                warn!("No delegate set for WebSocketAgent {}. `disconnected` on close not sent.", self.name);
                            }
                            break;
                        }
                        Some(Ok(WsMessage::Ping(_))) => {
                            info!("WebSocketAgent {} received ping", self.name);
                            break;
                        }
                        Some(Ok(WsMessage::Pong(_))) => {
                            info!("WebSocketAgent {} received pong", self.name);
                            break;
                        }
                        Some(Err(e)) => {
                            error!("WebSocket read error: {:?}", e);
                            let msg = Message::new(
                                vec![
                                    Value::Word("ws".into()),
                                    Value::Channel(self.channel.clone()),
                                    Value::Word("disconnected".into()),
                                ],
                                None,
                            );
                            if let Some(delegate) = self.delegate.lock().await.as_ref() {
                                let _ = delegate.send(msg).await;
                            } else {
                                warn!("No delegate set for WebSocketAgent {}. `disconnected` on error not sent.", self.name);
                            }
                            break;
                        }
                        None => break,
                    Some(_) => {
                            // Ignore other message types for now.
                        }
                    }
                }
            }
        }
        let _ = self.channel.control(ControlMessage::Stop).await;
    }
}
