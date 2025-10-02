// src/app/controller.rs
use std::sync::Arc;

use crate::core::{
    domain::{
        models::{Command, Response},
        services::CacheService,
    },
    usecases::{exec_get, exec_ping, exec_put},
};

pub struct RequestControllerService<C: CacheService> {
    cache: Arc<C>,
}

impl<C: CacheService> RequestControllerService<C> {
    pub fn new(cache: Arc<C>) -> Self {
        Self { cache }
    }

    pub async fn handle(&self, cmd: Command) -> Response {
        match cmd {
            Command::Ping => exec_ping().await,
            Command::Put { key, value, ttl } => {
                exec_put(self.cache.as_ref(), key, value, ttl).await
            }
            Command::Get { key } => exec_get(self.cache.as_ref(), key).await,
            Command::Unknown(other) => Response::Echo(other),
        }
    }
}
