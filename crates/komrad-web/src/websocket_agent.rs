// for ws_stream.next()
use async_trait::async_trait;
use futures::SinkExt;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use komrad_agent::{Agent, AgentBehavior, AgentLifecycle};
use komrad_ast::prelude::{
    Channel, ChannelListener, ControlMessage, Message, Number, Scope, Value,
};
use std::sync::Arc;
use tokio::select;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tokio_tungstenite::WebSocketStream;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

pub struct WebSocketAgent {
    name: String,
    channel: Channel,
    listener: Arc<ChannelListener>,
    // The tungstenite WebSocket stream
    ws_stream: Arc<Mutex<WebSocketStream<TokioIo<Upgraded>>>>,
    // The delegate that receives messages [ws myChannel text/disconnected/etc.]
    delegate: Channel,
    // Cancellation token for graceful shutdown
    cancellation_token: CancellationToken,
}

impl WebSocketAgent {
    pub fn new(
        name: &str,
        ws_stream: WebSocketStream<TokioIo<Upgraded>>,
        delegate: Channel,
    ) -> Arc<Self> {
        let (channel, listener) = Channel::new(32);
        Arc::new(Self {
            name: name.to_string(),
            channel,
            listener: Arc::new(listener),
            ws_stream: Arc::new(Mutex::new(ws_stream)),
            delegate,
            cancellation_token: CancellationToken::new(),
        })
    }
}

#[async_trait]
impl AgentLifecycle for WebSocketAgent {
    async fn init(self: Arc<Self>, _scope: &mut Scope) {
        error!("WebSocketAgent init: {}", self.name);
        // Immediately notify the Komrad delegate that we are "connected".
        // e.g. [ws <myChannel> connected]
        let msg = Message::new(
            vec![
                Value::Word("ws".into()),
                Value::Channel(self.channel.clone()),
                Value::Word("connected".into()),
            ],
            None,
        );
        let _ = self.delegate.send(msg).await;

        // Spawn a background task to read from the socket and forward messages
        // to the delegate
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
        // Optionally send a "disconnected" message if we're shutting down abruptly
        debug!("WebSocketAgent stopping: {}", self.name);
        // Clean up, close the stream, etc.
        let _ = self.cancellation_token.cancel();
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
        // Expect messages like [ send "some text" ], etc.
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
                    let mut ws = self.ws_stream.lock().await;
                    if let Err(e) = ws.send(WsMessage::Text(text_str.into())).await {
                        error!("Error sending text message via WebSocket: {:?}", e);
                    }
                }
            }
            // You might add "close" or "ping" commands here, too.
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
        let mut ws = self.ws_stream.lock().await;
        loop {
            select! {
                // Wait for cancellation
                _ = cancellation_token.cancelled() => {
                    info!("WebSocketAgent {} cancelled", self.name);
                    // Notify delegate of disconnection
                    let msg = Message::new(
                        vec![
                            Value::Word("ws".into()),
                            Value::Channel(self.channel.clone()),
                            Value::Word("disconnected".into()),
                        ],
                        None,
                    );
                    break;
                }
                // Read messages from the WebSocket
                Some(msg_result) = ws.next() => {
                    match msg_result {
                        Ok(WsMessage::Text(text)) => {
                            // Forward to the Komrad delegate:
                            // [ws <myChannel> text <String>]
                            let msg = Message::new(
                                vec![
                                    Value::Word("ws".into()),
                                    Value::Channel(self.channel.clone()),
                                    Value::Word("text".into()),
                                    Value::String(text.to_string()),
                                ],
                                None,
                            );
                            let _ = self.delegate.send(msg).await;
                        }
                        Ok(WsMessage::Binary(bin)) => {
                            // If you want to handle binary, do similarly:
                            // [ws channel binary <Bytes>]
                            // or ignore it.
                        }
                        Ok(WsMessage::Close(_frame)) => {
                            // Notify delegate of disconnection, then stop
                            let msg = Message::new(
                                vec![
                                    Value::Word("ws".into()),
                                    Value::Channel(self.channel.clone()),
                                    Value::Word("disconnected".into()),
                                ],
                                None,
                            );
                            let _ = self.delegate.send(msg).await;
                            break;
                        }
                        Err(e) => {
                            error!("WebSocket read error: {:?}", e);
                            // Possibly notify "disconnected"
                            let msg = Message::new(
                                vec![
                                    Value::Word("ws".into()),
                                    Value::Channel(self.channel.clone()),
                                    Value::Word("disconnected".into()),
                                ],
                                None,
                            );
                            let _ = self.delegate.send(msg).await;
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
        // Once the loop finishes, stop the agent
        let _ = self.channel.control(ControlMessage::Stop).await;
    }
}
