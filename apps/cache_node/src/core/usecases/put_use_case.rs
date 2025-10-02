use tracing::trace;

use crate::core::domain::{models::Response, services::CacheService};

pub async fn exec_put<C: CacheService>(
    cache: &C,
    key: String,
    value: String,
    ttl: Option<u64>,
) -> Response {
    if key.is_empty() || value.is_empty() {
        return Response::Empty;
    }

    trace!("Putting key: {}, value: {}, ttl: {:?}", key, value, ttl);

    cache.put(key, value, ttl).await;

    Response::OkEmpty
}
