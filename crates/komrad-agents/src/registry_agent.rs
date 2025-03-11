use komrad_agent::{AgentBehavior, AgentLifecycle};
use komrad_ast::prelude::{Block, Channel, ChannelListener, Message, RuntimeError, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// RegistryAgent holds definitions of agents as AST Blocks.
pub struct RegistryAgent {
    pub registry: RwLock<HashMap<String, Block>>,
    channel: Channel,
    listener: Arc<Mutex<ChannelListener>>,
    running: Arc<Mutex<bool>>,
}

impl RegistryAgent {
    pub fn new() -> Arc<Self> {
        let (channel, listener) = Channel::new(32);
        Arc::new(Self {
            registry: RwLock::new(HashMap::new()),
            channel,
            listener: Arc::new(Mutex::new(listener)),
            running: Arc::new(Mutex::new(true)),
        })
    }

    pub fn default() -> Arc<Self> {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for RegistryAgent {
    async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
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

#[async_trait::async_trait]
impl AgentBehavior for RegistryAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        if let Some(cmd) = msg.first_word() {
            match cmd.as_str() {
                "define" => {
                    let terms = msg.terms();
                    if terms.len() < 4 {
                        if let Some(reply_chan) = msg.reply_to() {
                            let reply = Message::new(
                                vec![Value::Error(RuntimeError::InvalidAgentDefinition)],
                                None,
                            );
                            let _ = reply_chan.send(reply).await;
                        }
                        return true;
                    }
                    // Check that the second term is "agent"
                    if let Value::Word(ref keyword) = terms[1] {
                        if keyword != "agent" {
                            if let Some(reply_chan) = msg.reply_to() {
                                let reply = Message::new(
                                    vec![Value::Error(RuntimeError::InvalidAgentDefinition)],
                                    None,
                                );
                                let _ = reply_chan.send(reply).await;
                            }
                            return true;
                        }
                    } else {
                        if let Some(reply_chan) = msg.reply_to() {
                            let reply = Message::new(
                                vec![Value::Error(RuntimeError::InvalidAgentDefinition)],
                                None,
                            );
                            let _ = reply_chan.send(reply).await;
                        }
                        return true;
                    }
                    // Third term is the agent name.
                    let agent_name = if let Value::Word(ref name) = terms[2] {
                        name.clone()
                    } else {
                        if let Some(reply_chan) = msg.reply_to() {
                            let reply = Message::new(
                                vec![Value::Error(RuntimeError::InvalidAgentDefinition)],
                                None,
                            );
                            let _ = reply_chan.send(reply).await;
                        }
                        return true;
                    };
                    // Fourth term must be a Block.
                    if let Value::Block(boxed_block) = &terms[3] {
                        let block = *boxed_block.clone();
                        {
                            let mut reg = self.registry.write().await;
                            reg.insert(agent_name.clone(), block);
                        }
                        if let Some(reply_chan) = msg.reply_to() {
                            // We send a confirmation string on success.
                            let reply = Message::new(vec![Value::String("defined".into())], None);
                            let _ = reply_chan.send(reply).await;
                        }
                    } else {
                        if let Some(reply_chan) = msg.reply_to() {
                            let reply = Message::new(
                                vec![Value::Error(RuntimeError::InvalidAgentDefinition)],
                                None,
                            );
                            let _ = reply_chan.send(reply).await;
                        }
                    }
                }
                "spawn" => {
                    let terms = msg.terms();
                    if terms.len() < 3 {
                        if let Some(reply_chan) = msg.reply_to() {
                            let reply = Message::new(
                                vec![Value::Error(RuntimeError::InvalidAgentDefinition)],
                                None,
                            );
                            let _ = reply_chan.send(reply).await;
                        }
                        return true;
                    }
                    // Check that the second term is "agent"
                    if let Value::Word(ref keyword) = terms[1] {
                        if keyword != "agent" {
                            if let Some(reply_chan) = msg.reply_to() {
                                let reply = Message::new(
                                    vec![Value::Error(RuntimeError::InvalidAgentDefinition)],
                                    None,
                                );
                                let _ = reply_chan.send(reply).await;
                            }
                            return true;
                        }
                    } else {
                        if let Some(reply_chan) = msg.reply_to() {
                            let reply = Message::new(
                                vec![Value::Error(RuntimeError::InvalidAgentDefinition)],
                                None,
                            );
                            let _ = reply_chan.send(reply).await;
                        }
                        return true;
                    }
                    // Third term is the agent name.
                    let agent_name = if let Value::Word(ref name) = terms[2] {
                        name.clone()
                    } else {
                        if let Some(reply_chan) = msg.reply_to() {
                            let reply = Message::new(
                                vec![Value::Error(RuntimeError::InvalidAgentDefinition)],
                                None,
                            );
                            let _ = reply_chan.send(reply).await;
                        }
                        return true;
                    };
                    let reg = self.registry.read().await;
                    if reg.contains_key(&agent_name) {
                        // Spawn a new instance of the agent by creating a new channel.
                        let (spawned_channel, _listener) = Channel::new(32);
                        if let Some(reply_chan) = msg.reply_to() {
                            let reply =
                                Message::new(vec![Value::Channel(spawned_channel.clone())], None);
                            let _ = reply_chan.send(reply).await;
                        }
                    } else {
                        if let Some(reply_chan) = msg.reply_to() {
                            let reply =
                                Message::new(vec![Value::Error(RuntimeError::AgentNotFound)], None);
                            let _ = reply_chan.send(reply).await;
                        }
                    }
                }
                _ => {
                    // Unknown command; for now, ignore.
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use komrad_ast::prelude::{Block, Message, Statement, Value};

    #[tokio::test]
    async fn test_define_agent_valid() {
        let registry = RegistryAgent::new();
        let reg_chan = registry.clone().spawn();

        let (reply_chan, mut reply_listener) = Channel::new(10);
        let block = Block::new(vec![Statement::NoOp]);
        let msg = Message::new(
            vec![
                Value::Word("define".into()),
                Value::Word("agent".into()),
                Value::Word("Alice".into()),
                Value::Block(Box::new(block.clone())),
            ],
            Some(reply_chan.clone()),
        );
        reg_chan.send(msg).await.unwrap();

        let reply = reply_listener.recv().await.unwrap();
        assert_eq!(reply.terms(), &[Value::String("defined".into())]);

        let reg_map = registry.registry.read().await;
        assert!(reg_map.contains_key("Alice"));
        assert_eq!(reg_map.get("Alice").unwrap(), &block);
    }

    #[tokio::test]
    async fn test_define_agent_invalid() {
        let registry = RegistryAgent::new();
        let reg_chan = registry.clone().spawn();

        let (reply_chan, mut reply_listener) = Channel::new(10);
        // Missing the Block definition
        let msg = Message::new(
            vec![
                Value::Word("define".into()),
                Value::Word("agent".into()),
                Value::Word("Alice".into()),
                Value::String("not a block".into()),
            ],
            Some(reply_chan.clone()),
        );
        reg_chan.send(msg).await.unwrap();

        let reply = reply_listener.recv().await.unwrap();
        assert_eq!(
            reply.terms(),
            &[Value::Error(RuntimeError::InvalidAgentDefinition)]
        );
    }

    #[tokio::test]
    async fn test_spawn_agent_defined() {
        let registry = RegistryAgent::new();
        let reg_chan = registry.clone().spawn();

        let block = Block::new(vec![Statement::NoOp]);
        {
            let mut reg_map = registry.registry.write().await;
            reg_map.insert("Alice".to_string(), block);
        }

        let (reply_chan, mut reply_listener) = Channel::new(10);
        let msg = Message::new(
            vec![
                Value::Word("spawn".into()),
                Value::Word("agent".into()),
                Value::Word("Alice".into()),
            ],
            Some(reply_chan.clone()),
        );
        reg_chan.send(msg).await.unwrap();

        let reply = reply_listener.recv().await.unwrap();
        match reply.terms().get(0) {
            Some(Value::Channel(_ch)) => { /* success */ }
            other => panic!("Expected a channel, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_spawn_agent_not_defined() {
        let registry = RegistryAgent::new();
        let reg_chan = registry.clone().spawn();

        let (reply_chan, mut reply_listener) = Channel::new(10);
        let msg = Message::new(
            vec![
                Value::Word("spawn".into()),
                Value::Word("agent".into()),
                Value::Word("Bob".into()),
            ],
            Some(reply_chan.clone()),
        );
        reg_chan.send(msg).await.unwrap();

        let reply = reply_listener.recv().await.unwrap();
        assert_eq!(reply.terms(), &[Value::Error(RuntimeError::AgentNotFound)]);
    }
}
