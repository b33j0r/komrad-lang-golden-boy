use crate::AgentBehavior;
use async_trait::async_trait;
use komrad_ast::prelude::{Channel, ChannelListener, Message, MessageBuilder, Number, Value};
use komrad_ast::scope::Scope;
use komrad_macros::{agent_stateful_impl, agent_stateless_impl};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error};

//
// ------------------ StdLibAgent (Stateless) ------------------
//
pub struct StdLibAgent {
    channel: Channel,
    listener: Arc<ChannelListener>,
}

agent_stateless_impl!(StdLibAgent);

impl StdLibAgent {
    /// Creates a new `ListAgent` from the message’s terms and returns its channel.
    /// The first term is typically `"list"`, and the remaining are the initial items.
    pub fn make_new_list_agent(&self, msg: &Message) -> Channel {
        let items = msg.rest().to_vec();
        let agent = ListAgent::new(items);
        // `spawn()` returns the new agent’s Channel, so we can return that to the caller
        agent.spawn()
    }
}

/// The `StdLibAgent` interprets the first term of a message:
/// - If it’s `"list"`, spawns a new ListAgent and replies with its channel.
/// - Otherwise, logs an unknown command.
#[async_trait]
impl AgentBehavior for StdLibAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        if let Some(cmd) = msg.first_word() {
            if cmd == "List" {
                if let Some(reply_chan) = msg.reply_to() {
                    let list_chan = self.make_new_list_agent(&msg);
                    let reply = Message::default().with_terms(vec![Value::Channel(list_chan)]);
                    if reply_chan.send(reply).await.is_err() {
                        error!("StdLibAgent: failed to reply with new list agent");
                    }
                } else {
                    error!("StdLibAgent: 'list' command requires a reply channel");
                }
            } else {
                debug!("StdLibAgent: unknown command '{cmd}'");
            }
        } else {
            debug!("StdLibAgent: no command in message");
        }
        true
    }
}

//
// ------------------ ListAgent (Stateful) ------------------
//
pub struct ListAgent {
    channel: Channel,
    listener: Arc<ChannelListener>,
    scope: Arc<Mutex<Scope>>,       // required for stateful agents
    items: Arc<RwLock<Vec<Value>>>, // agent's internal data
}

agent_stateful_impl!(ListAgent);

impl ListAgent {
    /// Custom constructor (required for a stateful agent).
    pub fn new(initial_items: Vec<Value>) -> Arc<Self> {
        let (channel, listener) = Channel::new(32);
        Arc::new(Self {
            channel,
            listener: Arc::new(listener),
            scope: Arc::new(Mutex::new(Scope::new())),
            items: Arc::new(RwLock::new(initial_items)),
        })
    }

    // Submethods for internal list operations
    pub async fn handle_items(&self) -> Vec<Value> {
        self.items.read().await.clone()
    }

    pub async fn handle_add_item(&self, item: Value) {
        self.items.write().await.push(item);
    }

    pub async fn handle_get_item(&self, index: usize) -> Option<Value> {
        let items = self.items.read().await;
        items.get(index).cloned()
    }

    pub async fn handle_get_length(&self) -> usize {
        let items = self.items.read().await;
        items.len()
    }
}

/// The `ListAgent` receives messages on its channel (inside its `actor_loop`) and
/// processes them via this `handle_message` method. It can respond via `reply_to()`.
#[async_trait]
impl AgentBehavior for ListAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        match msg.first_word().as_deref() {
            Some("items") => {
                if let Some(reply_chan) = msg.reply_to() {
                    let items = self.items.read().await.clone();
                    let reply = Message::new(vec![Value::List(items)], None);
                    if reply_chan.send(reply).await.is_err() {
                        error!("ListAgent: failed to send 'items' reply");
                    }
                } else {
                    error!("ListAgent: 'items' command requires a reply channel");
                }
            }
            Some("add") => {
                if let Some(item) = msg.rest().get(0) {
                    self.handle_add_item(item.clone()).await;
                    if let Some(reply_chan) = msg.reply_to() {
                        let reply = Message::default().with_terms(vec![Value::from("ok")]);
                        if reply_chan.send(reply).await.is_err() {
                            error!("ListAgent: failed to send 'add' reply");
                        }
                    }
                } else {
                    error!("ListAgent: 'add' requires an item argument");
                }
            }
            Some("get") => {
                if let Some(index_val) = msg.rest().get(0) {
                    let index = match index_val {
                        Value::Number(Number::Int(i)) => Some(*i as usize),
                        Value::Number(Number::UInt(u)) => Some(*u as usize),
                        Value::Number(Number::Float(f)) => Some(*f as usize),
                        _ => None,
                    };
                    if let Some(idx) = index {
                        if let Some(reply_chan) = msg.reply_to() {
                            let item = self.handle_get_item(idx).await.unwrap_or(Value::Empty);
                            let reply = Message::default().with_terms(vec![item]);
                            if reply_chan.send(reply).await.is_err() {
                                error!("ListAgent: failed to send 'get' reply");
                            }
                        } else {
                            error!("ListAgent: 'get' requires a reply channel");
                        }
                    } else {
                        error!("ListAgent: 'get' index argument must be a number");
                    }
                } else {
                    error!("ListAgent: 'get' command requires an index argument");
                }
            }
            Some("length") => {
                if let Some(reply_chan) = msg.reply_to() {
                    let len = self.handle_get_length().await;
                    let reply =
                        Message::default().with_terms(vec![Value::Number(Number::Int(len as i64))]);
                    if reply_chan.send(reply).await.is_err() {
                        error!("ListAgent: failed to send 'length' reply");
                    }
                } else {
                    error!("ListAgent: 'length' requires a reply channel");
                }
            }
            Some(other) => {
                error!("ListAgent: unknown command '{other}'");
                if let Some(reply_chan) = msg.reply_to() {
                    error!("ListAgent: unknown command '{other}'");
                    let reply = Message::default().with_terms(vec![Value::from("error")]);
                    if reply_chan.send(reply).await.is_err() {
                        error!("ListAgent: failed to send 'error' reply");
                    }
                }
            }
            None => {
                error!("ListAgent: no command in message {:?}", msg);
                if let Some(reply_chan) = msg.reply_to() {
                    let reply = Message::default().with_terms(vec![Value::from("error")]);
                    if reply_chan.send(reply).await.is_err() {
                        error!("ListAgent: failed to send 'error' reply");
                    }
                }
            }
        }
        true
    }
}

