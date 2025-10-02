use std::{f32::consts::E, sync::Arc, time::Instant};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::{client::CacheClient, errors::AppError};

#[derive(Clone)]
pub struct AppState {
    pub client: Arc<CacheClient>,
}

#[derive(Deserialize)]
pub struct PutBody {
    value: String,
    #[serde(default)]
    ttl: Option<u64>,
}

#[derive(Serialize)]
pub struct PingResponse {
    message: String,
    elapsed_ms: u128,
}

#[derive(Serialize)]
pub struct PutResponse {
    key: String,
    elapsed_ms: u128,
}

#[derive(Serialize)]
pub struct GetResponse {
    key: String,
    value: Option<String>,
    elapsed_ms: u128,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        error!("AppError: {self:?}");
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

pub async fn ping(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let start = Instant::now();
    let response = state.client.request_raw("PING", "").await?;

    if !response.is_success() {
        return Err(AppError::ConnectionError(format!(
            "PING failed: {}",
            response.payload
        )));
    }

    let elapsed_ms = start.elapsed().as_millis();

    Ok((
        StatusCode::OK,
        Json(PingResponse {
            message: response.payload,
            elapsed_ms,
        }),
    ))
}

pub async fn put_kv(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(body): Json<PutBody>,
) -> Result<impl IntoResponse, AppError> {
    let start = Instant::now();
    let response = state.client.put(&key, &body.value, body.ttl).await?;
    let elapsed_ms = start.elapsed().as_millis();

    if !response.is_success() {
        return Err(AppError::ConnectionError(format!(
            "PUT failed: {}",
            response.payload
        )));
    }

    Ok((StatusCode::OK, Json(PutResponse { key, elapsed_ms })))
}

pub async fn get_kv(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let start = Instant::now();
    let response = state.client.get(&key).await?;
    let elapsed_ms = start.elapsed().as_millis();

    if !response.is_success() {
        return Err(AppError::ConnectionError(format!(
            "GET failed: {}",
            response.payload
        )));
    }

    Ok((
        StatusCode::OK,
        Json(GetResponse {
            key,
            value: Some(response.payload),
            elapsed_ms,
        }),
    ))
}
