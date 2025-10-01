use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Socket error: {0}")]
    SocketError(String),
}

#[derive(Debug, Error)]
pub enum SocketError {
    #[error("Canal de escritura cerrado para socket {0}")]
    WriteChannelClosed(String),

    #[error("Timeout esperando respuesta (socket {socket_id}, req_id {req_id})")]
    Timeout { socket_id: String, req_id: String },

    #[error("Canal de respuesta cerrado (socket {socket_id}, req_id {req_id})")]
    ResponseChannelClosed { socket_id: String, req_id: String },

    #[error("Mensaje mal formado: {0}")]
    BadMessage(String),
}
