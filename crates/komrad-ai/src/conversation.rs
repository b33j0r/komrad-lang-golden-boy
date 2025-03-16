use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Timeframe {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: String,
    pub timestamp: Timeframe,
    pub events: Vec<ConversationEvent>,
}

#[derive(Debug, Clone)]
pub enum ConversationEvent {
    Message(ConversationMessage),
}

#[derive(Debug, Clone)]
pub enum ParticipantRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct Participant {
    pub id: String,
    pub name: String,
    pub role: ParticipantRole,
}

#[derive(Debug, Clone)]
pub struct ConversationMessage {
    pub id: String,
    pub timestamp: Timeframe,
    pub sender: Arc<Participant>,
    pub content: Vec<ConversationContent>,
}

#[derive(Debug, Clone)]
pub enum ConversationContent {
    Text(String),
    Code(String),
    Image(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_creation() {
        let participant = Arc::new(Participant {
            id: "user1".to_string(),
            name: "Alice".to_string(),
            role: ParticipantRole::User,
        });

        let message = ConversationMessage {
            id: "msg1".to_string(),
            timestamp: Timeframe {
                start: chrono::Utc::now(),
                end: None,
            },
            sender: participant.clone(),
            content: vec![ConversationContent::Text("Hello, world!".to_string())],
        };

        let conversation = Conversation {
            id: "conv1".to_string(),
            timestamp: Timeframe {
                start: chrono::Utc::now(),
                end: None,
            },
            events: vec![ConversationEvent::Message(message)],
        };

        assert_eq!(conversation.id, "conv1");
        assert_eq!(conversation.events.len(), 1);
    }
}
