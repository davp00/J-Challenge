use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Socket error: {0}")]
    SocketError(String),

    #[error("Error on socket reading: {0}")]
    SocketReadingError(String),
}