//
// ------------------ TESTS ------------------
//
#[cfg(test)]
mod tests {
    use super::*;
    use komrad_ast::prelude::{Channel, Message, MessageBuilder, Number, Value};
    use tokio::time::{Duration, sleep};
    use tracing::info;

    #[tokio::test]
    async fn test_stdlib_agent_spawn() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .try_init();

        // 1. Create and spawn a StdLibAgent
        let stdlib_agent = StdLibAgent::new();
        let stdlib_chan = stdlib_agent.clone().spawn();

        // 2. Create a reply channel
        let (reply_chan, reply_listener) = Channel::new(8);

        // 3. Send a "list" command to the agent’s channel
        let msg = Message::default()
            .with_terms(vec![Value::Word("List".into())])
            .with_reply_to(Some(reply_chan));
        stdlib_chan.send(msg).await.unwrap();

        // 4. Wait for the asynchronous reply
        sleep(Duration::from_millis(50)).await;

        // 5. Check the reply
        let reply = reply_listener
            .recv()
            .await
            .expect("Should receive 'list' reply");
        let first_value = reply.terms().get(0).cloned().unwrap_or(Value::Empty);
        match first_value {
            Value::Channel(list_chan) => {
                info!("Received list channel: {:?}", list_chan);
            }
            other => {
                panic!("Expected a Channel, got: {:?}", other);
            }
        }
    }

    #[tokio::test]
    async fn test_list_agent_spawn_items() {
        // 1. Create and spawn a list agent with initial items
        let list_agent = ListAgent::new(vec![Value::from(1), Value::from(2)]);
        let list_chan = list_agent.clone().spawn();

        // 2. Create a reply channel
        let (reply_chan, reply_listener) = Channel::new(8);

        // 3. Send a "items" command to the agent’s channel
        let msg = Message::new(vec![Value::Word("items".into())], Some(reply_chan.clone()));
        list_chan.send(msg).await.unwrap();

        // Wait for the reply
        let reply = reply_listener.recv().await.unwrap();

        let items = reply.terms();
        assert_eq!(items.len(), 1);

        println!("items: {:?}", items);
    }

    #[tokio::test]
    async fn test_list_agent_spawn_add_and_length() {
        // 1. Create and spawn an empty list agent
        let list_agent = ListAgent::new(vec![]);
        let list_chan = list_agent.clone().spawn();

        // 2. Add "42" to the list
        let (reply_chan_add, reply_listener_add) = Channel::new(8);
        let msg_add = Message::default()
            .with_terms(vec![Value::Word("add".into()), Value::from(42)])
            .with_reply_to(Some(reply_chan_add));
        list_chan.send(msg_add).await.unwrap();

        // Wait briefly
        sleep(Duration::from_millis(50)).await;
        let add_reply = reply_listener_add
            .recv()
            .await
            .expect("Should receive 'add' reply");
        assert_eq!(add_reply.terms().get(0).unwrap(), &Value::from("ok"));

        // 3. Check the length
        let (reply_chan_len, reply_listener_len) = Channel::new(8);
        let msg_len = Message::default()
            .with_terms(vec![Value::Word("length".into())])
            .with_reply_to(Some(reply_chan_len));
        list_chan.send(msg_len).await.unwrap();

        sleep(Duration::from_millis(50)).await;
        let len_reply = reply_listener_len
            .recv()
            .await
            .expect("Should receive 'length' reply");
        match len_reply.terms().get(0) {
            Some(Value::Number(Number::Int(n))) => {
                assert_eq!(*n, 1);
            }
            other => {
                panic!("Expected length=1, got: {:?}", other);
            }
        }
    }

    #[tokio::test]
    async fn test_list_agent_spawn_get() {
        // 1. Create and spawn a list agent with initial items: ["a", "b"]
        let list_agent = ListAgent::new(vec![Value::from("a"), Value::from("b")]);
        let list_chan = list_agent.clone().spawn();

        // 2. Send a "get 1" command
        let (reply_chan, reply_listener) = Channel::new(8);
        let msg_get = Message::default()
            .with_terms(vec![Value::Word("get".into()), Value::from(1)])
            .with_reply_to(Some(reply_chan));
        list_chan.send(msg_get).await.unwrap();

        sleep(Duration::from_millis(50)).await;
        // 3. Check the reply
        let reply_get = reply_listener
            .recv()
            .await
            .expect("Should receive 'get' reply");
        assert_eq!(reply_get.terms().get(0).unwrap(), &Value::from("b"));
    }
}
