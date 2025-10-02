use std::sync::Arc;

use app_core::{
    UseCase, UseCaseValidatable,
    clock::{AppTime, Clock},
};
use async_trait::async_trait;
use tracing::trace;

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
    clock: Arc<dyn Clock>,
}

impl PutKeyUseCase {
    pub fn new(
        hasher_service: Arc<dyn ConsistentHasherService>,
        network_service: Arc<dyn NetworkService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            hasher_service,
            network_service,
            clock,
        }
    }
}

#[async_trait]
impl UseCase<PutKeyUseCaseInput, PutKeyUseCaseOutput, AppError> for PutKeyUseCase {
    async fn execute(&self, input: PutKeyUseCaseInput) -> Result<PutKeyUseCaseOutput, AppError> {
        let hash = self.hasher_service.create_hash(&input.key);

        let node_id_option = self.hasher_service.get_node_id_from_hash(&hash);

        if node_id_option.is_none() {
            return Err(AppError::NodeNotFound(format!(
                "No node found for key {} with hash {} on PUT",
                input.key, hash
            )));
        }

        let node_id = node_id_option.unwrap();
        trace!("Node ID {} for key: {} {:?}", node_id, input.key, input.ttl);

        let expires_at = match input.ttl {
            Some(ttl_ms) => Some(self.clock.now_millis().as_millis_u64() + ttl_ms),
            None => None,
        };

        let put_result = self
            .network_service
            .request_put_key(&node_id, &input.key, &input.value, expires_at)
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
