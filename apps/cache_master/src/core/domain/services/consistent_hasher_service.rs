pub trait ConsistentHasherService: Send + Sync {
    fn create_hash(&self, key: &str) -> String;

    fn add_node(&self, node_id: &str) -> bool;

    fn remove_node(&self, node_id: &str) -> bool;

    fn node_exists(&self, node_id: &str) -> bool;

    fn get_node_id_from_hash(&self, hash: &str) -> Option<String>;
}
