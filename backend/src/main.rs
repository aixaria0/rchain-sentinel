mod block_verification;
mod models;
mod rnode;
mod verification;

use axum::{extract::State, routing::get, Json, Router};

use block_verification::BlockVerificationEngine;
use models::{FinalizedBlockEvidence, HealthResponse, NetworkStatus, VerificationReport};
use rnode::RNodeClient;
use verification::VerificationEngine;

use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    rnode: Arc<RNodeClient>,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "rchain-sentinel",
        version: "0.1.0",
    })
}

async fn network_status(State(state): State<AppState>) -> Json<NetworkStatus> {
    Json(state.rnode.status().await)
}

async fn finalized_block_evidence(
    State(state): State<AppState>,
) -> Json<FinalizedBlockEvidence> {
    let evidence = match state.rnode.fetch_last_finalized_block_evidence().await {
        Ok(evidence) => evidence,
        Err(error) => FinalizedBlockEvidence::unavailable(error),
    };

    Json(evidence)
}

async fn verify_network(State(state): State<AppState>) -> Json<VerificationReport> {
    let network_status = state.rnode.status().await;

    let finalized_block = match state.rnode.fetch_last_finalized_block_evidence().await {
        Ok(evidence) => evidence,
        Err(error) => FinalizedBlockEvidence::unavailable(error),
    };

    let report = VerificationEngine::verify_network(&network_status, &finalized_block);
    Json(report)
}

async fn verify_block(State(state): State<AppState>) -> Json<VerificationReport> {
    let evidence = match state.rnode.fetch_last_finalized_block_evidence().await {
        Ok(evidence) => evidence,
        Err(error) => FinalizedBlockEvidence::unavailable(error),
    };

    Json(BlockVerificationEngine::verify(&evidence, &state.rnode.target_url()))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let rnode_url = std::env::var("RCHAIN_RNODE_URL")
        .unwrap_or_else(|_| "http://localhost:40403".to_string());

    println!("RChain Sentinel");
    println!("RNode target: {}", rnode_url);

    let state = AppState {
        rnode: Arc::new(RNodeClient::new(rnode_url)),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/network/status", get(network_status))
        .route("/api/evidence/last-finalized-block", get(finalized_block_evidence))
        .route("/api/verify", get(verify_network))
        .route("/api/verify/block", get(verify_block))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("failed to bind server");

    println!("Listening on http://0.0.0.0:8080");

    axum::serve(listener, app)
        .await
        .expect("server failed");
}
