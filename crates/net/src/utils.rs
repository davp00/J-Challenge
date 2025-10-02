use crate::error::SocketError;

pub fn split_once_space(input: &str) -> Result<(&str, &str), SocketError> {
    input
        .split_once(' ')
        .ok_or_else(|| SocketError::BadMessage(input.to_string()))
}

pub fn split_message(input: &str) -> Vec<&str> {
    let mut in_quotes = false;
    let mut start = 0;
    let mut parts = Vec::new();

    for (i, c) in input.char_indices() {
        match c {
            '"' => {
                if in_quotes {
                    // fin de comillas -> cerramos token aquí
                    parts.push(&input[start..i]);
                }
                in_quotes = !in_quotes;
                start = i + 1; // saltamos la comilla
            }
            ' ' if !in_quotes => {
                if start != i {
                    parts.push(&input[start..i]);
                }
                start = i + 1;
            }
            _ => {}
        }
    }

    if start < input.len() {
        parts.push(&input[start..]);
    }

    parts
}
