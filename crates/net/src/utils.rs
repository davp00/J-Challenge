use crate::error::SocketError;

pub fn split_once_space(input: &str) -> Result<(&str, &str), SocketError> {
    input
        .split_once(' ')
        .ok_or_else(|| SocketError::BadMessage(input.to_string()))
}
