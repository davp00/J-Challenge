use std::sync::Arc;

use app_core::{UseCase, UseCaseValidatable};
use async_trait::async_trait;

use crate::core::domain::{
    models::{
        AppError,
        usecases::remove_node_use_case::{RemoveNodeUseCaseInput, RemoveNodeUseCaseOutput},
    },
    services::{ConsistentHasherService, NetworkService},
};

pub struct RemoveNodeUseCase {
    hasher_service: Arc<dyn ConsistentHasherService>,
    network_service: Arc<dyn NetworkService>,
}

impl RemoveNodeUseCase {
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
impl UseCase<RemoveNodeUseCaseInput, RemoveNodeUseCaseOutput, AppError> for RemoveNodeUseCase {
    async fn execute(
        &self,
        input: RemoveNodeUseCaseInput,
    ) -> Result<RemoveNodeUseCaseOutput, AppError> {
        let node_id = input.node_id.as_ref();

        let hasher_service_remove_result = self.hasher_service.remove_node(node_id);

        println!(
            "Remove node result from hasher service: {node_id} {hasher_service_remove_result}"
        );
        if !hasher_service_remove_result {
            return Err(AppError::NodeNotFound(format!(
                "{node_id} in hasher service",
            )));
        }

        let network_service_remove_result = self.network_service.remove_node(node_id).await?;

        println!(
            "Remove node result from network service: {node_id} {network_service_remove_result}"
        );

        if !network_service_remove_result {
            return Err(AppError::NodeNotFound(format!(
                "{node_id} in network service",
            )));
        }

        Ok(RemoveNodeUseCaseOutput { success: true })
    }
}

#[async_trait]
impl UseCaseValidatable<RemoveNodeUseCaseInput, RemoveNodeUseCaseOutput, AppError>
    for RemoveNodeUseCase
{
    async fn validate(&self, input: &RemoveNodeUseCaseInput) -> Result<(), AppError> {
        if input.node_id.is_empty() {
            return Err(AppError::FirstConnectionEmpty);
        }

        Ok(())
    }
}
