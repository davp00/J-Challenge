#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Ping,
    Put {
        key: String,
        value: String,
        ttl: Option<u64>,
    },
    Get {
        key: String,
    },
    Unknown(String),
}
