use std::sync::Arc;

use async_trait::async_trait;
use dashmap::{DashMap, Entry};

use crate::{
    core::domain::{models::AppError, services::NetworkService},
    infrastructure::app_state::{AppNetworkNode, AppNetworkState},
};

pub struct TcpNetworkService {
    network_state: Arc<AppNetworkState>,
    nodes: DashMap<Arc<str>, DashMap<Arc<str>, Arc<AppNetworkNode>>>,
}

impl TcpNetworkService {
    #[inline]
    pub fn from_state(network_state: Arc<AppNetworkState>) -> Self {
        Self {
            network_state,
            nodes: DashMap::new(),
        }
    }

    #[inline]
    fn ensure_shard(
        &self,
        master_id: &str,
    ) -> dashmap::mapref::one::RefMut<'_, Arc<str>, DashMap<Arc<str>, Arc<AppNetworkNode>>> {
        self.nodes.entry(Arc::<str>::from(master_id)).or_default()
    }

    #[inline]
    fn resolve_node(&self, node_id: &str) -> Result<Arc<AppNetworkNode>, AppError> {
        self.network_state
            .nodes_registry
            .get(node_id)
            .map(|r| r.value().clone())
            .ok_or_else(|| {
                AppError::ConnectionError(format!("Nodo no existe en registry: {node_id}"))
            })
    }

    #[inline]
    fn get_shard(
        &self,
        master_id: &str,
    ) -> Option<dashmap::mapref::one::Ref<'_, Arc<str>, DashMap<Arc<str>, Arc<AppNetworkNode>>>>
    {
        self.nodes.get(master_id)
    }
}

#[async_trait]
impl NetworkService for TcpNetworkService {
    fn get_node_id_with_less_replicas(&self) -> Option<String> {
        self.nodes
            .iter()
            .min_by_key(|entry| entry.value().len())
            .map(|entry| entry.key().to_string())
    }

    fn get_all_nodes_by_id(&self, node_id: &str) -> Vec<String> {
        todo!()
    }

    async fn add_master_node(&self, node_id: &str) -> Result<bool, AppError> {
        if node_id.is_empty() {
            return Err(AppError::ConnectionError("Nodo maestro sin id".to_string()));
        }

        let node_arc = self.resolve_node(node_id)?;

        let shard = self.ensure_shard(node_id);

        match shard.entry(Arc::<str>::from(node_id)) {
            Entry::Occupied(_) => Ok(false), // ya estaba como master en su shard
            Entry::Vacant(v) => {
                v.insert(node_arc);
                Ok(true)
            }
        }
    }

    async fn add_replica_node(
        &self,
        master_node_id: &str,
        node_id: &str,
    ) -> Result<bool, AppError> {
        let replica_arc = self.resolve_node(node_id)?;

        let Some(shard) = self.get_shard(master_node_id) else {
            return Err(AppError::ConnectionError(format!(
                "Shard del master no existe: {master_node_id}"
            )));
        };

        match shard.entry(Arc::<str>::from(node_id)) {
            Entry::Occupied(_) => {
                println!("Node {node_id} already exists in {master_node_id}");
                Ok(false)
            }
            Entry::Vacant(v) => {
                v.insert(replica_arc);
                Ok(true)
            }
        }
    }
}
