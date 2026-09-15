mod models;
mod rnode;
mod verifier;
mod verification;

use axum::{
    extract::State,
    routing::get,
    Json,
    Router,
};

use models::{
    HealthResponse,
    NetworkStatus,
    VerificationReport,
};

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

async fn network_status(
    State(state): State<AppState>,
) -> Json<NetworkStatus> {
    Json(state.rnode.status().await)
}

async fn verify_network(
    State(state): State<AppState>,
) -> Json<VerificationReport> {
    let network_status = state.rnode.status().await;

    let report =
        VerificationEngine::verify_network(&network_status);

    Json(report)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let rnode_url = std::env::var("RCHAIN_RNODE_URL")
        .unwrap_or_else(|_| {
            "http://localhost:40403".to_string()
        });

    println!("RChain Sentinel");
    println!("RNode target: {}", rnode_url);

    let state = AppState {
        rnode: Arc::new(
            RNodeClient::new(rnode_url)
        ),
    };

    let app = Router::new()
        .route(
            "/health",
            get(health),
        )
        .route(
            "/api/network/status",
            get(network_status),
        )
        .route(
            "/api/verify",
            get(verify_network),
        )
        .with_state(state)
        .layer(CorsLayer::permissive());

    let listener =
        tokio::net::TcpListener::bind("0.0.0.0:8080")
            .await
            .expect("failed to bind server");

    println!(
        "Listening on http://0.0.0.0:8080"
    );

    axum::serve(listener, app)
        .await
        .expect("server failed");
}
