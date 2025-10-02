use std::sync::Arc;

use app_core::{UseCaseValidatable, utils::split_message};

use crate::{
    core::domain::models::{
        AppError,
        usecases::{GetKeyUseCaseInput, PutKeyUseCaseInput},
    },
    infrastructure::di::CacheMasterModule,
};

pub struct RequestController {
    module_dependencies: Arc<CacheMasterModule>,
}

impl RequestController {
    pub fn new(module_dependencies: Arc<CacheMasterModule>) -> Self {
        Self {
            module_dependencies,
        }
    }
}

impl RequestController {
    pub async fn handle_request(&self, action: &str, payload: &str) -> Result<String, AppError> {
        let mut parts = split_message(payload).into_iter();

        match action {
            "PING" => Ok(String::from("PONG")),
            "PUT" => {
                let key = parts.next().unwrap_or_default().to_string();
                let value = parts.next().unwrap_or_default().to_string();
                let ttl = parts.next().and_then(|s| s.parse::<u64>().ok());

                let response = self
                    .module_dependencies
                    .put_key_use_case
                    .validate_and_execute(PutKeyUseCaseInput { key, value, ttl })
                    .await?;

                if !response.success {
                    return Err(AppError::BadRequest("Failed to put key".to_string()));
                }

                Ok("OK".to_string())
            }
            "GET" => {
                let key = parts.next().unwrap_or_default().to_string();

                let response = self
                    .module_dependencies
                    .get_key_use_case
                    .validate_and_execute(GetKeyUseCaseInput { key })
                    .await?;

                if !response.success {
                    return Err(AppError::BadRequest("Key not found".to_string()));
                }

                Ok(response.result)
            }
            _ => Err(AppError::BadRequest(format!("Unknown action: {}", action))),
        }
    }
}
