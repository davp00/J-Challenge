use uuid::Uuid;

pub fn generate_short_id(len: usize) -> String {
    Uuid::new_v4()
        .to_string()
        .replace('-', "")
        .chars()
        .take(len)
        .collect()
}

pub fn split_message(input: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = Vec::with_capacity(4);
    let bytes = input.as_bytes();
    let mut i = 0usize;

    let skip_spaces = |mut j: usize| {
        while j < bytes.len() && bytes[j] == b' ' {
            j += 1;
        }
        j
    };

    let read_token = |mut i: usize| -> Option<(usize, usize, usize)> {
        i = skip_spaces(i);
        if i >= bytes.len() {
            return None;
        }

        if bytes[i] == b'"' {
            let start = i + 1;
            i += 1;
            let mut prev = b'\0';
            while i < bytes.len() {
                let b = bytes[i];
                if b == b'"' && prev != b'\\' {
                    let end = i;
                    i += 1;
                    return Some((start, end, i));
                }
                prev = b;
                i += 1;
            }
            Some((start, bytes.len(), bytes.len()))
        } else {
            let start = i;
            while i < bytes.len() && bytes[i] != b' ' {
                i += 1;
            }
            Some((start, i, i))
        }
    };

    // 1) Cabecera: 3 tokens
    while parts.len() < 3 {
        match read_token(i) {
            Some((s, e, next)) => {
                parts.push(&input[s..e]);
                i = next;
            }
            None => break,
        }
    }

    // 2) Resto como payload único (sin comillas exteriores si las hay)
    i = skip_spaces(i);
    if i < bytes.len() {
        let mut end = bytes.len();
        while end > i && (bytes[end - 1] == b'\r' || bytes[end - 1] == b'\n') {
            end -= 1;
        }

        // recorta comillas exteriores opcionales
        let (mut s, mut e) = (i, end);
        if e > s && bytes[s] == b'"' && bytes[e - 1] == b'"' {
            s += 1;
            e -= 1;
        }

        parts.push(&input[s..e]);
    }

    parts
}

#[cfg(test)]
mod tests {
    use super::generate_short_id;

    // -------- generate_short_id --------

    #[test]
    fn id_respects_requested_length() {
        let id = generate_short_id(8);
        assert_eq!(id.len(), 8);

        let id = generate_short_id(16);
        assert_eq!(id.len(), 16);
    }

    #[test]
    fn id_has_no_dashes() {
        let id = generate_short_id(24);
        assert!(!id.contains('-'));
    }

    #[test]
    fn id_is_unique_enough_for_tests() {
        let a = generate_short_id(12);
        let b = generate_short_id(12);
        assert_ne!(a, b);
    }

    #[test]
    fn id_never_exceeds_32_when_requesting_more() {
        // UUID v4 sin guiones = 32 caracteres hex
        let id = generate_short_id(100);
        assert_eq!(id.len(), 32);
    }

    // -------- split_message --------
}
