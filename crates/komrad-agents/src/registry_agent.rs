use crate::agent_agent::AgentAgent;
use crate::dynamic_agent::DynamicAgent;
use komrad_agent::execute::Execute;
use komrad_agent::scope::Scope;
use komrad_agent::{AgentBehavior, AgentControl, AgentFactory, AgentLifecycle, AgentState};
use komrad_ast::prelude::{Block, Channel, ChannelListener, Message, RuntimeError, ToSexpr, Value};
use komrad_web::{HttpListenerFactory, TeraAgentFactory};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

pub enum RegistryFactory {
    FromBlock(Block),
    FromFactory(Arc<dyn AgentFactory>),
}

/// RegistryAgent holds definitions of agents as AST Blocks.
pub struct RegistryAgent {
    pub registry: RwLock<HashMap<String, RegistryFactory>>,
    channel: Channel,
    listener: Arc<Mutex<ChannelListener>>,
    control_tx: tokio::sync::mpsc::Sender<AgentControl>,
    control_rx: Mutex<tokio::sync::mpsc::Receiver<AgentControl>>,
    state_tx: tokio::sync::watch::Sender<AgentState>,
    state_rx: tokio::sync::watch::Receiver<AgentState>,
}

impl Drop for RegistryAgent {
    fn drop(&mut self) {
        debug!("RegistryAgent is being dropped");
        self.control_tx.send(AgentControl::Stop);
    }
}

impl RegistryAgent {
    pub fn new() -> Arc<Self> {
        let (channel, listener) = Channel::new(32);
        let mut initial_registry: HashMap<String, RegistryFactory> = HashMap::new();
        initial_registry.insert(
            "HttpListener".to_string(),
            RegistryFactory::FromFactory(Arc::new(HttpListenerFactory)),
        );
        initial_registry.insert(
            "Tera".to_string(),
            RegistryFactory::FromFactory(Arc::new(TeraAgentFactory {
                base_dir: PathBuf::from("."),
            })),
        );
        let registry = RwLock::new(initial_registry);
        let (control_tx, control_rx) = tokio::sync::mpsc::channel(8);
        let (state_tx, state_rx) = tokio::sync::watch::channel(AgentState::Started);

        Arc::new(Self {
            registry,
            channel,
            listener: Arc::new(Mutex::new(listener)),
            control_tx,
            control_rx: Mutex::new(control_rx),
            state_tx,
            state_rx,
        })
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for RegistryAgent {
    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
        // We don't have a specific scope for this agent, but we can return a new one.
        Arc::new(Mutex::new(Scope::new()))
    }

    async fn stop(&self) {
        self.control_tx.send(AgentControl::Stop).await.unwrap();
    }

    fn channel(&self) -> &Channel {
        &self.channel
    }

    fn listener(&self) -> &Mutex<ChannelListener> {
        &self.listener
    }

    async fn recv_control(&self) -> Result<AgentControl, komrad_ast::prelude::RuntimeError> {
        let mut control = self.control_rx.lock().await;
        match control.recv().await {
            Some(control) => Ok(control),
            None => Err(komrad_ast::prelude::RuntimeError::ReceiveControlError),
        }
    }

    async fn notify_stopped(&self) {
        // Notify the agent that it has stopped
        let _ = self.state_tx.send(AgentState::Stopped);
    }
}

#[async_trait::async_trait]
impl AgentBehavior for RegistryAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        info!("⚙️ {:}", msg.to_sexpr().format(0));
        if let Some(cmd) = msg.first_word() {
            match cmd.as_str() {
                "define" => {
                    debug!("RegistryAgent: define command received");
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
                            reg.insert(agent_name.clone(), RegistryFactory::FromBlock(block));
                        }
                        if let Some(reply_chan) = msg.reply_to() {
                            // We send a confirmation string on success.
                            let reply = Message::new(vec![Value::Word(agent_name)], None);
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
                    debug!("RegistryAgent: spawn command received");
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

                    // Fourth term must be a Block if provided
                    let initial_scope_block = if terms.len() > 3 {
                        if let Value::Block(boxed_block) = &terms[3] {
                            Some(*boxed_block.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let reg = self.registry.read().await;
                    if reg.contains_key(&agent_name) {
                        // Create the initial scope by executing the initial scope block
                        let initial_scope_block = if let Some(block) = initial_scope_block {
                            block
                        } else {
                            Block::new(vec![])
                        };
                        let mut initial_scope = Scope::new();
                        initial_scope_block.execute(&mut initial_scope).await;

                        // Invoke the correct factory method
                        let agent_chan = match reg.get(&agent_name).unwrap() {
                            RegistryFactory::FromBlock(block) => {
                                let agent =
                                    DynamicAgent::from_block(&agent_name, block, initial_scope)
                                        .await;
                                info!("RegistryAgent: spawning agent {} from block", agent_name);
                                agent.clone().spawn()
                            }
                            RegistryFactory::FromFactory(factory) => {
                                let agent = factory.create_agent(&agent_name, initial_scope);
                                info!("RegistryAgent: spawning agent {} from factory", agent_name);
                                agent.clone().spawn()
                            }
                        };
                        if let Some(reply_chan) = msg.reply_to() {
                            let reply = Message::new(vec![Value::Channel(agent_chan)], None);
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
        assert_eq!(reply.terms(), &[Value::Word("Alice".into())]);

        let reg_map = registry.registry.read().await;
        assert!(reg_map.contains_key("Alice"));
        match reg_map.get("Alice").unwrap() {
            RegistryFactory::FromBlock(block) => {
                assert_eq!(block, &block);
            }
            _ => panic!("Expected a block, got something else"),
        }
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
            reg_map.insert(
                "Alice".to_string(),
                RegistryFactory::FromBlock(block.clone()),
            );
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
