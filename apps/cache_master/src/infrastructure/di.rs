use std::sync::Arc;

use app_core::clock::AppClock;

use crate::{
    core::usecases::{AssignNodeUseCase, GetKeyUseCase, PutKeyUseCase, RemoveNodeUseCase},
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
    pub get_key_use_case: Arc<GetKeyUseCase>,
    pub put_key_use_case: Arc<PutKeyUseCase>,
}

impl CacheMasterModule {
    pub fn build_from_state(app_state: Arc<AppState>) -> Self {
        let consistent_hasher_service = Arc::new(DashmapConsistentHasherService::new());
        let tcp_network_service = Arc::new(TcpNetworkService::from_state(
            app_state.network_state.clone(),
        ));
        let clock = Arc::new(AppClock::new());

        let assign_node_use_case = Arc::new(AssignNodeUseCase::new(
            consistent_hasher_service.clone(),
            tcp_network_service.clone(),
        ));

        let delete_node_use_case = Arc::new(crate::core::usecases::RemoveNodeUseCase::new(
            consistent_hasher_service.clone(),
            tcp_network_service.clone(),
        ));

        let get_key_use_case = Arc::new(GetKeyUseCase::new(
            consistent_hasher_service.clone(),
            tcp_network_service.clone(),
        ));

        let put_key_use_case = Arc::new(PutKeyUseCase::new(
            consistent_hasher_service,
            tcp_network_service.clone(),
            clock.clone(),
        ));

        Self {
            assign_node_use_case,
            tcp_network_service,
            delete_node_use_case,
            get_key_use_case,
            put_key_use_case,
        }
    }
}
