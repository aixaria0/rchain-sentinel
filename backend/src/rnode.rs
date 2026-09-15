use crate::models::{FinalizedBlockEvidence, NetworkStatus, RNodeStatusPayload};
use reqwest::Client;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct RNodeClient { client: Client, base_url: String }

impl RNodeClient {
    pub fn new(base_url: impl Into<String>) -> Self { Self { client: Client::new(), base_url: base_url.into().trim_end_matches('/').to_string() } }

    pub async fn status(&self) -> NetworkStatus {
        let start = std::time::Instant::now();
        let url = format!("{}/api/status", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) => {
                let http_status = response.status().as_u16();
                let latency_ms = start.elapsed().as_millis();
                if !response.status().is_success() { return NetworkStatus { reachable: false, node_url: self.base_url.clone(), latency_ms: Some(latency_ms), http_status: Some(http_status), probe: url, error: Some(format!("RNode returned HTTP {}", http_status)), rnode: None }; }
                match response.json::<RNodeStatusPayload>().await {
                    Ok(payload) => NetworkStatus { reachable: true, node_url: self.base_url.clone(), latency_ms: Some(latency_ms), http_status: Some(http_status), probe: url, error: None, rnode: Some(payload) },
                    Err(error) => NetworkStatus { reachable: true, node_url: self.base_url.clone(), latency_ms: Some(latency_ms), http_status: Some(http_status), probe: url, error: Some(format!("RNode response parsing failed: {}", error)), rnode: None },
                }
            }
            Err(error) => NetworkStatus { reachable: false, node_url: self.base_url.clone(), latency_ms: None, http_status: None, probe: url, error: Some(error.to_string()), rnode: None },
        }
    }

    pub async fn fetch_last_finalized_block(&self) -> Result<Value, String> {
        self.get_json("/api/last-finalized-block").await
    }

    pub async fn fetch_block(&self, hash: &str) -> Result<Value, String> {
        let encoded = urlencoding::encode(hash);
        self.get_json(&format!("/api/block/{}", encoded)).await
    }

    pub async fn is_finalized(&self, hash: &str) -> Result<bool, String> {
        let raw = self.get_json(&format!("/api/is-finalized/{}", urlencoding::encode(hash))).await?;
        Self::parse_finalized_bool(&raw).ok_or_else(|| "is-finalized response did not contain a boolean finality value".to_string())
    }

    async fn get_json(&self, path: &str) -> Result<Value, String> {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client.get(&url).send().await.map_err(|error| error.to_string())?;
        let status = response.status();
        if !status.is_success() { return Err(format!("RNode endpoint {} returned HTTP {}", path, status.as_u16())); }
        response.json::<Value>().await.map_err(|error| format!("Failed to parse {} response: {}", path, error))
    }

    pub async fn fetch_last_finalized_block_evidence(&self) -> Result<FinalizedBlockEvidence, String> {
        let raw = self.fetch_last_finalized_block().await?;
        let serialized = serde_json::to_vec(&raw).map_err(|error| format!("Failed to serialize evidence: {}", error))?;
        let digest = Sha256::digest(&serialized);
        let block_hash = Self::find_string(&raw, &["blockHash", "block_hash", "hash", "id"]);
        let parent_hash = Self::find_parent_hash(&raw);
        let proposer = Self::find_string(&raw, &["sender", "proposer", "creator", "validator"]);
        let signature = Self::find_string(&raw, &["sig", "signature", "blockSignature", "block_signature"]);
        let justification_present = Self::contains_key(&raw, &["justifications", "justification", "approvedBlock", "approved_block"]);

        let mut evidence = FinalizedBlockEvidence::available(raw)
            .with_sha256(format!("{:x}", digest))
            .with_block_fields(block_hash.clone(), parent_hash, proposer, signature, justification_present);

        if let Some(hash) = block_hash {
            let full_block_result = self.fetch_block(&hash).await;
            let finality_result = self.is_finalized(&hash).await;
            let full_block = full_block_result.as_ref().ok().cloned();
            let full_block_hash = full_block.as_ref().and_then(|block| Self::find_string(block, &["blockHash", "block_hash", "hash", "id"]));
            let node_reported_finalized = finality_result.as_ref().ok().copied();
            let finality_hash_match = match (&full_block_hash, evidence.block_hash.as_ref()) { (Some(full), Some(observed)) => Some(full == observed), _ => None };
            let errors = [full_block_result.err(), finality_result.err()].into_iter().flatten().collect::<Vec<_>>();
            evidence = evidence.with_protocol_evidence(full_block, full_block_hash, node_reported_finalized, finality_hash_match, if errors.is_empty() { None } else { Some(errors.join("; ")) });
        } else {
            evidence = evidence.with_protocol_evidence(None, None, None, None, Some("Cannot query /block/{hash} or /is-finalized/{hash}: finalized block hash is missing.".to_string()));
        }
        Ok(evidence)
    }

    fn parse_finalized_bool(value: &Value) -> Option<bool> {
        match value {
            Value::Bool(value) => Some(*value),
            Value::Object(map) => ["finalized", "isFinalized", "is_finalized", "value"].iter().find_map(|key| map.get(*key).and_then(Self::bool_value)),
            _ => None,
        }
    }
    fn bool_value(value: &Value) -> Option<bool> { value.as_bool() }
    fn find_parent_hash(value: &Value) -> Option<String> { Self::find_string(value, &["parentHash", "parent_hash", "parentsHash", "parents_hash"]).or_else(|| Self::find_first_array_string(value, &["parents"])) }
    fn find_first_array_string(value: &Value, keys: &[&str]) -> Option<String> { match value { Value::Object(map) => { for key in keys { if let Some(Value::Array(items)) = map.get(*key) { if let Some(first) = items.iter().find_map(Self::scalar_string) { return Some(first); } } } map.values().find_map(|nested| Self::find_first_array_string(nested, keys)) }, Value::Array(items) => items.iter().find_map(|item| Self::find_first_array_string(item, keys)), _ => None } }
    fn find_string(value: &Value, keys: &[&str]) -> Option<String> { match value { Value::Object(map) => { for key in keys { if let Some(candidate) = map.get(*key) { if let Some(text) = Self::scalar_string(candidate) { return Some(text); } } } map.values().find_map(|nested| Self::find_string(nested, keys)) }, Value::Array(items) => items.iter().find_map(|item| Self::find_string(item, keys)), _ => None } }
    fn contains_key(value: &Value, keys: &[&str]) -> bool { match value { Value::Object(map) => keys.iter().any(|key| map.contains_key(*key)) || map.values().any(|nested| Self::contains_key(nested, keys)), Value::Array(items) => items.iter().any(|item| Self::contains_key(item, keys)), _ => false } }
    fn scalar_string(value: &Value) -> Option<String> { match value { Value::String(text) if !text.is_empty() => Some(text.clone()), Value::Number(number) => Some(number.to_string()), Value::Bool(value) => Some(value.to_string()), _ => None } }
}
