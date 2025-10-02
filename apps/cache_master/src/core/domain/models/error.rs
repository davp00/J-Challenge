use core::error;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Socket error: {0}")]
    SocketError(String),

    #[error("Primera conexión sin datos")]
    FirstConnectionEmpty,

    #[error("Connection Error: {0}")]
    ConnectionError(String),
}
