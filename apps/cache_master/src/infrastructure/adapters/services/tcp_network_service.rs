use std::sync::Arc;

use app_net::RequestDataInput;
use async_trait::async_trait;
use dashmap::{DashMap, Entry};

use crate::{
    core::domain::{models::AppError, services::NetworkService},
    infrastructure::{
        adapters::services::request_all_race_first_abort_rest,
        app_state::{AppNetworkNode, AppNetworkState},
    },
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

    pub fn get_all_nodes(&self, node_id: &str) -> Vec<Arc<AppNetworkNode>> {
        let mut result = Vec::new();

        if let Some(shard) = self.get_shard(node_id) {
            for entry in shard.iter() {
                result.push(entry.value().clone());
            }
        }

        result
    }

    pub fn pretty_print(&self) {
        println!(
            "🚀 TcpNetworkService: {}",
            self.network_state.nodes_registry.len()
        );
        for master_entry in self.nodes.iter() {
            let master_id = master_entry.key();
            let replicas = master_entry.value();

            println!("  🌐 Master Node: {master_id}");

            if replicas.is_empty() {
                println!("    └─ (no replicas)");
            } else {
                for replica_entry in replicas.iter() {
                    let replica_id = replica_entry.key();
                    println!("    └─ 📦 Replica: {replica_id}");
                }
            }
        }
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

    fn get_all_nodes_by_id(&self, _: &str) -> Vec<String> {
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
                node_arc.set_master_id(node_id);
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
                replica_arc.set_master_id(master_node_id);
                v.insert(replica_arc);
                Ok(true)
            }
        }
    }

    async fn remove_node(&self, node_id: &str) -> Result<bool, AppError> {
        let mut removed_topology = false;
        let node_arc = self
            .network_state
            .nodes_registry
            .get(node_id)
            .map(|r| r.value().clone());

        if let Some(node) = node_arc {
            match node.get_master_id() {
                // Réplica: vive dentro del shard de su master
                Some(master_id) => {
                    if let Some(shard) = self.nodes.get_mut(master_id.as_ref())
                        && shard.remove(node_id).is_some()
                    {
                        removed_topology = true;
                        node.master_id.write().take();

                        let empty = shard.is_empty();
                        drop(shard); // liberar lock del shard

                        if empty {
                            self.nodes.remove(master_id.as_ref());
                        }
                    }
                }
                // Master: su shard es su propio node_id
                None => {
                    //TODO: Manejar mucho mejor este caso
                    return Err(AppError::ConnectionError(
                        "Es un nodo perdido según nuestra logica :)".to_string(),
                    ));
                }
            }
        } else {
            // No está en registry: intentamos barrer shards por si quedó colgado ahí
            for mut shard in self.nodes.iter_mut() {
                if shard.value_mut().remove(node_id).is_some() {
                    removed_topology = true;
                    break;
                }
            }
            // O quizá era un master con shard vacío
            if !removed_topology {
                removed_topology |= self.nodes.remove(node_id).is_some();
            }
        }

        // Remover SIEMPRE del registry (si existe)
        let removed_registry = self.network_state.nodes_registry.remove(node_id).is_some();

        Ok(removed_topology || removed_registry)
    }

    async fn request_put_key(
        &self,
        node_id: &str,
        key: &str,
        value: &str,
    ) -> Result<bool, AppError> {
        let request = RequestDataInput {
            action: "PUT",
            payload: &format!("{} \"{}\"", key, value),
        };

        let nodes = self.get_all_nodes(node_id);

        let response = request_all_race_first_abort_rest(&nodes, request)
            .await
            .map_err(|e| AppError::ConnectionError(e.to_string()))?;

        if response.is_success() {
            return Ok(true);
        }

        Err(AppError::ConnectionError(format!(
            "Error en PUT: {} {}",
            response.code, response.payload
        )))
    }

    async fn request_get_key(&self, node_id: &str, key: &str) -> Result<Option<String>, AppError> {
        let request = RequestDataInput {
            action: "GET",
            payload: key,
        };

        let nodes = self.get_all_nodes(node_id);

        let response = request_all_race_first_abort_rest(&nodes, request)
            .await
            .map_err(|e| AppError::ConnectionError(e.to_string()))?;

        if response.is_success() {
            return Ok(Some(response.payload));
        }

        Ok(None)
    }
}
