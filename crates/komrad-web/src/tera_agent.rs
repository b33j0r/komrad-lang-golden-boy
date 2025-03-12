use crate::HttpListener;
use komrad_agent::execute::Execute;
use komrad_agent::scope::Scope;
use komrad_agent::{Agent, AgentBehavior, AgentFactory, AgentLifecycle};
use komrad_ast::prelude::{Channel, ChannelListener, Message, ToSexpr, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tera::Tera;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};

/// Interface to the Tera templating engine.
pub struct TeraAgent {
    name: String,
    base_dir: PathBuf,
    running: Arc<Mutex<bool>>,
    channel: Channel, // We'll store our sending handle
    listener: Arc<Mutex<ChannelListener>>,
    scope: Arc<Mutex<Scope>>,
}

impl TeraAgent {
    pub fn new(base_dir: &Path, name: &str, scope: Scope) -> Arc<Self> {
        let (chan, listener) = Channel::new(32);
        Arc::new(Self {
            name: name.to_string(),
            base_dir: base_dir.to_path_buf(),
            running: Arc::new(Mutex::new(true)),
            channel: chan,
            listener: Arc::new(Mutex::new(listener)),
            scope: Arc::new(Mutex::new(scope)),
        })
    }

    pub async fn render_tera_from_file_name(
        &self,
        template_name: &str,
        scope: Scope,
    ) -> Result<String, tera::Error> {
        // Load the Tera templates from the base directory
        let tera = Tera::new(&format!("{}/**/*.html", self.base_dir.display()))?;

        // Make context from scope
        let mut context = tera::Context::new();
        for (name, value) in scope.iter().await {
            context.insert(name, value.to_string().as_str());
        }

        // Render the template with the provided context
        let rendered = tera.render(template_name, &context).map_err(|e| {
            error!("Error rendering template: {}", e);
            e
        })?;

        Ok(rendered)
    }
}

#[async_trait::async_trait]
impl AgentBehavior for TeraAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        match msg.first_word() {
            Some(word) => match word.as_str() {
                "render" => {
                    // Check if the message has the correct number of terms
                    if msg.terms().len() != 3 {
                        warn!("Invalid number of terms for render command");
                        return false;
                    }
                    // Render a template using Tera
                    let template_name = msg.terms()[1].to_string();
                    let context = msg.terms()[2].clone();
                    // Check that context is a block and get its scope
                    if let Value::Block(block) = context {
                        let mut block_scope = Scope::new();
                        let result = block.execute(&mut block_scope).await;
                        info!("Rendering template: {}", template_name);

                        if let Some(reply_chan) = msg.reply_to() {
                            let result = self
                                .render_tera_from_file_name(&template_name, block_scope)
                                .await;

                            match result {
                                Ok(rendered) => {
                                    // Send the rendered template back to the reply channel
                                    let reply_msg =
                                        Message::new(vec![Value::String(rendered)], None);
                                    reply_chan.send(reply_msg).await.unwrap();
                                }
                                Err(e) => {
                                    error!("Error rendering template: {}", e);
                                    let error_msg =
                                        Message::new(vec![Value::String(e.to_string())], None);
                                    reply_chan.send(error_msg).await.unwrap();
                                }
                            }
                        }
                        true
                    } else {
                        false
                    }
                }
                _ => {
                    // Handle other messages
                    info!(
                        "Received unknown Tera message: {:?}",
                        msg.to_sexpr().format(0)
                    );
                    true
                }
            },
            None => {
                // Handle other messages
                info!("Received message: {:?}", msg);
                true
            }
        }
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for TeraAgent {
    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
        return self.scope.clone();
    }

    async fn stop(&self) {
        match self.running.try_lock() {
            Ok(mut running) => {
                *running = false;
            }
            Err(_) => {
                warn!("TeraAgent is already stopped");
            }
        }
    }

    fn is_running(&self) -> bool {
        match self.running.try_lock() {
            Ok(running) => *running,
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

impl Agent for TeraAgent {}

pub struct TeraAgentFactory {
    pub base_dir: PathBuf,
}

#[async_trait::async_trait]
impl AgentFactory for TeraAgentFactory {
    fn create_agent(&self, name: &str, initial_scope: Scope) -> Arc<dyn Agent> {
        TeraAgent::new(&self.base_dir, name, initial_scope)
    }
}
