use std::sync::Arc;

use app_net::Socket;
use dashmap::DashMap;
use parking_lot::RwLock;

pub struct AppNetworkNode {
    pub master_id: RwLock<Option<Arc<str>>>,
    pub node_id: Arc<str>,
    pub socket: Arc<Socket>,
}

impl AppNetworkNode {
    #[inline]
    pub fn new(socket: Arc<Socket>, node_id: Arc<str>) -> Self {
        Self {
            socket,
            master_id: RwLock::new(None),
            node_id,
        }
    }

    #[inline]
    pub fn new_shared(socket: Arc<Socket>, node_id: Arc<str>) -> Arc<Self> {
        Arc::new(Self::new(socket, node_id))
    }

    pub fn set_master_id(&self, id: &str) {
        let mut g = self.master_id.write();
        *g = Some(Arc::<str>::from(id));
    }

    pub fn get_master_id(&self) -> Option<Arc<str>> {
        self.master_id.read().clone()
    }
}

pub struct AppNetworkState {
    pub nodes_registry: DashMap<Arc<str>, Arc<AppNetworkNode>>,
}

impl AppNetworkState {
    #[inline]
    pub fn new() -> Self {
        Self {
            nodes_registry: DashMap::new(),
        }
    }

    #[inline]
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

pub struct AppState {
    pub network_state: Arc<AppNetworkState>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            network_state: AppNetworkState::new_shared(),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }
}
