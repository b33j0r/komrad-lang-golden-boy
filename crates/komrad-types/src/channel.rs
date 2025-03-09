use crate::address::Address;
use crate::Msg;
use std::fmt::Display;
use std::hash::Hash;
use tokio::sync::mpsc;

/// Channels are first-class citizens in the Komrad language.
#[derive(Debug, Clone)]
pub struct Channel {
    pub address: Address,
    pub sender: mpsc::Sender<Msg>,
}

impl Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Channel({})", self.address)
    }
}

impl PartialEq for Channel {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}

impl Eq for Channel {}

impl Hash for Channel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.address.hash(state);
    }
}

/// The receiving end of a listener, used internally to dispatch a message to a handler.
///
/// Listeners are an internal struct that is used to receive messages from a channel.
/// They are just about the only thing in this execution flow that isn't a Value.
#[derive(Debug)]
pub struct ChannelListener {
    address: Address,
    receiver: mpsc::Receiver<Msg>,
}

impl ChannelListener {
    pub fn new(address: Address, receiver: mpsc::Receiver<Msg>) -> Self {
        Self { address, receiver }
    }

    pub fn address(&self) -> &Address {
        &self.address
    }

    pub async fn recv(&mut self) -> Option<Msg> {
        self.receiver.recv().await
    }
}

impl Display for ChannelListener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChannelListener({})", self.address)
    }
}
