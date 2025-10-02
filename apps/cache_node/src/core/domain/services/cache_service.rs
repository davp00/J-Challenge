use async_trait::async_trait;

#[async_trait]
pub trait CacheService: Send + Sync {
    async fn put(&self, key: String, value: String, ttl: Option<u64>);
    async fn get(&self, key: &String) -> Option<String>;
}
