use std::str::FromStr;

use app_net::SocketError;

#[derive(Debug, Clone)]
pub enum NodeKind {
    Master(String),
    Replica(String),
    Client(String),
}

impl FromStr for NodeKind {
    type Err = SocketError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split_whitespace();

        match (parts.next(), parts.next()) {
            (Some("MASTER"), Some(id)) => Ok(NodeKind::Master(id.to_string())),
            (Some("REPLICA"), Some(id)) => Ok(NodeKind::Replica(id.to_string())),
            (Some(id), None) => Ok(NodeKind::Client(id.to_string())),
            _ => Err(SocketError::BadRequest("Node type not found".to_string())),
        }
    }
}
