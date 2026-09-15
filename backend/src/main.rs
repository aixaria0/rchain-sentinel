mod block_verification;
mod casper_evidence;
mod cross_node;
mod models;
mod rnode;
mod verification;

use axum::{extract::State, response::Html, routing::get, Json, Router};

use block_verification::BlockVerificationEngine;
use casper_evidence::CasperEvidenceEngine;
use cross_node::CrossNodeVerificationEngine;
use models::{CasperEvidenceReport, CrossNodeReport, ExplorerBlockReport, FinalizedBlockEvidence, HealthResponse, NetworkStatus, VerificationReport};
use rnode::RNodeClient;
use verification::VerificationEngine;

use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    rnode: Arc<RNodeClient>,
    rnode_urls: Arc<Vec<String>>,
}

async fn explorer() -> Html<&'static str> {
    Html(include_str!("../console.html"))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok", service: "rchain-sentinel", version: "0.1.0" })
}

async fn network_status(State(state): State<AppState>) -> Json<NetworkStatus> {
    Json(state.rnode.status().await)
}

async fn finalized_block_evidence(State(state): State<AppState>) -> Json<FinalizedBlockEvidence> {
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
    Json(VerificationEngine::verify_network(&network_status, &finalized_block))
}

async fn verify_block(State(state): State<AppState>) -> Json<VerificationReport> {
    let network_status = state.rnode.status().await;
    let evidence = match state.rnode.fetch_last_finalized_block_evidence().await {
        Ok(evidence) => evidence,
        Err(error) => FinalizedBlockEvidence::unavailable(error),
    };
    Json(BlockVerificationEngine::verify(&evidence, &network_status.node_url))
}

async fn verify_casper(State(state): State<AppState>) -> Json<CasperEvidenceReport> {
    let evidence = match state.rnode.fetch_last_finalized_block_evidence().await {
        Ok(evidence) => evidence,
        Err(error) => FinalizedBlockEvidence::unavailable(error),
    };
    Json(CasperEvidenceEngine::analyze(&evidence))
}

async fn verify_cross_node(State(state): State<AppState>) -> Json<CrossNodeReport> {
    Json(CrossNodeVerificationEngine::verify(&state.rnode_urls).await)
}

async fn explorer_block(State(state): State<AppState>) -> Json<ExplorerBlockReport> {
    let network = state.rnode.status().await;
    let block = match state.rnode.fetch_last_finalized_block_evidence().await {
        Ok(evidence) => evidence,
        Err(error) => FinalizedBlockEvidence::unavailable(error),
    };
    let block_verification = BlockVerificationEngine::verify(&block, &network.node_url);
    let casper = CasperEvidenceEngine::analyze(&block);
    let cross_node = CrossNodeVerificationEngine::verify(&state.rnode_urls).await;

    let mut explanation = Vec::new();
    if let Some(height) = network.rnode.as_ref().and_then(|s| s.last_finalized_block_number) {
        explanation.push(format!("RNode reports finalized height {}.", height));
    }
    if let Some(hash) = &block.block_hash {
        explanation.push(format!("Observed block identity: {}.", hash));
    } else {
        explanation.push("Block hash was not present in the observed payload.".to_string());
    }
    if let Some(proposer) = &block.proposer {
        explanation.push(format!("Observed proposer/validator field: {}.", proposer));
    }
    if casper.bond_structure_valid {
        explanation.push(format!("Observed {} structurally valid validator bonds with total observed stake {:?}.", casper.bond_count, casper.total_observed_stake));
    } else {
        explanation.push("Validator bond evidence is incomplete or structurally inconsistent.".to_string());
    }
    if casper.justification_structure_valid {
        explanation.push(format!("Observed {} structurally valid justification references.", casper.justification_count));
    } else {
        explanation.push("Justification evidence is incomplete or structurally inconsistent.".to_string());
    }
    explanation.push(format!("Cross-node observation: {}/{} reachable nodes agree; observed node-count quorum is {}.", cross_node.agreeing_nodes, cross_node.reachable_count, if cross_node.quorum_observed { "met" } else { "not met" }));
    explanation.push("This explanation is evidence-based and does not claim Casper finality unless authenticated protocol semantics establish it.".to_string());

    Json(ExplorerBlockReport { network, block, block_verification, casper, cross_node, explanation })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let rnode_url = std::env::var("RCHAIN_RNODE_URL").unwrap_or_else(|_| "http://localhost:40403".to_string());
    let rnode_urls = std::env::var("RCHAIN_RNODE_URLS")
        .ok()
        .map(|value| value.split(',').map(str::trim).filter(|url| !url.is_empty()).map(ToOwned::to_owned).collect::<Vec<_>>())
        .filter(|urls| !urls.is_empty())
        .unwrap_or_else(|| vec![rnode_url.clone()]);

    println!("RChain Sentinel");
    println!("RNode target: {}", rnode_url);
    println!("Cross-node targets: {}", rnode_urls.len());

    let state = AppState { rnode: Arc::new(RNodeClient::new(rnode_url)), rnode_urls: Arc::new(rnode_urls) };
    let app = Router::new()
        .route("/", get(explorer))
        .route("/health", get(health))
        .route("/api/network/status", get(network_status))
        .route("/api/evidence/last-finalized-block", get(finalized_block_evidence))
        .route("/api/verify", get(verify_network))
        .route("/api/verify/block", get(verify_block))
        .route("/api/verify/casper", get(verify_casper))
        .route("/api/verify/cross-node", get(verify_cross_node))
        .route("/api/explorer/block", get(explorer_block))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.expect("failed to bind server");
    println!("Listening on http://0.0.0.0:8080");
    axum::serve(listener, app).await.expect("server failed");
}
