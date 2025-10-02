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
