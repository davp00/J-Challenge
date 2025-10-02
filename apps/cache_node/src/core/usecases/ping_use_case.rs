use crate::core::domain::models::Response;

pub async fn exec_ping() -> Response {
    Response::Pong
}
