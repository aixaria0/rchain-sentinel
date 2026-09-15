use crate::models::NetworkStatus;
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

        match self.client.get(&url).send().await {
            Ok(response) => {
                let http_status = response.status().as_u16();
                let latency_ms = start.elapsed().as_millis();

                NetworkStatus {
                    reachable: response.status().is_success(),
                    node_url: self.base_url.clone(),
                    latency_ms: Some(latency_ms),
                    http_status: Some(http_status),
                    probe: url,
                    error: if response.status().is_success() {
                        None
                    } else {
                        Some(format!("RNode returned HTTP {}", http_status))
                    },
                }
            }

            Err(error) => NetworkStatus {
                reachable: false,
                node_url: self.base_url.clone(),
                latency_ms: None,
                http_status: None,
                probe: url,
                error: Some(error.to_string()),
            },
        }
    }
}
