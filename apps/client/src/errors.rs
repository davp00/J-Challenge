use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Socket error: {0}")]
    SocketError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),
}
