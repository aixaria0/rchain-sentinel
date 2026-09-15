use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct RNodeClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub reachable: bool,
    pub node_url: String,
    pub latency_ms: Option<u128>,
    pub error: Option<String>,
}

impl RNodeClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    pub async fn check(&self) -> NetworkStatus {
        let start = std::time::Instant::now();

        let url = format!("{}/status", self.base_url);

        match self.client.get(&url).send().await {
            Ok(response) => {
                let latency_ms = start.elapsed().as_millis();

                NetworkStatus {
                    reachable: response.status().is_success(),
                    node_url: self.base_url.clone(),
                    latency_ms: Some(latency_ms),
                    error: if response.status().is_success() {
                        None
                    } else {
                        Some(format!("HTTP {}", response.status()))
                    },
                }
            }

            Err(error) => NetworkStatus {
                reachable: false,
                node_url: self.base_url.clone(),
                latency_ms: None,
                error: Some(error.to_string()),
            },
        }
    }
}
