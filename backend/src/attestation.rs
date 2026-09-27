use crate::{
    cross_node::CrossNodeVerificationEngine,
    models::{CrossNodeReport, FinalizedBlockEvidence, NetworkStatus},
    rnode::RNodeClient,
};
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub const SENTINEL_ATTESTATION_SCHEMA: &str = "rchain-sentinel-attestation/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelFailureDomainDeclaration {
    pub node_url: String,
    pub operator_id: String,
    pub provider_id: String,
    pub region: String,
    pub failure_domain_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SentinelGenesisEvidence {
    pub configured_hash: String,
    pub available: bool,
    pub raw: Option<Value>,
    pub payload_sha256: Option<String>,
    pub observed_hash: Option<String>,
    pub observed_height: Option<u64>,
    pub hash_match: Option<bool>,
    pub height_zero: Option<bool>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SentinelAttestationPayload {
    pub schema: &'static str,
    pub collected_at_unix_ms: u128,
    pub network: NetworkStatus,
    pub genesis: SentinelGenesisEvidence,
    pub finalized_block: FinalizedBlockEvidence,
    pub cross_node: CrossNodeReport,
    pub failure_domains: Vec<SentinelFailureDomainDeclaration>,
}

#[derive(Debug, Serialize)]
pub struct SentinelAttestationSignature {
    pub algorithm: &'static str,
    pub key_id: String,
    pub public_key_hex: String,
    pub signature_hex: String,
}

#[derive(Debug, Serialize)]
pub struct SignedSentinelAttestation {
    pub schema: &'static str,
    pub payload: SentinelAttestationPayload,
    pub payload_sha256: String,
    pub signature: SentinelAttestationSignature,
}

#[derive(Clone)]
pub struct SentinelAttestationService {
    signing_key: SigningKey,
    genesis_hash: String,
    failure_domains: Arc<Vec<SentinelFailureDomainDeclaration>>,
}

impl SentinelAttestationService {
    pub fn from_env(rnode_urls: &[String]) -> Result<Self, String> {
        let secret_hex = std::env::var("RCHAIN_SENTINEL_ED25519_SECRET_HEX")
            .map_err(|_| "RCHAIN_SENTINEL_ED25519_SECRET_HEX is not configured".to_string())?;
        let secret = hex::decode(secret_hex.trim())
            .map_err(|error| format!("invalid Sentinel Ed25519 secret hex: {error}"))?;
        let secret: [u8; 32] = secret
            .try_into()
            .map_err(|_| "Sentinel Ed25519 secret must contain exactly 32 bytes".to_string())?;

        let genesis_hash = std::env::var("RCHAIN_GENESIS_HASH")
            .map_err(|_| "RCHAIN_GENESIS_HASH is not configured".to_string())?;
        if genesis_hash.trim().is_empty() {
            return Err("RCHAIN_GENESIS_HASH must not be empty".to_string());
        }

        let failure_domains_json = std::env::var("RCHAIN_FAILURE_DOMAINS_JSON")
            .map_err(|_| "RCHAIN_FAILURE_DOMAINS_JSON is not configured".to_string())?;
        let failure_domains: Vec<SentinelFailureDomainDeclaration> =
            serde_json::from_str(&failure_domains_json)
                .map_err(|error| format!("invalid RCHAIN_FAILURE_DOMAINS_JSON: {error}"))?;
        validate_failure_domains(rnode_urls, &failure_domains)?;

        Ok(Self {
            signing_key: SigningKey::from_bytes(&secret),
            genesis_hash,
            failure_domains: Arc::new(failure_domains),
        })
    }

    pub fn key_id(&self) -> String {
        let bytes = self.signing_key.verifying_key().to_bytes();
        format!("sha256:{:x}", Sha256::digest(bytes))
    }

    pub async fn collect_and_sign(
        &self,
        rnode: &RNodeClient,
        rnode_urls: &[String],
    ) -> Result<SignedSentinelAttestation, String> {
        validate_failure_domains(rnode_urls, &self.failure_domains)?;

        let network = rnode.status().await;
        let finalized_block = match rnode.fetch_last_finalized_block_evidence().await {
            Ok(value) => value,
            Err(error) => FinalizedBlockEvidence::unavailable(error),
        };
        let cross_node = CrossNodeVerificationEngine::verify(rnode_urls).await;
        let genesis = collect_genesis_evidence(rnode, &self.genesis_hash).await;

        let collected_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock is before Unix epoch: {error}"))?
            .as_millis();

        let payload = SentinelAttestationPayload {
            schema: SENTINEL_ATTESTATION_SCHEMA,
            collected_at_unix_ms,
            network,
            genesis,
            finalized_block,
            cross_node,
            failure_domains: self.failure_domains.as_ref().clone(),
        };

        let canonical_payload = canonical_json_bytes(&payload)?;
        let payload_sha256 = format!("sha256:{:x}", Sha256::digest(&canonical_payload));

        let mut signing_bytes = Vec::with_capacity(
            SENTINEL_ATTESTATION_SCHEMA.len() + 1 + canonical_payload.len(),
        );
        signing_bytes.extend_from_slice(SENTINEL_ATTESTATION_SCHEMA.as_bytes());
        signing_bytes.push(b'\n');
        signing_bytes.extend_from_slice(&canonical_payload);

        let signature = self.signing_key.sign(&signing_bytes);
        let public_key = self.signing_key.verifying_key().to_bytes();

        Ok(SignedSentinelAttestation {
            schema: SENTINEL_ATTESTATION_SCHEMA,
            payload,
            payload_sha256,
            signature: SentinelAttestationSignature {
                algorithm: "Ed25519",
                key_id: format!("sha256:{:x}", Sha256::digest(public_key)),
                public_key_hex: hex::encode(public_key),
                signature_hex: hex::encode(signature.to_bytes()),
            },
        })
    }
}

fn normalize_url(value: &str) -> String {
    value.trim().trim_end_matches('/').to_string()
}

fn validate_failure_domains(
    rnode_urls: &[String],
    declarations: &[SentinelFailureDomainDeclaration],
) -> Result<(), String> {
    if declarations.len() < 2 {
        return Err("strict Sentinel attestation requires at least two failure-domain declarations".to_string());
    }

    let targets: Vec<String> = rnode_urls.iter().map(|value| normalize_url(value)).collect();
    let target_set: BTreeSet<String> = targets.iter().cloned().collect();
    if target_set.len() != targets.len() {
        return Err("RCHAIN_RNODE_URLS contains duplicate normalized targets".to_string());
    }

    let mut declared_urls = BTreeSet::new();
    let mut operators = BTreeSet::new();
    let mut failure_domains = BTreeSet::new();

    for declaration in declarations {
        if declaration.node_url.trim().is_empty()
            || declaration.operator_id.trim().is_empty()
            || declaration.provider_id.trim().is_empty()
            || declaration.region.trim().is_empty()
            || declaration.failure_domain_id.trim().is_empty()
        {
            return Err("failure-domain declarations require node_url, operator_id, provider_id, region, and failure_domain_id".to_string());
        }

        let normalized = normalize_url(&declaration.node_url);
        if !declared_urls.insert(normalized) {
            return Err("failure-domain declarations contain a duplicate node_url".to_string());
        }
        operators.insert(declaration.operator_id.trim().to_string());
        failure_domains.insert(declaration.failure_domain_id.trim().to_string());
    }

    if declared_urls != target_set {
        return Err("failure-domain declarations must exactly cover RCHAIN_RNODE_URLS".to_string());
    }
    if operators.len() < 2 {
        return Err("strict Sentinel attestation requires at least two distinct operator_id values".to_string());
    }
    if failure_domains.len() < 2 {
        return Err("strict Sentinel attestation requires at least two distinct failure_domain_id values".to_string());
    }

    Ok(())
}

async fn collect_genesis_evidence(
    rnode: &RNodeClient,
    configured_hash: &str,
) -> SentinelGenesisEvidence {
    match rnode.fetch_block(configured_hash).await {
        Ok(raw) => {
            let serialized = serde_json::to_vec(&raw).ok();
            let payload_sha256 = serialized
                .as_ref()
                .map(|bytes| format!("sha256:{:x}", Sha256::digest(bytes)));
            let observed_hash = find_string(
                &raw,
                &["blockHash", "block_hash", "hash", "id"],
            );
            let observed_height = find_u64(
                &raw,
                &["blockNumber", "block_number", "height"],
            );
            let hash_match = observed_hash
                .as_ref()
                .map(|value| value.eq_ignore_ascii_case(configured_hash));
            let height_zero = observed_height.map(|value| value == 0);

            SentinelGenesisEvidence {
                configured_hash: configured_hash.to_string(),
                available: true,
                raw: Some(raw),
                payload_sha256,
                observed_hash,
                observed_height,
                hash_match,
                height_zero,
                error: None,
            }
        }
        Err(error) => SentinelGenesisEvidence {
            configured_hash: configured_hash.to_string(),
            available: false,
            raw: None,
            payload_sha256: None,
            observed_hash: None,
            observed_height: None,
            hash_match: None,
            height_zero: None,
            error: Some(error),
        },
    }
}

fn find_string(value: &Value, keys: &[&str]) -> Option<String> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(candidate) = map.get(*key) {
                    if let Some(text) = candidate.as_str() {
                        if !text.is_empty() {
                            return Some(text.to_string());
                        }
                    }
                }
            }
            map.values().find_map(|nested| find_string(nested, keys))
        }
        Value::Array(items) => items.iter().find_map(|item| find_string(item, keys)),
        _ => None,
    }
}

