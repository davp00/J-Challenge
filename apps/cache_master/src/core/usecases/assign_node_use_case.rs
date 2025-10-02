use std::sync::Arc;

use app_core::{UseCase, UseCaseValidatable};
use async_trait::async_trait;
use tracing::info;

use crate::core::domain::{
    models::{
        AppError, NodeType,
        usecases::assign_node_use_case::{AssignNodeUseCaseInput, AssignNodeUseCaseOutput},
    },
    services::{ConsistentHasherService, NetworkService},
};

pub struct AssignNodeUseCase {
    hasher_service: Arc<dyn ConsistentHasherService>,
    network_service: Arc<dyn NetworkService>,
}

impl AssignNodeUseCase {
    pub fn new(
        hasher_service: Arc<dyn ConsistentHasherService>,
        network_service: Arc<dyn NetworkService>,
    ) -> Self {
        Self {
            hasher_service,
            network_service,
        }
    }

    async fn handle_master_insert(
        &self,
        input: AssignNodeUseCaseInput,
    ) -> Result<AssignNodeUseCaseOutput, AppError> {
        self.hasher_service.add_node(&input.node_id);

        if !self.hasher_service.node_exists(&input.node_id) {
            return Err(AppError::ConnectionError(
                "No se pudo agregar el nodo".to_string(),
            ));
        }

        let success = self.network_service.add_master_node(&input.node_id).await?;

        Ok(AssignNodeUseCaseOutput { success })
    }

    async fn handle_replica_insert(
        &self,
        input: AssignNodeUseCaseInput,
    ) -> Result<AssignNodeUseCaseOutput, AppError> {
        let possible_master_node_id = self.network_service.get_node_id_with_less_replicas();

        match possible_master_node_id {
            Some(master_node_id) => {
                let success = self
                    .network_service
                    .add_replica_node(master_node_id.as_str(), &input.node_id)
                    .await?;

                Ok(AssignNodeUseCaseOutput { success })
            }
            None => Err(AppError::ConnectionError(
                "No hay nodos en la red".to_string(),
            )),
        }
    }
}

#[async_trait]
impl UseCase<AssignNodeUseCaseInput, AssignNodeUseCaseOutput, AppError> for AssignNodeUseCase {
    async fn execute(
        &self,
        input: AssignNodeUseCaseInput,
    ) -> Result<AssignNodeUseCaseOutput, AppError> {
        info!("New Node: {:?}", input);
        match input.node_type {
            NodeType::Master => self.handle_master_insert(input).await,
            NodeType::Replica => self.handle_replica_insert(input).await,
            _ => Err(AppError::ConnectionError(
                "Nodo sin identificador".to_string(),
            )),
        }
    }
}

#[async_trait]
impl UseCaseValidatable<AssignNodeUseCaseInput, AssignNodeUseCaseOutput, AppError>
    for AssignNodeUseCase
{
    async fn validate(&self, input: &AssignNodeUseCaseInput) -> Result<(), AppError> {
        if input.node_id.is_empty() {
            return Err(AppError::FirstConnectionEmpty);
        }

        Ok(())
    }
}
