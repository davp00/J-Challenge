use std::sync::Arc;

use crate::{
    core::services::request_controller_service::RequestControllerService,
    infrastructure::adapters::services::cache_service::InMemCache,
};

pub struct CacheNodeModule {
    pub request_controller_service: Arc<RequestControllerService<InMemCache>>,
}

impl CacheNodeModule {
    pub fn init_dependencies() -> Self {
        let cache = Arc::new(InMemCache::new());
        let request_controller_service = Arc::new(RequestControllerService::new(cache));

        Self {
            request_controller_service,
        }
    }
}
