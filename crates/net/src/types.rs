use crate::error::SocketError;

pub type ReqId = String;

pub type SocketResult<T> = Result<T, SocketError>;
