use crate::dynamic_agent::DynamicAgent;
use komrad_agent::execute::Execute;
use komrad_agent::stdlib_agent::ListAgentFactory;
use komrad_agent::{AgentBehavior, AgentFactory, AgentLifecycle};
use komrad_ast::prelude::{Block, Channel, ChannelListener, Message, RuntimeError, ToSexpr, Value};
use komrad_ast::scope::Scope;

#[cfg(feature = "templates")]
use komrad_web::TeraAgentFactory;

#[cfg(feature = "hyper")]
use komrad_web::HyperListenerFactory;

#[cfg(feature = "axum")]
use komrad_web::AxumListenerFactory;

#[cfg(feature = "warp")]
use komrad_web::WarpListenerFactory;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info};

pub enum RegistryFactory {
    FromBlock(Block),
    FromFactory(Arc<dyn AgentFactory>),
}

/// RegistryAgent holds definitions of agents as AST Blocks.
pub struct RegistryAgent {
    pub registry: RwLock<HashMap<String, RegistryFactory>>,
    channel: Channel,
    listener: Arc<ChannelListener>,
}

impl RegistryAgent {
    pub fn new() -> Arc<Self> {
        let (channel, listener) = Channel::new(32);
        let mut initial_registry: HashMap<String, RegistryFactory> = HashMap::new();

        #[cfg(feature = "hyper")]
        initial_registry.insert(
            "HyperListener".to_string(),
            RegistryFactory::FromFactory(Arc::new(HyperListenerFactory)),
        );

        #[cfg(feature = "axum")]
        initial_registry.insert(
            "AxumListener".to_string(),
            RegistryFactory::FromFactory(Arc::new(AxumListenerFactory)),
        );
        #[cfg(feature = "warp")]
        initial_registry.insert(
            "WarpListener".to_string(),
            RegistryFactory::FromFactory(Arc::new(WarpListenerFactory)),
        );
        #[cfg(feature = "templates")]
        initial_registry.insert(
            "Tera".to_string(),
            RegistryFactory::FromFactory(Arc::new(TeraAgentFactory {
                base_dir: PathBuf::from("."),
            })),
        );
        initial_registry.insert(
            "List".to_string(),
            RegistryFactory::FromFactory(Arc::new(ListAgentFactory)),
        );
        let registry = RwLock::new(initial_registry);

        Arc::new(Self {
            registry,
            channel,
            listener: Arc::new(listener),
        })
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for RegistryAgent {
    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
        // We don't have a specific scope for this agent, but we can return a new one.
        Arc::new(Mutex::new(Scope::new()))
    }

    fn channel(&self) -> &Channel {
        &self.channel
    }

    fn listener(&self) -> Arc<ChannelListener> {
        self.listener.clone()
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
                            let reply = Message::new(
                                vec![Value::Error(RuntimeError::AgentNotRegistered(agent_name))],
                                None,
                            );
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

        let (reply_chan, reply_listener) = Channel::new(10);
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
                assert_eq!(block.statements().len(), 1);
                if let Statement::NoOp = block.statements()[0] {
                    // success
                } else {
                    panic!("Expected a NoOp statement");
                }
            }
            _ => panic!("Expected a block, got something else"),
        }
    }

    #[tokio::test]
    async fn test_define_agent_invalid() {
        let registry = RegistryAgent::new();
        let reg_chan = registry.clone().spawn();

        let (reply_chan, reply_listener) = Channel::new(10);
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

        let (reply_chan, reply_listener) = Channel::new(10);
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

        let (reply_chan, reply_listener) = Channel::new(10);
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
        assert_eq!(
            reply.terms(),
            &[Value::Error(RuntimeError::AgentNotRegistered)]
        );
    }
}
