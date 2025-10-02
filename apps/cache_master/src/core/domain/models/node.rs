use std::str::FromStr;

use crate::core::domain::models::AppError;

#[derive(Debug)]
pub enum NodeType {
    Master,
    Replica,
    Client,
}

#[derive(Debug)]
pub struct EntryNode {
    pub node_type: NodeType,
    pub id: String,
}

impl EntryNode {
    #[inline]
    pub fn new(node_type: NodeType, id: String) -> Self {
        Self { node_type, id }
    }
}

impl FromStr for EntryNode {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split_whitespace();

        match (parts.next(), parts.next()) {
            (Some("MASTER"), Some(id)) => Ok(EntryNode::new(NodeType::Master, id.to_string())),
            (Some("REPLICA"), Some(id)) => Ok(EntryNode::new(NodeType::Replica, id.to_string())),
            (Some(id), None) => Ok(EntryNode::new(NodeType::Client, id.to_string())),
            _ => Err(AppError::ConnectionError("Node type not found".to_string())),
        }
    }
}
