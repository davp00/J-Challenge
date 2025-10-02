use crate::core::domain::models::NodeType;

#[derive(Debug)]
pub struct AssignNodeUseCaseInput {
    pub node_id: String,
    pub node_type: NodeType,
}

#[derive(Debug)]
pub struct AssignNodeUseCaseOutput {
    pub success: bool,
}
