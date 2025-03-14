use komrad_agent::scope::Scope;
use komrad_agent::{Agent, AgentBehavior, AgentLifecycle};
use komrad_ast::prelude::{Channel, ChannelListener, Message, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

// Use Tokio's async FS API and stream utilities.
use tokio::fs;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReadDirStream;

pub struct FsAgent {
    channel: Channel,
    listener: Arc<ChannelListener>,
}

impl FsAgent {
    /// Create a new FsAgent.
    pub fn new() -> Arc<Self> {
        let (channel, listener) = Channel::new(32);
        Arc::new(Self {
            channel,
            listener: Arc::new(listener),
        })
    }

    /// Handler for "read-all" command.
    async fn handle_read_all(&self, msg: &Message) {
        if msg.terms().len() < 2 {
            error!("read-all: missing file path");
            return;
        }
        let file_path = match &msg.terms()[1] {
            Value::String(s) => s.clone(),
            other => {
                error!("read-all: file path is not a string: {:?}", other);
                return;
            }
        };

        match fs::read_to_string(&file_path).await {
            Ok(contents) => {
                if let Some(reply_chan) = msg.reply_to() {
                    let reply = Message::new(vec![Value::String(contents)], None);
                    let _ = reply_chan.send(reply).await;
                }
            }
            Err(e) => {
                error!("read-all: error reading {}: {:?}", file_path, e);
                if let Some(reply_chan) = msg.reply_to() {
                    let reply = Message::new(vec![Value::String(format!("Error: {:?}", e))], None);
                    let _ = reply_chan.send(reply).await;
                }
            }
        }
    }

    /// Handler for "read-all-binary" command.
    async fn handle_read_all_binary(&self, msg: &Message) {
        if msg.terms().len() < 2 {
            error!("read-all-binary: missing file path");
            return;
        }
        let file_path = match &msg.terms()[1] {
            Value::String(s) => s.clone(),
            other => {
                error!("read-all-binary: file path is not a string: {:?}", other);
                return;
            }
        };

        match fs::read(&file_path).await {
            Ok(contents) => {
                if let Some(reply_chan) = msg.reply_to() {
                    let reply = Message::new(vec![Value::Bytes(contents)], None);
                    let _ = reply_chan.send(reply).await;
                }
            }
            Err(e) => {
                error!("read-all-binary: error reading {}: {:?}", file_path, e);
                if let Some(reply_chan) = msg.reply_to() {
                    let reply = Message::new(vec![Value::String(format!("Error: {:?}", e))], None);
                    let _ = reply_chan.send(reply).await;
                }
            }
        }
    }

    /// Handler for "list-dir" command.
    async fn handle_list_dir(&self, msg: &Message) {
        if msg.terms().len() < 2 {
            error!("list-dir: missing directory path");
            return;
        }
        let dir_path = match &msg.terms()[1] {
            Value::String(s) => s.clone(),
            other => {
                error!("list-dir: directory path is not a string: {:?}", other);
                return;
            }
        };

        match fs::read_dir(&dir_path).await {
            Ok(read_dir) => {
                let mut stream = ReadDirStream::new(read_dir);
                let mut names = vec![];
                while let Some(entry) = stream.next().await {
                    match entry {
                        Ok(e) => {
                            if let Some(name) = e.file_name().to_str() {
                                names.push(Value::String(name.to_string()));
                            }
                        }
                        Err(e) => {
                            error!("list-dir: error reading entry: {:?}", e);
                        }
                    }
                }
                if let Some(reply_chan) = msg.reply_to() {
                    let reply = Message::new(vec![Value::List(names)], None);
                    let _ = reply_chan.send(reply).await;
                }
            }
            Err(e) => {
                error!("list-dir: error reading directory {}: {:?}", dir_path, e);
                if let Some(reply_chan) = msg.reply_to() {
                    let reply = Message::new(vec![Value::String(format!("Error: {:?}", e))], None);
                    let _ = reply_chan.send(reply).await;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl AgentLifecycle for FsAgent {
    async fn get_scope(&self) -> Arc<Mutex<Scope>> {
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
impl AgentBehavior for FsAgent {
    async fn handle_message(&self, msg: Message) -> bool {
        if let Some(cmd) = msg.first_word() {
            match cmd.as_str() {
                "read-all" => {
                    self.handle_read_all(&msg).await;
                    return true;
                }
                "read-all-binary" => {
                    self.handle_read_all_binary(&msg).await;
                    return true;
                }
                "list-dir" => {
                    self.handle_list_dir(&msg).await;
                    return true;
                }
                other => {
                    warn!("FsAgent: unknown command: {:?}", other);
                    return true;
                }
            }
        }
        true
    }
}

impl Agent for FsAgent {}
