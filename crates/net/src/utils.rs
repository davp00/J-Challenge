use crate::error::SocketError;

pub fn split_once_space(input: &str) -> Result<(&str, &str), SocketError> {
    input
        .split_once(' ')
        .ok_or_else(|| SocketError::BadMessage(input.to_string()))
}

pub fn split_message(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut i = 0;
    let bytes = input.as_bytes();

    while i < bytes.len() {
        // saltar espacios
        while i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }

        if i >= bytes.len() {
            break;
        }

        if bytes[i] == b'"' {
            // token con comillas - buscar la comilla de cierre
            let start = i + 1; // después de la comilla inicial
            i += 1;
            let mut depth = 1;

            while i < bytes.len() {
                if bytes[i] == b'"' {
                    // verificar si es la comilla de cierre (seguida por espacio o fin)
                    if i + 1 >= bytes.len() || bytes[i + 1] == b' ' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                }
                i += 1;
            }

            parts.push(&input[start..i]);
            i += 1; // saltar la comilla de cierre
        } else {
            // token sin comillas
            let start = i;
            while i < bytes.len() && bytes[i] != b' ' && bytes[i] != b'"' {
                i += 1;
            }
            parts.push(&input[start..i]);
        }
    }

    parts
}
