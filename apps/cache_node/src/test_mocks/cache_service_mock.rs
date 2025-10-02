use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use parking_lot::Mutex;

use crate::core::domain::services::CacheService;

pub struct MockCache {
    pub store: Arc<Mutex<HashMap<String, String>>>,
}

impl MockCache {
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl CacheService for MockCache {
    async fn put(&self, key: String, value: String, _ttl: Option<u64>) {
        self.store.lock().insert(key, value);
    }

    async fn get(&self, key: &String) -> Option<String> {
        self.store.lock().get(key).cloned()
    }
}
