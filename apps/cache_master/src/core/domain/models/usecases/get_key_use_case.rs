#[derive(Debug)]
pub struct GetKeyUseCaseInput {
    pub key: String,
}

#[derive(Debug)]
pub struct GetKeyUseCaseOutput {
    pub success: bool,
    pub result: String,
}
