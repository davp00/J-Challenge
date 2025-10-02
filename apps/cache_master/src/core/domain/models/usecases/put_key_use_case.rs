#[derive(Debug)]
pub struct PutKeyUseCaseInput {
    pub key: String,
    pub value: String,
}

#[derive(Debug)]
pub struct PutKeyUseCaseOutput {
    pub success: bool,
}
