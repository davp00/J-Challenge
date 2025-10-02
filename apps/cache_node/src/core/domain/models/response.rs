pub enum Response {
    OkEmpty,
    OkValue(String),
    Pong,
    Echo(String),
    Empty,
    Error(String),
}

impl Response {
    pub fn to_wire(&self) -> String {
        match self {
            Response::Pong => "pong".to_string(),
            Response::OkEmpty => "".to_string(),
            Response::OkValue(v) => format!("{}", v),
            Response::Echo(s) => format!("echo:{s}"),
            Response::Empty => "EMPTY".to_string(),
            Response::Error(e) => format!("ERROR: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::domain::models::Response;

    #[test]
    fn response_to_wire_variants() {
        assert_eq!(Response::Pong.to_wire(), "pong");
        assert_eq!(Response::OkEmpty.to_wire(), "");
        assert_eq!(Response::OkValue("abc".into()).to_wire(), "abc");
        assert_eq!(Response::Echo("x".into()).to_wire(), "echo:x");
        assert_eq!(Response::Empty.to_wire(), "EMPTY");
        assert_eq!(Response::Error("boom".into()).to_wire(), "ERROR: boom");
    }
}
