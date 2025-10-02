#[derive(Debug)]
pub struct RemoveNodeUseCaseInput {
    pub node_id: String,
}

#[derive(Debug)]
pub struct RemoveNodeUseCaseOutput {
    pub success: bool,
}
