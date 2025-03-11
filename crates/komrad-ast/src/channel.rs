use crate::error::RuntimeError;
use crate::message::Message;
use tokio::sync::mpsc;
use uuid::Uuid;

const CHANNEL_DIGEST_LEN: usize = 8;

#[derive(Clone)]
pub struct Channel {
    uuid: Uuid,
    sender: mpsc::Sender<Message>,
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

impl PartialEq for Channel {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

#[derive(Debug)]
pub struct ChannelListener {
    uuid: Uuid,
    receiver: mpsc::Receiver<Message>,
}

impl Channel {
    pub fn new(capacity: usize) -> (Self, ChannelListener) {
        let (sender, receiver) = mpsc::channel(capacity);
        let uuid = Uuid::now_v7();
        (
            Channel {
                uuid,
                sender: sender.clone(),
            },
            ChannelListener { uuid, receiver },
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
}

impl ChannelListener {
    pub async fn recv(&mut self) -> Result<Message, RuntimeError> {
        self.receiver.recv().await.ok_or(RuntimeError::ReceiveError)
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;
    use crate::value::Value;

    #[tokio::test]
    async fn test_channel_basic_send_receive() {
        let (channel, mut listener) = Channel::new(10);

        let msg = Message::new(vec![Value::String("Hello".into())], None);
        channel
            .send(msg.clone())
            .await
            .expect("Failed to send message");

        let received = listener.recv().await.expect("Failed to receive message");
        assert_eq!(
            received, msg,
            "Sent and received messages should be identical"
        );
    }

    #[tokio::test]
    async fn test_channel_fifo_order() {
        let (channel, mut listener) = Channel::new(10);

        let msg1 = Message::new(vec![Value::String("First".into())], None);
        let msg2 = Message::new(vec![Value::String("Second".into())], None);

        channel
            .send(msg1.clone())
            .await
            .expect("Failed to send first message");
        channel
            .send(msg2.clone())
            .await
            .expect("Failed to send second message");

        let received1 = listener
            .recv()
            .await
            .expect("Failed to receive first message");
        let received2 = listener
            .recv()
            .await
            .expect("Failed to receive second message");

        assert_eq!(received1, msg1, "First message should be received first");
        assert_eq!(received2, msg2, "Second message should be received second");
    }

    #[tokio::test]
    async fn test_channel_multiple_senders() {
        let (channel, mut listener) = Channel::new(10);

        let channel2 = channel.clone();

        let msg1 = Message::new(vec![Value::String("From channel 1".into())], None);
        let msg2 = Message::new(vec![Value::String("From channel 2".into())], None);

        channel
            .send(msg1.clone())
            .await
            .expect("Channel 1 failed to send");
        channel2
            .send(msg2.clone())
            .await
            .expect("Channel 2 failed to send");

        let received1 = listener.recv().await.expect("Failed to receive message 1");
        let received2 = listener.recv().await.expect("Failed to receive message 2");

        assert!(
            received1 == msg1 || received1 == msg2,
            "Unexpected message order"
        );
        assert!(
            received2 == msg1 || received2 == msg2,
            "Unexpected message order"
        );
        assert_ne!(received1, received2, "Messages should not be duplicated");
    }

    #[tokio::test]
    async fn test_channel_receive_after_closure() {
        let (channel, mut listener) = Channel::new(10);

        let msg = Message::new(vec![Value::String("Test".into())], None);
        channel
            .send(msg.clone())
            .await
            .expect("Failed to send message");

        drop(channel); // Closing the sender

        let received = listener
            .recv()
            .await
            .expect("Should still receive first message");
        assert_eq!(
            received, msg,
            "Message should be correctly received before closure"
        );

        let err = listener.recv().await;
        assert!(
            matches!(err, Err(RuntimeError::ReceiveError)),
            "Receiver should return an error after closure"
        );
    }

    #[tokio::test]
    async fn test_channel_send_after_closure() {
        let (channel, listener) = Channel::new(1);

        drop(listener); // Closing the receiver

        let msg = Message::new(vec![Value::String("Lost Message".into())], None);
        let result = channel.send(msg.clone()).await;
        assert!(
            matches!(result, Err(RuntimeError::SendError)),
            "Sending to a closed channel should return an error"
        );
    }
}
