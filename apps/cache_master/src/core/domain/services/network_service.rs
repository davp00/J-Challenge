use async_trait::async_trait;

use crate::core::domain::models::AppError;

#[async_trait]
pub trait NetworkService: Send + Sync {
    fn get_node_id_with_less_replicas(&self) -> Option<String>;

    fn get_all_nodes_by_id(&self, node_id: &str) -> Vec<String>;

    async fn add_master_node(&self, node_id: &str) -> Result<bool, AppError>;

    async fn add_replica_node(&self, master_node_id: &str, node_id: &str)
    -> Result<bool, AppError>;

    async fn remove_node(&self, node_id: &str) -> Result<bool, AppError>;

    fn count_replica_nodes(&self, node_id: &str) -> usize;

    async fn request_put_key(
        &self,
        node_id: &str,
        key: &str,
        value: &str,
        ttl: Option<u64>,
    ) -> Result<bool, AppError>;

    async fn request_get_key(&self, node_id: &str, key: &str) -> Result<Option<String>, AppError>;
}
