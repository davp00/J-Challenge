use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::core::domain::{
    models::AppError,
    services::{ConsistentHasherService, NetworkService},
};
use app_core::clock::{AppTime, Clock};

// ----------------- MockHasher -----------------

pub struct MockHasher {
    // respuestas configurables
    pub add_node_result: bool,
    pub node_exists_result: bool,

    // nodo que devolverá get_node_id_from_hash
    pub node_for_hash: Mutex<Option<String>>,

    // tracking
    pub last_add_node: Mutex<Option<String>>,
    pub last_node_exists: Mutex<Option<String>>,
    pub last_remove_node: Mutex<Option<String>>,
}

impl MockHasher {
    pub fn new() -> Self {
        Self {
            add_node_result: true,
            node_exists_result: true,
            node_for_hash: Mutex::new(None),
            last_add_node: Mutex::new(None),
            last_node_exists: Mutex::new(None),
            last_remove_node: Mutex::new(None),
        }
    }

    pub fn with_exists(exists: bool) -> Self {
        Self {
            node_exists_result: exists,
            ..Self::new()
        }
    }

    pub fn set_node_for_hash(&self, node_id: Option<&str>) {
        *self.node_for_hash.lock() = node_id.map(|s| s.to_string());
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
        *self.last_remove_node.lock() = Some(node_id.to_string());
        true
    }
    fn node_exists(&self, node_id: &str) -> bool {
        *self.last_node_exists.lock() = Some(node_id.to_string());
        self.node_exists_result
    }
    fn get_node_id_from_hash(&self, _hash: &str) -> Option<String> {
        self.node_for_hash.lock().clone()
    }
}

// ----------------- MockNetwork -----------------

pub struct MockNetwork {
    // configurables
    pub next_master_for_replica: Mutex<Option<String>>,
    pub add_master_result: Mutex<Result<bool, AppError>>,
    pub add_replica_result: Mutex<Result<bool, AppError>>,
    pub replica_count: Mutex<usize>,
    pub remove_result: Mutex<Result<bool, AppError>>,

    // GET
    pub request_get_key_result: Mutex<Result<Option<String>, AppError>>,

    // PUT
    pub request_put_key_result: Mutex<Result<bool, AppError>>,

    // tracking
    pub last_add_master: Mutex<Option<String>>,
    pub last_add_replica: Mutex<Option<(String, String)>>,
    pub last_remove_node: Mutex<Option<String>>,
    pub last_request_get: Mutex<Option<(String, String)>>,
    pub last_request_put: Mutex<Option<(String, String, String, Option<u64>)>>,
}

impl MockNetwork {
    pub fn new() -> Self {
        Self {
            next_master_for_replica: Mutex::new(None),
            add_master_result: Mutex::new(Ok(true)),
            add_replica_result: Mutex::new(Ok(true)),
            replica_count: Mutex::new(0),
            remove_result: Mutex::new(Ok(true)),
            request_get_key_result: Mutex::new(Ok(None)),
            request_put_key_result: Mutex::new(Ok(true)),
            last_add_master: Mutex::new(None),
            last_add_replica: Mutex::new(None),
            last_remove_node: Mutex::new(None),
            last_request_get: Mutex::new(None),
            last_request_put: Mutex::new(None),
        }
    }

    // setters helpers
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
    pub fn set_request_get_key_result(&self, r: Result<Option<String>, AppError>) {
        *self.request_get_key_result.lock() = r;
    }
    pub fn set_request_put_key_result(&self, r: Result<bool, AppError>) {
        *self.request_put_key_result.lock() = r;
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
        *self.last_remove_node.lock() = Some(node_id.to_string());
        self.remove_result.lock().clone()
    }

    fn count_replica_nodes(&self, _node_id: &str) -> usize {
        *self.replica_count.lock()
    }

    async fn request_put_key(
        &self,
        node_id: &str,
        key: &str,
        value: &str,
        ttl: Option<u64>,
    ) -> Result<bool, AppError> {
        *self.last_request_put.lock() =
            Some((node_id.to_string(), key.to_string(), value.to_string(), ttl));
        self.request_put_key_result.lock().clone()
    }

    async fn request_get_key(&self, node_id: &str, key: &str) -> Result<Option<String>, AppError> {
        *self.last_request_get.lock() = Some((node_id.to_string(), key.to_string()));
        self.request_get_key_result.lock().clone()
    }
}

// ----------------- MockClock -----------------

pub struct MockClock {
    pub now_ms: AtomicU64,
}

impl MockClock {
    pub fn new(initial_ms: u64) -> Self {
        Self {
            now_ms: AtomicU64::new(initial_ms),
        }
    }
    pub fn set_now(&self, ms: u64) {
        self.now_ms.store(ms, Ordering::SeqCst);
    }
}

impl Clock for MockClock {
    fn now_millis(&self) -> AppTime {
        AppTime::new(self.now_ms.load(Ordering::SeqCst))
    }
}
