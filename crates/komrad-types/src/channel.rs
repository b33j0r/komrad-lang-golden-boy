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

#[derive(Debug)]
pub struct ChannelListener {
    pub address: Address,
    pub receiver: mpsc::Receiver<Msg>,
}

impl Display for ChannelListener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChannelListener({})", self.address)
    }
}
