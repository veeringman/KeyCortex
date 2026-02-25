use axum::{Json, Router, routing::get};
use serde::Serialize;
use std::net::SocketAddr;
use tracing::info;

#[derive(Debug, Serialize)]
struct HealthResponse {
    service: &'static str,
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct VersionResponse {
    service: &'static str,
    version: &'static str,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let app = Router::new()
        .route("/health", get(health))
        .route("/version", get(version));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("wallet-service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "wallet-service",
        status: "ok",
    })
}

async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        service: "wallet-service",
        version: env!("CARGO_PKG_VERSION"),
    })
}
