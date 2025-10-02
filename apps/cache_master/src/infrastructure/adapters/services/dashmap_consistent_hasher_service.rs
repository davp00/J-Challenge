use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

use dashmap::{DashMap, Entry};
use parking_lot::RwLock;

use crate::core::domain::services::ConsistentHasherService;

const VNODE_REPLICAS: usize = 128;

pub struct DashmapConsistentHasherService {
    ring: RwLock<BTreeMap<u64, Arc<str>>>,
    real_nodes: DashMap<Arc<str>, ()>,
    vnodes: usize,
}

impl DashmapConsistentHasherService {
    pub fn new() -> Self {
        Self {
            ring: RwLock::new(BTreeMap::new()),
            real_nodes: DashMap::new(),
            vnodes: VNODE_REPLICAS,
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    #[inline]
    fn hash_u64(&self, key: &str) -> u64 {
        let mut h = DefaultHasher::new();
        key.hash(&mut h);
        h.finish()
    }

    fn insert_vnodes(&self, node_id: &Arc<str>) {
        let mut ring = self.ring.write();
        for i in 0..self.vnodes {
            let vnode_key = format!("{node_id}#{i}");
            let hv = self.hash_u64(&vnode_key);

            ring.insert(hv, node_id.clone());
        }
    }

    fn locate_node(&self, target: u64) -> Option<Arc<str>> {
        let ring = self.ring.read();
        if ring.is_empty() {
            return None;
        }

        if let Some((_, node)) = ring.range(target..).next() {
            return Some(node.clone());
        }

        ring.iter().next().map(|(_, node)| node.clone())
    }
}

impl ConsistentHasherService for DashmapConsistentHasherService {
    fn create_hash(&self, key: &str) -> String {
        let hv = self.hash_u64(key);
        format!("{:016x}", hv)
    }

    fn add_node(&self, node_id: &str) -> bool {
        let node_arc: Arc<str> = Arc::<str>::from(node_id);

        match self.real_nodes.entry(node_arc.clone()) {
            Entry::Occupied(_) => {
                // Ya existe -> no reinsertar vnodes
                false
            }
            Entry::Vacant(v) => {
                v.insert(());
                self.insert_vnodes(&node_arc);
                true
            }
        }
    }

    fn node_exists(&self, node_id: &str) -> bool {
        self.real_nodes.contains_key(node_id)
    }

    fn get_node_id_from_hash(&self, hash: &str) -> String {
        let parsed = if let Ok(v) = u64::from_str_radix(hash.trim_start_matches("0x"), 16) {
            v
        } else if let Ok(v) = hash.parse::<u64>() {
            v
        } else {
            return String::new();
        };

        match self.locate_node(parsed) {
            Some(node) => node.to_string(),
            None => String::new(),
        }
    }
}
