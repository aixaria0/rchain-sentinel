use crate::models::{NetworkStatus, RNodeStatusPayload};
use reqwest::Client;
use serde_json::Value;
use sha2::{Digest, Sha256};

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
                        error: Some(format!("RNode returned HTTP {}", http_status)),
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
                        error: Some(format!("RNode response parsing failed: {}", error)),
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

    pub async fn fetch_last_finalized_block(&self) -> Result<Value, String> {
        let url = format!("{}/api/last-finalized-block", self.base_url);

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

        response.json::<Value>().await.map_err(|error| {
            format!(
                "Failed to parse last-finalized-block response: {}",
                error
            )
        })
    }

    pub async fn fetch_last_finalized_block_evidence(
        &self,
    ) -> Result<crate::models::FinalizedBlockEvidence, String> {
        let raw = self.fetch_last_finalized_block().await?;
        let serialized = serde_json::to_vec(&raw)
            .map_err(|error| format!("Failed to serialize evidence: {}", error))?;
        let digest = Sha256::digest(&serialized);

        let block_hash = Self::find_string(&raw, &[
            "blockHash", "block_hash", "hash", "id",
        ]);
        let parent_hash = Self::find_string(&raw, &[
            "parentHash", "parent_hash", "parentsHash", "parents_hash",
        ]);
        let proposer = Self::find_string(&raw, &[
            "proposer", "sender", "creator", "validator",
        ]);
        let signature = Self::find_string(&raw, &[
            "signature", "sig", "blockSignature", "block_signature",
        ]);
        let justification_present = Self::contains_key(&raw, &[
            "justification", "justifications", "approvedBlock", "approved_block",
        ]);

        Ok(crate::models::FinalizedBlockEvidence::available(raw)
            .with_sha256(format!("{:x}", digest))
            .with_block_fields(
                block_hash,
                parent_hash,
                proposer,
                signature,
                justification_present,
            ))
    }

    fn find_string(value: &Value, keys: &[&str]) -> Option<String> {
        match value {
            Value::Object(map) => {
                for key in keys {
                    if let Some(candidate) = map.get(*key) {
                        if let Some(text) = Self::scalar_string(candidate) {
                            return Some(text);
                        }
                    }
                }

                for nested in map.values() {
                    if let Some(found) = Self::find_string(nested, keys) {
                        return Some(found);
                    }
                }

                None
            }
            Value::Array(items) => {
                for item in items {
                    if let Some(found) = Self::find_string(item, keys) {
                        return Some(found);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn contains_key(value: &Value, keys: &[&str]) -> bool {
        match value {
            Value::Object(map) => {
                if keys.iter().any(|key| map.contains_key(*key)) {
                    return true;
                }
                map.values().any(|nested| Self::contains_key(nested, keys))
            }
            Value::Array(items) => items.iter().any(|item| Self::contains_key(item, keys)),
            _ => false,
        }
    }

    fn scalar_string(value: &Value) -> Option<String> {
        match value {
            Value::String(text) if !text.is_empty() => Some(text.clone()),
            Value::Number(number) => Some(number.to_string()),
            Value::Bool(value) => Some(value.to_string()),
            _ => None,
        }
    }
}
