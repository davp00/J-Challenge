use std::sync::Arc;

use async_trait::async_trait;

use crate::core::{domain::services::CacheService, services::Cache};

pub struct InMemCache {
    cache: Arc<Cache<String, String>>,
}

impl InMemCache {
    pub fn new() -> Self {
        Self {
            cache: Cache::new(),
        }
    }
}

#[async_trait]
impl CacheService for InMemCache {
    async fn put(&self, key: String, value: String, ttl: Option<u64>) {
        self.cache.put(key, value, ttl);
    }
    async fn get(&self, key: &String) -> Option<String> {
        self.cache.get(&key).map(|entry| (*entry).clone())
    }
}
