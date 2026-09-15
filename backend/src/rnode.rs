use crate::models::{NetworkStatus, RNodeStatusResponse};
use reqwest::Client;

#[derive(Clone)]
pub struct RNodeClient {
    client: Client,
    base_url: String,
}

impl RNodeClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    pub async fn status(&self) -> NetworkStatus {
        let start = std::time::Instant::now();
        let url = format!("{}/api/status", self.base_url);

        let response = match self.client.get(&url).send().await {
            Ok(response) => response,
            Err(error) => {
                return NetworkStatus {
                    reachable: false,
                    node_url: self.base_url.clone(),
                    latency_ms: None,
                    http_status: None,
                    error: Some(error.to_string()),
                    node_version: None,
                    network_id: None,
                    shard_id: None,
                    peers: None,
                    nodes: None,
                    native_token_name: None,
                    native_token_symbol: None,
                    native_token_decimals: None,
                    last_finalized_block: None,
                    is_validator: None,
                    is_read_only: None,
                    is_ready: None,
                    current_epoch: None,
                    epoch_length: None,
                };
            }
        };

        let latency_ms = start.elapsed().as_millis();
        let http_status = response.status().as_u16();

        if !response.status().is_success() {
            return NetworkStatus {
                reachable: false,
                node_url: self.base_url.clone(),
                latency_ms: Some(latency_ms),
                http_status: Some(http_status),
                error: Some(format!("RNode returned HTTP {}", http_status)),
                node_version: None,
                network_id: None,
                shard_id: None,
                peers: None,
                nodes: None,
                native_token_name: None,
                native_token_symbol: None,
                native_token_decimals: None,
                last_finalized_block: None,
                is_validator: None,
                is_read_only: None,
                is_ready: None,
                current_epoch: None,
                epoch_length: None,
            };
        }

        let payload = match response.json::<RNodeStatusResponse>().await {
            Ok(payload) => payload,
            Err(error) => {
                return NetworkStatus {
                    reachable: false,
                    node_url: self.base_url.clone(),
                    latency_ms: Some(latency_ms),
                    http_status: Some(http_status),
                    error: Some(format!(
                        "Invalid RNode status response: {}",
                        error
                    )),
                    node_version: None,
                    network_id: None,
                    shard_id: None,
                    peers: None,
                    nodes: None,
                    native_token_name: None,
                    native_token_symbol: None,
                    native_token_decimals: None,
                    last_finalized_block: None,
                    is_validator: None,
                    is_read_only: None,
                    is_ready: None,
                    current_epoch: None,
                    epoch_length: None,
                };
            }
        };

        NetworkStatus {
            reachable: true,
            node_url: self.base_url.clone(),
            latency_ms: Some(latency_ms),
            http_status: Some(http_status),
            error: None,
            node_version: payload.version.and_then(|v| v.node),
            network_id: payload.network_id,
            shard_id: payload.shard_id,
            peers: payload.peers,
            nodes: payload.nodes,
            native_token_name: payload.native_token_name,
            native_token_symbol: payload.native_token_symbol,
            native_token_decimals: payload.native_token_decimals,
            last_finalized_block: payload.last_finalized_block_number,
            is_validator: payload.is_validator,
            is_read_only: payload.is_read_only,
            is_ready: payload.is_ready,
            current_epoch: payload.current_epoch,
            epoch_length: payload.epoch_length,
        }
    }
}
