mod rnode;

use axum::{
    extract::State,
    routing::get,
    Json,
    Router,
};

use rnode::{NetworkStatus, RNodeClient};

use serde::Serialize;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    rnode: Arc<RNodeClient>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "rchain-sentinel",
        version: "0.1.0",
    })
}

async fn network_status(
    State(state): State<AppState>,
) -> Json<NetworkStatus> {
    Json(state.rnode.check().await)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let rnode_url = std::env::var("RCHAIN_RNODE_URL")
        .unwrap_or_else(|_| "http://localhost:40403".to_string());

    let state = AppState {
        rnode: Arc::new(RNodeClient::new(rnode_url)),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/network/status", get(network_status))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let address = "0.0.0.0:8080";

    println!("RChain Sentinel backend listening on {}", address);
    println!("RNode target: {}", rnode_url);

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("failed to bind server");

    axum::serve(listener, app)
        .await
        .expect("server failed");
}
