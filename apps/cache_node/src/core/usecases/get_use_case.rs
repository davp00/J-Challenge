use crate::core::domain::{models::Response, services::CacheService};

pub async fn exec_get<C: CacheService>(cache: &C, key: String) -> Response {
    if key.is_empty() {
        return Response::Empty;
    }
    match cache.get(&key).await {
        Some(v) => Response::OkValue(v),
        None => Response::OkEmpty,
    }
}
