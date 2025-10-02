use std::sync::Arc;

use crate::{
    core::usecases::{AssignNodeUseCase, RemoveNodeUseCase},
    infrastructure::{
        adapters::services::{
            dashmap_consistent_hasher_service::DashmapConsistentHasherService,
            tcp_network_service::TcpNetworkService,
        },
        app_state::AppState,
    },
};

pub struct CacheMasterModule {
    pub tcp_network_service: Arc<TcpNetworkService>,
    pub assign_node_use_case: Arc<AssignNodeUseCase>,
    pub delete_node_use_case: Arc<RemoveNodeUseCase>,
}

impl CacheMasterModule {
    pub fn build_from_state(app_state: Arc<AppState>) -> Self {
        let consistent_hasher_service = Arc::new(DashmapConsistentHasherService::new());
        let tcp_network_service = Arc::new(TcpNetworkService::from_state(
            app_state.network_state.clone(),
        ));

        let assign_node_use_case = Arc::new(AssignNodeUseCase::new(
            consistent_hasher_service.clone(),
            tcp_network_service.clone(),
        ));

        let delete_node_use_case = Arc::new(crate::core::usecases::RemoveNodeUseCase::new(
            consistent_hasher_service.clone(),
            tcp_network_service.clone(),
        ));

        Self {
            assign_node_use_case,
            tcp_network_service,
            delete_node_use_case,
        }
    }
}
