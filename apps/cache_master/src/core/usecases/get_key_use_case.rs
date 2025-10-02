use std::sync::Arc;

use app_core::{UseCase, UseCaseValidatable};
use async_trait::async_trait;
use tracing::trace;

use crate::core::domain::{
    models::{
        AppError,
        usecases::{GetKeyUseCaseInput, GetKeyUseCaseOutput},
    },
    services::{ConsistentHasherService, NetworkService},
};

pub struct GetKeyUseCase {
    hasher_service: Arc<dyn ConsistentHasherService>,
    network_service: Arc<dyn NetworkService>,
}

impl GetKeyUseCase {
    pub fn new(
        hasher_service: Arc<dyn ConsistentHasherService>,
        network_service: Arc<dyn NetworkService>,
    ) -> Self {
        Self {
            hasher_service,
            network_service,
        }
    }
}

#[async_trait]
impl UseCase<GetKeyUseCaseInput, GetKeyUseCaseOutput, AppError> for GetKeyUseCase {
    async fn execute(&self, input: GetKeyUseCaseInput) -> Result<GetKeyUseCaseOutput, AppError> {
        let hash = self.hasher_service.create_hash(&input.key);
        trace!("Hash for key {}: {}", input.key, hash);

        let node_id_option = self.hasher_service.get_node_id_from_hash(&hash);

        if node_id_option.is_none() {
            return Err(AppError::NodeNotFound(format!(
                "No node found for key {} with hash {}",
                input.key, hash
            )));
        }

        let node_id = node_id_option.unwrap();

        trace!("Node ID for key {}: {}", input.key, node_id);

        let get_result = self
            .network_service
            .request_get_key(&node_id, &input.key)
            .await?;

        Ok(GetKeyUseCaseOutput {
            success: true,
            result: get_result.unwrap_or_default(),
        }) // TODO: Implement actual logic
    }
}

#[async_trait]
impl UseCaseValidatable<GetKeyUseCaseInput, GetKeyUseCaseOutput, AppError> for GetKeyUseCase {
    async fn validate(&self, input: &GetKeyUseCaseInput) -> Result<(), AppError> {
        if input.key.is_empty() {
            return Err(AppError::BadRequest("Key is empty".to_string()));
        }

        Ok(())
    }
}
