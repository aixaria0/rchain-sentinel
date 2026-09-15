use crate::models::{NetworkStatus, RNodeStatusPayload};
use reqwest::Client;
use serde_json::Value;

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

                if !response.status().is_success() {
                    return NetworkStatus {
                        reachable: false,
                        node_url: self.base_url.clone(),
                        latency_ms: Some(latency_ms),
                        http_status: Some(http_status),
                        probe: url,
                        error: Some(format!(
                            "RNode returned HTTP {}",
                            http_status
                        )),
                        rnode: None,
                    };
                }

                match response.json::<RNodeStatusPayload>().await {
                    Ok(payload) => NetworkStatus {
                        reachable: true,
                        node_url: self.base_url.clone(),
                        latency_ms: Some(latency_ms),
                        http_status: Some(http_status),
                        probe: url,
                        error: None,
                        rnode: Some(payload),
                    },

                    Err(error) => NetworkStatus {
                        reachable: true,
                        node_url: self.base_url.clone(),
                        latency_ms: Some(latency_ms),
                        http_status: Some(http_status),
                        probe: url,
                        error: Some(format!(
                            "RNode response parsing failed: {}",
                            error
                        )),
                        rnode: None,
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
                rnode: None,
            },
        }
    }

    pub async fn fetch_last_finalized_block(
        &self,
    ) -> Result<Value, String> {
        let url = format!(
            "{}/api/last-finalized-block",
            self.base_url
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|error| error.to_string())?;

        let status = response.status();

        if !status.is_success() {
            return Err(format!(
                "Last-finalized-block endpoint returned HTTP {}",
                status.as_u16()
            ));
        }

        response
            .json::<Value>()
            .await
            .map_err(|error| {
                format!(
                    "Failed to parse last-finalized-block response: {}",
                    error
                )
            })
    }
}
