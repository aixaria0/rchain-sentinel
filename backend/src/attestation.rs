use crate::models::{CrossNodeReport, FinalizedBlockEvidence, NetworkStatus};
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::Path};

pub const SENTINEL_ATTESTATION_SCHEMA: &str = "rchain-sentinel-attestation/v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FailureDomainDeclaration {
    pub node_url: String,
    pub operator_id: String,
    pub provider_id: String,
    pub region: String,
    pub failure_domain_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SentinelAttestationPayload {
    pub schema: &'static str,
    pub collected_at_unix_ms: u64,
    pub network: NetworkStatus,
    pub finalized_block: FinalizedBlockEvidence,
    pub cross_node: CrossNodeReport,
    pub failure_domains: Vec<FailureDomainDeclaration>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SentinelAttestationSignature {
    pub algorithm: &'static str,
    pub key_id: String,
    pub public_key_hex: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignedSentinelAttestation {
    pub schema: &'static str,
    pub payload: SentinelAttestationPayload,
    pub payload_sha256: String,
    pub signature: SentinelAttestationSignature,
}

pub fn load_failure_domain_declarations(
    rnode_urls: &[String],
) -> Result<Vec<FailureDomainDeclaration>, String> {
    let raw = if let Ok(path) = std::env::var("RCHAIN_SENTINEL_FAILURE_DOMAINS_FILE") {
        fs::read_to_string(Path::new(&path))
            .map_err(|error| format!("failed to read failure-domain declaration file: {error}"))?
    } else if let Ok(value) = std::env::var("RCHAIN_SENTINEL_FAILURE_DOMAINS_JSON") {
        value
    } else {
        return Err(
            "signed Sentinel attestation requires RCHAIN_SENTINEL_FAILURE_DOMAINS_FILE or RCHAIN_SENTINEL_FAILURE_DOMAINS_JSON"
                .to_string(),
        );
    };

    let declarations: Vec<FailureDomainDeclaration> = serde_json::from_str(&raw)
        .map_err(|error| format!("invalid failure-domain declaration JSON: {error}"))?;
    validate_failure_domain_declarations(rnode_urls, declarations)
}

pub fn validate_failure_domain_declarations(
    rnode_urls: &[String],
    declarations: Vec<FailureDomainDeclaration>,
) -> Result<Vec<FailureDomainDeclaration>, String> {
    let configured: BTreeSet<String> =
        rnode_urls.iter().map(|url| normalize_url(url)).collect();
    if configured.is_empty() {
        return Err("failure-domain declarations require at least one configured RNode target".to_string());
    }

    let mut normalized = Vec::with_capacity(declarations.len());
    let mut declared = BTreeSet::new();
    for item in declarations {
        let node_url = normalize_url(&item.node_url);
        let operator_id = item.operator_id.trim().to_string();
        let provider_id = item.provider_id.trim().to_string();
        let region = item.region.trim().to_string();
        let failure_domain_id = item.failure_domain_id.trim().to_string();

        if node_url.is_empty()
            || operator_id.is_empty()
            || provider_id.is_empty()
            || region.is_empty()
            || failure_domain_id.is_empty()
        {
            return Err(
                "failure-domain declarations require non-empty node_url, operator_id, provider_id, region, and failure_domain_id"
                    .to_string(),
            );
        }
        if !declared.insert(node_url.clone()) {
            return Err(format!("duplicate failure-domain declaration for target {node_url}"));
        }

        normalized.push(FailureDomainDeclaration {
            node_url,
            operator_id,
            provider_id,
            region,
            failure_domain_id,
        });
    }

    if declared != configured {
        let missing = configured.difference(&declared).cloned().collect::<Vec<_>>();
        let extra = declared.difference(&configured).cloned().collect::<Vec<_>>();
        return Err(format!(
            "failure-domain declarations must exactly cover configured RNode targets; missing={missing:?}, extra={extra:?}"
        ));
    }

    normalized.sort_by(|left, right| left.node_url.cmp(&right.node_url));
    Ok(normalized)
}

fn normalize_url(value: &str) -> String {
    value.trim().trim_end_matches('/').to_string()
}

pub struct AttestationSigner {
    signing_key: SigningKey,
    key_id: String,
    public_key_hex: String,
}

impl AttestationSigner {
    pub fn from_environment() -> Result<Option<Self>, String> {
        if let Ok(path) = std::env::var("RCHAIN_SENTINEL_ED25519_PRIVATE_KEY_FILE") {
            let secret = fs::read_to_string(Path::new(&path))
                .map_err(|error| format!("failed to read Sentinel signing key file: {error}"))?;
            return Self::from_secret_hex(secret.trim()).map(Some);
        }

        if let Ok(secret) = std::env::var("RCHAIN_SENTINEL_ED25519_PRIVATE_KEY_HEX") {
            return Self::from_secret_hex(secret.trim()).map(Some);
        }

        Ok(None)
    }

    pub fn from_secret_hex(secret_hex: &str) -> Result<Self, String> {
        let secret = decode_hex(secret_hex)?;
        let secret: [u8; 32] = secret
            .try_into()
            .map_err(|_| "Sentinel Ed25519 private key must be exactly 32 bytes".to_string())?;
        let signing_key = SigningKey::from_bytes(&secret);
        let public_key = signing_key.verifying_key().to_bytes();
        let public_key_hex = encode_hex(&public_key);
        let key_id = sha256_prefixed(&public_key);

        Ok(Self {
            signing_key,
            key_id,
            public_key_hex,
        })
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub fn sign_snapshot(
        &self,
        collected_at_unix_ms: u64,
        network: NetworkStatus,
        finalized_block: FinalizedBlockEvidence,
        cross_node: CrossNodeReport,
        failure_domains: Vec<FailureDomainDeclaration>,
    ) -> Result<SignedSentinelAttestation, String> {
        let payload = SentinelAttestationPayload {
            schema: SENTINEL_ATTESTATION_SCHEMA,
            collected_at_unix_ms,
            network,
            finalized_block,
            cross_node,
            failure_domains,
        };

        let payload_bytes = canonical_json_bytes(&payload)?;
        let payload_sha256 = sha256_prefixed(&payload_bytes);
        let signing_bytes = signing_bytes(&payload_bytes);
        let signature = self.signing_key.sign(&signing_bytes);

        Ok(SignedSentinelAttestation {
            schema: SENTINEL_ATTESTATION_SCHEMA,
            payload,
            payload_sha256,
            signature: SentinelAttestationSignature {
                algorithm: "Ed25519",
                key_id: self.key_id.clone(),
                public_key_hex: self.public_key_hex.clone(),
                signature_hex: encode_hex(&signature.to_bytes()),
            },
        })
    }
}

fn signing_bytes(payload_bytes: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(SENTINEL_ATTESTATION_SCHEMA.len() + 1 + payload_bytes.len());
    bytes.extend_from_slice(SENTINEL_ATTESTATION_SCHEMA.as_bytes());
    bytes.push(b'\n');
    bytes.extend_from_slice(payload_bytes);
    bytes
}

fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let value = serde_json::to_value(value)
        .map_err(|error| format!("failed to serialize Sentinel attestation payload: {error}"))?;
    let canonical = canonicalize(value);
    serde_json::to_vec(&canonical)
        .map_err(|error| format!("failed to encode canonical Sentinel attestation payload: {error}"))
}

fn canonicalize(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(canonicalize).collect()),
        Value::Object(map) => {
            let mut entries: Vec<_> = map.into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let mut output = serde_json::Map::new();
            for (key, value) in entries {
                output.insert(key, canonicalize(value));
            }
            Value::Object(output)
        }
        scalar => scalar,
    }
}

