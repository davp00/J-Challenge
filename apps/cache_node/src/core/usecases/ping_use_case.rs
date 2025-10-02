use crate::core::domain::models::Response;

pub async fn exec_ping() -> Response {
    Response::Pong
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::domain::models::Response;

    #[tokio::test]
    async fn exec_ping_returns_pong() {
        let resp = exec_ping().await;
        match resp {
            Response::Pong => {}
            _ => panic!("Expected Response::Pong"),
        }
    }

    #[tokio::test]
    async fn exec_ping_to_wire_is_pong_string() {
        let resp = exec_ping().await;
        assert_eq!(resp.to_wire(), "pong");
    }
}
