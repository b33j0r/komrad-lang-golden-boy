use crate::conversation::Conversation;
use komrad_agent::agent_lifecycle_impl;
use komrad_ast::agent::{Agent, AgentBehavior, AgentLifecycle};
use komrad_ast::prelude::{
    AgentFactory, Channel, ChannelListener, Message, MessageBuilder, RuntimeError, Scope, Value,
};
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::options::GenerationOptions;
use ollama_rs::Ollama;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

pub struct OllamaAgent {
    scope: Scope,
    channel: Channel,
    listener: Arc<ChannelListener>,
    ollama: Arc<Mutex<Ollama>>,
    model: String,
    temperature: f32,
    conversations: HashMap<String, Conversation>,
}

impl OllamaAgent {
    pub fn new(model: String, temperature: f32) -> Self {
        let scope = Scope::new();
        let (channel, channel_listener) = Channel::new(32);
        let ollama = Ollama::default();
        Self {
            scope,
            channel,
            listener: Arc::new(channel_listener),
            ollama: Arc::new(Mutex::new(ollama)),
            model,
            temperature,
            conversations: HashMap::new(),
        }
    }

    pub async fn handle_generate(&self, prompt: &str) -> Result<String, String> {
        let request = GenerationRequest::new(self.model.clone(), prompt)
            .options(GenerationOptions::default().temperature(self.temperature));
        let ollama = self.ollama.lock().await;
        let response = ollama.generate(request).await.map_err(|e| e.to_string())?;
        Ok(response.response)
    }

    pub async fn handle_conversation(&self) -> Result<String, String> {
        Ok("Heyo".to_string())
    }
}

agent_lifecycle_impl!(OllamaAgent);

#[async_trait::async_trait]
impl AgentBehavior for OllamaAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        match msg.first_word().unwrap().as_str() {
            "generate" => {
                let default_prompt = Value::String("Write a poem about a cat".to_string());

                let prompt = msg.rest().get(0).unwrap_or(&default_prompt);

                if let Value::String(prompt) = prompt {
                    match self.handle_generate(prompt).await {
                        Ok(response) => {
                            if let Some(reply_to) = msg.reply_to() {
                                let response =
                                    Message::default().with_term(Value::String(response));
                                match reply_to.send(response).await {
                                    Ok(_) => {
                                        info!("Message sent successfully");
                                    }
                                    Err(e) => {
                                        error!("Failed to send message: {}", e);
                                    }
                                }
                            } else {
                                error!("No reply_to found");
                            }
                        }
                        Err(e) => {
                            error!("Error generating response: {}", e);
                            if let Some(reply_to) = msg.reply_to() {
                                let error_message = Message::default()
                                    .with_term(Value::Error(RuntimeError::ExternalServiceError));
                                reply_to.send(error_message).await.unwrap_or_else(|e| {
                                    error!("Failed to send error message: {}", e);
                                });
                            }
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }
}

impl Agent for OllamaAgent {}

pub struct OllamaAgentFactory;

impl AgentFactory for OllamaAgentFactory {
    fn create_agent(&self, name: &str, initial_scope: Scope) -> Arc<dyn Agent> {
        let model = "gemma3:latest".to_string();
        let temperature = 0.5;
        Arc::new(OllamaAgent::new(model, temperature))
    }
}
