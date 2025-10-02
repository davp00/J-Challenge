use std::sync::Arc;

use app_core::{UseCase, UseCaseValidatable};
use async_trait::async_trait;

use crate::core::domain::{
    models::{
        AppError,
        usecases::{PutKeyUseCaseInput, PutKeyUseCaseOutput},
    },
    services::{ConsistentHasherService, NetworkService},
};

pub struct PutKeyUseCase {
    hasher_service: Arc<dyn ConsistentHasherService>,
    network_service: Arc<dyn NetworkService>,
}

impl PutKeyUseCase {
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
impl UseCase<PutKeyUseCaseInput, PutKeyUseCaseOutput, AppError> for PutKeyUseCase {
    async fn execute(&self, input: PutKeyUseCaseInput) -> Result<PutKeyUseCaseOutput, AppError> {
        let hash = self.hasher_service.create_hash(&input.key);
        println!("Hash for key {}: {}", input.key, hash);

        let node_id_option = self.hasher_service.get_node_id_from_hash(&hash);

        if node_id_option.is_none() {
            return Err(AppError::NodeNotFound(format!(
                "No node found for key {} with hash {} on PUT",
                input.key, hash
            )));
        }

        let node_id = node_id_option.unwrap();
        println!("Node ID for key {}: {}", input.key, node_id);

        let put_result = self
            .network_service
            .request_put_key(&node_id, &input.key, &input.value, input.ttl)
            .await?;

        Ok(PutKeyUseCaseOutput {
            success: put_result,
        })
    }
}

#[async_trait]
impl UseCaseValidatable<PutKeyUseCaseInput, PutKeyUseCaseOutput, AppError> for PutKeyUseCase {
    async fn validate(&self, input: &PutKeyUseCaseInput) -> Result<(), AppError> {
        if input.key.is_empty() {
            return Err(AppError::BadRequest("Key is empty".to_string()));
        }

        if input.value.is_empty() {
            return Err(AppError::BadRequest("Value is empty".to_string()));
        }

        Ok(())
    }
}
