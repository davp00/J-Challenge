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

#[cfg(test)]
mod tests {
    use crate::{
        core::{domain::models::Response, usecases::exec_put},
        test_mocks::cache_service_mock::MockCache,
    };

    #[tokio::test]
    async fn exec_put_returns_empty_when_key_is_empty() {
        let cache = MockCache::new();
        let resp = exec_put(&cache, "".into(), "value".into(), None).await;

        match resp {
            Response::Empty => {}
            _ => panic!("Expected Response::Empty"),
        }

        // Nada debería haberse guardado
        assert!(cache.store.lock().is_empty());
    }

    #[tokio::test]
    async fn exec_put_returns_empty_when_value_is_empty() {
        let cache = MockCache::new();
        let resp = exec_put(&cache, "key".into(), "".into(), None).await;

        match resp {
            Response::Empty => {}
            _ => panic!("Expected Response::Empty"),
        }

        assert!(cache.store.lock().is_empty());
    }

    #[tokio::test]
    async fn exec_put_stores_value_and_returns_okempty() {
        let cache = MockCache::new();
        let resp = exec_put(&cache, "key".into(), "value".into(), None).await;

        match resp {
            Response::OkEmpty => {}
            _ => panic!("Expected Response::OkEmpty"),
        }

        let stored = cache.store.lock();
        assert_eq!(stored.get("key"), Some(&"value".to_string()));
    }
}
