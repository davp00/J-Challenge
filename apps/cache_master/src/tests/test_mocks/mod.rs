use async_trait::async_trait;
use parking_lot::Mutex;

use crate::core::domain::{
    models::AppError,
    services::{ConsistentHasherService, NetworkService},
};

pub struct MockHasher {
    // respuestas configurables
    pub add_node_result: bool,
    pub node_exists_result: bool,

    // para inspección de llamadas
    pub last_add_node: Mutex<Option<String>>,
    pub last_node_exists: Mutex<Option<String>>,
    pub last_remove_node: Mutex<Option<String>>, // <-- NUEVO
}

impl MockHasher {
    pub fn new() -> Self {
        Self {
            add_node_result: true,
            node_exists_result: true,
            last_add_node: Mutex::new(None),
            last_node_exists: Mutex::new(None),
            last_remove_node: Mutex::new(None), // <-- NUEVO
        }
    }

    pub fn with_exists(exists: bool) -> Self {
        Self {
            node_exists_result: exists,
            ..Self::new()
        }
    }
}

impl ConsistentHasherService for MockHasher {
    fn create_hash(&self, _key: &str) -> String {
        "hash".into()
    }

    fn add_node(&self, node_id: &str) -> bool {
        *self.last_add_node.lock() = Some(node_id.to_string());
        self.add_node_result
    }

    fn remove_node(&self, node_id: &str) -> bool {
        *self.last_remove_node.lock() = Some(node_id.to_string()); // <-- track
        true
    }

    fn node_exists(&self, node_id: &str) -> bool {
        *self.last_node_exists.lock() = Some(node_id.to_string());
        self.node_exists_result
    }

    fn get_node_id_from_hash(&self, _hash: &str) -> Option<String> {
        None
    }
}

pub struct MockNetwork {
    // respuestas configurables
    pub next_master_for_replica: Mutex<Option<String>>,
    pub add_master_result: Mutex<Result<bool, AppError>>,
    pub add_replica_result: Mutex<Result<bool, AppError>>,

    pub replica_count: Mutex<usize>,
    pub remove_result: Mutex<Result<bool, AppError>>,

    // para inspección de llamadas
    pub last_add_master: Mutex<Option<String>>,
    pub last_add_replica: Mutex<Option<(String, String)>>,
    pub last_remove_node: Mutex<Option<String>>, // <-- NUEVO
}

impl MockNetwork {
    pub fn new() -> Self {
        Self {
            next_master_for_replica: Mutex::new(None),
            add_master_result: Mutex::new(Ok(true)),
            add_replica_result: Mutex::new(Ok(true)),
            replica_count: Mutex::new(0),
            remove_result: Mutex::new(Ok(true)),
            last_add_master: Mutex::new(None),
            last_add_replica: Mutex::new(None),
            last_remove_node: Mutex::new(None), // <-- NUEVO
        }
    }

    // helpers para configurar
    pub fn set_next_master(&self, id: Option<&str>) {
        *self.next_master_for_replica.lock() = id.map(|s| s.to_string());
    }
    pub fn set_add_master_result(&self, r: Result<bool, AppError>) {
        *self.add_master_result.lock() = r;
    }
    pub fn set_add_replica_result(&self, r: Result<bool, AppError>) {
        *self.add_replica_result.lock() = r;
    }
    pub fn set_replica_count(&self, count: usize) {
        *self.replica_count.lock() = count;
    }
    pub fn set_remove_result(&self, r: Result<bool, AppError>) {
        *self.remove_result.lock() = r;
    }
}

#[async_trait]
impl NetworkService for MockNetwork {
    fn get_node_id_with_less_replicas(&self) -> Option<String> {
        self.next_master_for_replica.lock().clone()
    }

    fn get_all_nodes_by_id(&self, _node_id: &str) -> Vec<String> {
        vec![]
    }

    async fn add_master_node(&self, node_id: &str) -> Result<bool, AppError> {
        *self.last_add_master.lock() = Some(node_id.to_string());
        self.add_master_result.lock().clone()
    }

    async fn add_replica_node(
        &self,
        master_node_id: &str,
        node_id: &str,
    ) -> Result<bool, AppError> {
        *self.last_add_replica.lock() = Some((master_node_id.to_string(), node_id.to_string()));
        self.add_replica_result.lock().clone()
    }

    async fn remove_node(&self, node_id: &str) -> Result<bool, AppError> {
        *self.last_remove_node.lock() = Some(node_id.to_string()); // <-- track
        self.remove_result.lock().clone()
    }

    fn count_replica_nodes(&self, _node_id: &str) -> usize {
        *self.replica_count.lock()
    }

    async fn request_put_key(
        &self,
        _node_id: &str,
        _key: &str,
        _value: &str,
        _ttl: Option<u64>,
    ) -> Result<bool, AppError> {
        Ok(true)
    }

    async fn request_get_key(
        &self,
        _node_id: &str,
        _key: &str,
    ) -> Result<Option<String>, AppError> {
        Ok(None)
    }
}