fn find_u64(value: &Value, keys: &[&str]) -> Option<u64> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(number) = map.get(*key).and_then(Value::as_u64) {
                    return Some(number);
                }
            }
            map.values().find_map(|nested| find_u64(nested, keys))
        }
        Value::Array(items) => items.iter().find_map(|item| find_u64(item, keys)),
        _ => None,
    }
}

fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let value = serde_json::to_value(value)
        .map_err(|error| format!("failed to serialize attestation payload: {error}"))?;
    let canonical = canonicalize_value(value);
    serde_json::to_vec(&canonical)
        .map_err(|error| format!("failed to encode canonical attestation payload: {error}"))
}

fn canonicalize_value(value: Value) -> Value {
    match value {
        Value::Array(items) => {
            Value::Array(items.into_iter().map(canonicalize_value).collect())
        }
        Value::Object(map) => {
            let mut entries = map.into_iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let mut output = Map::new();
            for (key, value) in entries {
                output.insert(key, canonicalize_value(value));
            }
            Value::Object(output)
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    fn declarations() -> Vec<SentinelFailureDomainDeclaration> {
        vec![
            SentinelFailureDomainDeclaration {
                node_url: "https://node-a.example/".to_string(),
                operator_id: "operator-a".to_string(),
                provider_id: "provider-a".to_string(),
                region: "region-a".to_string(),
                failure_domain_id: "fd-a".to_string(),
            },
            SentinelFailureDomainDeclaration {
                node_url: "https://node-b.example".to_string(),
                operator_id: "operator-b".to_string(),
                provider_id: "provider-b".to_string(),
                region: "region-b".to_string(),
                failure_domain_id: "fd-b".to_string(),
            },
        ]
    }

    #[test]
    fn failure_domain_validation_requires_exact_independent_coverage() {
        let urls = vec![
            "https://node-a.example".to_string(),
            "https://node-b.example/".to_string(),
        ];
        assert!(validate_failure_domains(&urls, &declarations()).is_ok());

        let mut duplicate_operator = declarations();
        duplicate_operator[1].operator_id = "operator-a".to_string();
        assert!(validate_failure_domains(&urls, &duplicate_operator).is_err());

        let missing = vec![declarations()[0].clone()];
        assert!(validate_failure_domains(&urls, &missing).is_err());
    }

    #[test]
    fn canonical_json_is_stable_across_object_key_order() {
        let left: Value = serde_json::from_str(r#"{"b":2,"a":{"d":4,"c":3}}"#).unwrap();
        let right: Value = serde_json::from_str(r#"{"a":{"c":3,"d":4},"b":2}"#).unwrap();
        let left = serde_json::to_vec(&canonicalize_value(left)).unwrap();
        let right = serde_json::to_vec(&canonicalize_value(right)).unwrap();
        assert_eq!(left, right);
    }

    #[test]
    fn signing_bytes_verify_with_raw_public_key() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let canonical = br#"{"a":1}"#;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(SENTINEL_ATTESTATION_SCHEMA.as_bytes());
        bytes.push(b'\n');
        bytes.extend_from_slice(canonical);
        let signature = key.sign(&bytes);
        let verifying = VerifyingKey::from_bytes(&key.verifying_key().to_bytes()).unwrap();
        let parsed = Signature::from_bytes(&signature.to_bytes());
        assert!(verifying.verify(&bytes, &parsed).is_ok());
    }
}
