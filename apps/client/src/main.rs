use std::net::SocketAddr;

use axum::{
    Router,
    routing::{get, put},
};
use dotenvy::{dotenv, from_filename};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    client::{CacheClient, CacheClientConfig},
    errors::AppError,
    http::{AppState, get_kv, ping, put_kv},
};

pub mod client;
pub mod errors;
pub mod http;

fn load_env_for_workspace() {
    let _ = from_filename(concat!(env!("CARGO_MANIFEST_DIR"), "/.env"));
    let _ = from_filename(".env");
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenv().ok();

    load_env_for_workspace();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = CacheClientConfig::from_env()?;
    let client = CacheClient::connect_with(cfg).await?;

    let app = Router::new()
        .route("/ping", get(ping))
        .route("/kv/{key}", put(put_kv).get(get_kv))
        .with_state(AppState { client });

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    info!("HTTP server listening on http://{addr}");

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
