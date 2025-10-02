use std::sync::Arc;

use app_net::Socket;
use dashmap::DashMap;

pub struct AppNetworkNode {
    pub socket: Arc<Socket>,
}

impl AppNetworkNode {
    #[inline]
    pub fn new(socket: Arc<Socket>) -> Self {
        Self { socket }
    }

    #[inline]
    pub fn new_shared(socket: Arc<Socket>) -> Arc<Self> {
        Arc::new(Self::new(socket))
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
