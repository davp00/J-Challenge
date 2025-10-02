#[derive(Debug)]
pub struct PutKeyUseCaseInput {
    pub key: String,
    pub value: String,
    pub ttl: Option<u64>,
}

#[derive(Debug)]
pub struct PutKeyUseCaseOutput {
    pub success: bool,
}
