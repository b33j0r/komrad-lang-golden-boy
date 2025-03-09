use crate::types::uuid7;
use std::fmt::Display;
use std::net::IpAddr;
use uuid::Uuid;

/// Addresses are used to identify channels across serialization boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Address {
    Named(String),
    UUID(uuid7::Uuid7),
    Ip { ip: IpAddr, port: u16, uuid: Uuid },
}

impl Address {
    pub fn new_named(name: String) -> Self {
        Self::Named(name)
    }

    pub fn new_uuid(uuid: uuid7::Uuid7) -> Self {
        Self::UUID(uuid)
    }

    pub fn new_ip(ip: IpAddr, port: u16, uuid: Uuid) -> Self {
        Self::Ip { ip, port, uuid }
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Address::Named(n) => write!(f, "{}", n),
            Address::UUID(u) => write!(f, "{}", u),
            Address::Ip { ip, port, uuid } => write!(f, "{}:{}/{}", ip, port, uuid),
        }
    }
}