fn sha256_prefixed(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{}", encode_hex(&digest))
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    if value.len() % 2 != 0 {
        return Err("hex input must have an even number of characters".to_string());
    }

    let mut output = Vec::with_capacity(value.len() / 2);
    let bytes = value.as_bytes();
    for index in (0..bytes.len()).step_by(2) {
        let high = decode_nibble(bytes[index])?;
        let low = decode_nibble(bytes[index + 1])?;
        output.push((high << 4) | low);
    }
    Ok(output)
}

fn decode_nibble(value: u8) -> Result<u8, String> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err("hex input contains a non-hexadecimal character".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        CrossNodeAgreement, RNodeStatusPayload,
    };
    use ed25519_dalek::{Signature, Verifier};

    fn network() -> NetworkStatus {
        NetworkStatus {
            reachable: true,
            node_url: "http://node-a:40403".to_string(),
            latency_ms: Some(7),
            http_status: Some(200),
            probe: "status".to_string(),
            error: None,
            rnode: Some(RNodeStatusPayload {
                node: None,
                address: Some("node-a".to_string()),
                network_id: Some("testnet".to_string()),
                shard_id: Some("root".to_string()),
                peers: None,
                nodes: None,
                latest_block_number: Some(11),
                last_finalized_block_number: Some(10),
                validator: Some(true),
                read_only: Some(false),
                ready: Some(true),
                current_epoch: Some(2),
            }),
        }
    }

    fn failure_domains() -> Vec<FailureDomainDeclaration> {
        vec![
            FailureDomainDeclaration {
                node_url: "http://node-a:40403".to_string(),
                operator_id: "operator-a".to_string(),
                provider_id: "provider-a".to_string(),
                region: "region-a".to_string(),
                failure_domain_id: "domain-a".to_string(),
            },
            FailureDomainDeclaration {
                node_url: "http://node-b:40403".to_string(),
                operator_id: "operator-b".to_string(),
                provider_id: "provider-b".to_string(),
                region: "region-b".to_string(),
                failure_domain_id: "domain-b".to_string(),
            },
        ]
    }

    fn cross_node() -> CrossNodeReport {
        CrossNodeReport {
            target_count: 2,
            reachable_count: 2,
            evidence_count: 2,
            agreeing_nodes: 2,
            quorum_required: 2,
            quorum_observed: true,
            agreement_ratio: 1.0,
            common_finalized_height: Some(10),
            common_block_hash: Some("abc".to_string()),
            height_agreement: true,
            hash_agreement: true,
            missing_height_nodes: 0,
            missing_hash_nodes: 0,
            conflicting_nodes: 0,
            agreement: true,
            status: "pass".to_string(),
            verification_basis: "test fixture".to_string(),
            observations: vec![
                CrossNodeAgreement {
                    node_url: "http://node-a:40403".to_string(),
                    reachable: true,
                    finalized_height: Some(10),
                    block_hash: Some("abc".to_string()),
                    payload_sha256: Some("a".to_string()),
                    proposer: None,
                    signature_present: true,
                    justification_present: true,
                    full_block_available: true,
                    full_block_hash_match: Some(true),
                    node_reported_finalized: Some(true),
                },
                CrossNodeAgreement {
                    node_url: "http://node-b:40403".to_string(),
                    reachable: true,
                    finalized_height: Some(10),
                    block_hash: Some("abc".to_string()),
                    payload_sha256: Some("b".to_string()),
                    proposer: None,
                    signature_present: true,
                    justification_present: true,
                    full_block_available: true,
                    full_block_hash_match: Some(true),
                    node_reported_finalized: Some(true),
                },
            ],
        }
    }

    #[test]
    fn canonical_json_sorts_object_keys_recursively() {
        let value = serde_json::json!({"z": 1, "a": {"d": 4, "b": 2}});
        let encoded = canonical_json_bytes(&value).expect("canonical JSON");
        assert_eq!(
            String::from_utf8(encoded).expect("utf8"),
            "{\"a\":{\"b\":2,\"d\":4},\"z\":1}"
        );
    }

    #[test]
    fn signed_snapshot_is_self_consistent() {
        let signer = AttestationSigner::from_secret_hex(&"11".repeat(32)).expect("signer");
        let snapshot = signer
            .sign_snapshot(
                1_000,
                network(),
                FinalizedBlockEvidence::unavailable("fixture"),
                cross_node(),
                failure_domains(),
            )
            .expect("snapshot");

        assert_eq!(snapshot.schema, SENTINEL_ATTESTATION_SCHEMA);
        assert_eq!(snapshot.payload.schema, SENTINEL_ATTESTATION_SCHEMA);
        assert_eq!(snapshot.signature.algorithm, "Ed25519");
        assert_eq!(snapshot.signature.key_id, signer.key_id());

        let payload_bytes = canonical_json_bytes(&snapshot.payload).expect("payload");
        assert_eq!(snapshot.payload_sha256, sha256_prefixed(&payload_bytes));

        let signature_bytes = decode_hex(&snapshot.signature.signature_hex).expect("signature hex");
        let signature_array: [u8; 64] = signature_bytes.try_into().expect("64-byte signature");
        let signature = Signature::from_bytes(&signature_array);
        signer
            .signing_key
            .verifying_key()
            .verify(&signing_bytes(&payload_bytes), &signature)
            .expect("signature verifies");
    }

    #[test]
    fn validates_exact_failure_domain_coverage() {
        let targets = vec![
            "http://node-a:40403/".to_string(),
            "http://node-b:40403".to_string(),
        ];
        let validated =
            validate_failure_domain_declarations(&targets, failure_domains()).expect("valid declarations");
        assert_eq!(validated.len(), 2);
        assert_eq!(validated[0].node_url, "http://node-a:40403");
        assert_eq!(validated[1].node_url, "http://node-b:40403");
    }

    #[test]
    fn rejects_missing_or_duplicate_failure_domain_declarations() {
        let targets = vec![
            "http://node-a:40403".to_string(),
            "http://node-b:40403".to_string(),
        ];

        assert!(validate_failure_domain_declarations(
            &targets,
            vec![failure_domains()[0].clone()],
        )
        .unwrap_err()
        .contains("exactly cover"));

        let duplicate = vec![
            failure_domains()[0].clone(),
            FailureDomainDeclaration {
                node_url: "http://node-a:40403/".to_string(),
                ..failure_domains()[1].clone()
            },
        ];
        assert!(validate_failure_domain_declarations(&targets, duplicate)
            .unwrap_err()
            .contains("duplicate"));
    }

    #[test]
    fn rejects_malformed_private_keys() {
        assert!(AttestationSigner::from_secret_hex("abcd").is_err());
        assert!(AttestationSigner::from_secret_hex(&"gg".repeat(32)).is_err());
    }
}
