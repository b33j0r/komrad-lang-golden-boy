use ollama_rs::{
    Ollama,
    generation::{
        completion::request::GenerationRequest,
        options::GenerationOptions,
        parameters::{FormatType, JsonSchema, JsonStructure},
    },
};
use serde::Deserialize;

pub enum KomradCode {
    GetScope,
    GetVariable { name: String },
    SetVariable { name: String, value: String },
    SendMessage { target: String, args: Vec<String> },
}

#[derive(JsonSchema, Deserialize, Debug)]
enum Action {
    Say(String),
}

#[allow(dead_code)]
#[derive(JsonSchema, Deserialize, Debug)]
struct Output {
    summary: String,
    actions: Vec<Action>,
}

pub struct Context {
    pub system_prompt: String,
    pub personality: String,
    pub memory: String,
    pub current_prompt: String,
    pub messages: Vec<String>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            system_prompt: r##"
You are a helpful assistant.
Respond in json format.
Use multiple Say actions to say multiple sentences.
            "##
            .to_string(),
            personality: "You talk like a pirate.".to_string(),
            memory: "The user doesn't like apologies".to_string(),
            current_prompt: "Tell me a joke".to_string(),
            messages: vec![],
        }
    }

    pub fn with_system_prompt(mut self, system_prompt: &str) -> Self {
        self.system_prompt = system_prompt.to_string();
        self
    }

    pub fn with_personality(mut self, personality: &str) -> Self {
        self.personality = personality.to_string();
        self
    }

    pub fn with_memory(mut self, memory: &str) -> Self {
        self.memory = memory.to_string();
        self
    }

    pub fn with_current_prompt(mut self, current_prompt: &str) -> Self {
        self.current_prompt = current_prompt.to_string();
        self
    }

    pub fn prompt(&self) -> String {
        format!(
            "{}\n{}\n{}\n{}",
            self.system_prompt.trim(),
            self.personality.trim(),
            self.memory.trim(),
            self.current_prompt.trim(),
        )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ollama = Ollama::default();
    let model = "gemma3:latest".to_string();
    let context = Context::new().with_current_prompt("Make up a story about a cat");
    let prompt = context.prompt();

    let format = FormatType::StructuredJson(JsonStructure::new::<Output>());
    dbg!(&format);
    let res = ollama
        .generate(
            GenerationRequest::new(model, prompt)
                .format(format)
                .options(GenerationOptions::default().temperature(0.0)),
        )
        .await?;

    dbg!(&res.response);
    let resp: Output = serde_json::from_str(&res.response)?;

    // Output {
    //     country: "Canada",
    //     capital: "Ottawa",
    //     languages: [
    //         "English",
    //         "French",
    //     ],
    //     temperature: Cold,
    // }
    dbg!(resp);

    Ok(())
}
