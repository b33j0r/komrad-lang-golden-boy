use crate::error::RuntimeError;
use crate::message::Message;
use crate::prelude::Value;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

const CHANNEL_DIGEST_LEN: usize = 8;

pub enum ControlMessage {
    Stop,
}

#[derive(Clone)]
pub struct Channel {
    uuid: Uuid,
    sender: mpsc::Sender<Message>,
    control_sender: mpsc::Sender<ControlMessage>,
}

impl Channel {
    pub async fn send_and_recv(&self, msg: Message) -> Result<Message, RuntimeError> {
        let (response_sender, response_receiver) = Channel::new(1);
        let msg_with_response = Message::new(msg.terms().clone(), Some(response_sender));
        self.send(msg_with_response).await?;
        response_receiver.recv().await
    }

    pub async fn get(&self, key: &str) -> Result<Value, RuntimeError> {
        let msg = Message::new(
            vec![
                Value::Word("get".to_string()),
                Value::String(key.to_string()),
            ],
            None,
        );
        let reply = self.send_and_recv(msg).await;
        match reply {
            Ok(msg) => Ok(msg.terms().iter().next().unwrap_or(&Value::Empty).clone()),
            Err(e) => Err(e),
        }
    }

    pub async fn items(&self) -> Result<Message, RuntimeError> {
        let msg = Message::new(vec![Value::Word("items".to_string())], None);
        self.send_and_recv(msg).await
    }
}

impl std::fmt::Debug for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.uuid.to_string();
        write!(
            f,
            "Channel({})",
            s[s.len() - CHANNEL_DIGEST_LEN..].to_string()
        )
    }
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.uuid.to_string();
        write!(
            f,
            "Channel({})",
            s[s.len() - CHANNEL_DIGEST_LEN..].to_string()
        )
    }
}

impl PartialEq for Channel {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

#[derive(Debug)]
pub struct ChannelListener {
    uuid: Uuid,
    // Separate the receivers into two distinct Mutexes
    message_receiver: Mutex<mpsc::Receiver<Message>>,
    control_receiver: Mutex<mpsc::Receiver<ControlMessage>>,
}

impl Channel {
    pub fn new(capacity: usize) -> (Self, ChannelListener) {
        let (sender, message_receiver) = mpsc::channel(capacity);
        let (control_sender, control_receiver) = mpsc::channel(capacity);
        let uuid = Uuid::now_v7();
        (
            Channel {
                uuid,
                sender: sender.clone(),
                control_sender: control_sender.clone(),
            },
            ChannelListener {
                uuid,
                message_receiver: Mutex::new(message_receiver),
                control_receiver: Mutex::new(control_receiver),
            },
        )
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub async fn send(&self, message: Message) -> Result<(), RuntimeError> {
        self.sender
            .send(message)
            .await
            .map_err(|_| RuntimeError::SendError)
    }

    pub async fn control(&self, message: ControlMessage) -> Result<(), RuntimeError> {
        self.control_sender
            .send(message)
            .await
            .map_err(|_| RuntimeError::SendControlError)
    }
}

impl ChannelListener {
    pub async fn recv(&self) -> Result<Message, RuntimeError> {
        let mut receiver = self.message_receiver.lock().await;
        receiver.recv().await.ok_or(RuntimeError::ReceiveError)
    }

    pub async fn recv_control(&self) -> Result<ControlMessage, RuntimeError> {
        let mut receiver = self.control_receiver.lock().await;
        receiver
            .recv()
            .await
            .ok_or(RuntimeError::ReceiveControlError)
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }
}
