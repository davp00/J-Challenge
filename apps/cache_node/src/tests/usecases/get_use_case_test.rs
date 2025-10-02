#[cfg(test)]
mod tests {
    use crate::{
        core::{
            domain::{models::Response, services::CacheService},
            usecases::exec_get,
        },
        tests::test_mocks::cache_service_mock::MockCache,
    };

    //------ Tests de exec_get --------

    #[tokio::test]
    async fn exec_get_returns_empty_when_key_is_empty() {
        let cache = MockCache::new();
        let resp = exec_get(&cache, "".to_string()).await;
        match resp {
            Response::Empty => {}
            _ => panic!("Expected Response::Empty"),
        }
    }

    #[tokio::test]
    async fn exec_get_returns_okvalue_when_found() {
        let cache = MockCache::new();
        cache.put("k".into(), "v".into(), None).await;

        let resp = exec_get(&cache, "k".to_string()).await;
        match resp {
            Response::OkValue(v) => assert_eq!(v, "v"),
            _ => panic!("Expected OkValue"),
        }
    }

    #[tokio::test]
    async fn exec_get_returns_okempty_when_missing() {
        let cache = MockCache::new();
        let resp = exec_get(&cache, "missing".to_string()).await;
        match resp {
            Response::OkEmpty => {}
            _ => panic!("Expected OkEmpty"),
        }
    }
}
