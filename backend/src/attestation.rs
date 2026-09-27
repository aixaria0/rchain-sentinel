use crate::models::{CrossNodeReport, FinalizedBlockEvidence, NetworkStatus};
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

pub const ATTESTATION_SCHEMA: &str = "rchain-sentinel-attestation/v1";
pub const SIGNING_KEY_ENV: &str = "SENTINEL_ED25519_SEED_HEX";

#[derive(Debug, Clone, Serialize)]
pub struct SentinelAttestationPayload {
    pub schema: &'static str,
    pub collected_at_unix_ms: u128,
    pub network: NetworkStatus,
    pub finalized_block: FinalizedBlockEvidence,
    pub cross_node: CrossNodeReport,
}

impl SentinelAttestationPayload {
    pub fn new(
        network: NetworkStatus,
        finalized_block: FinalizedBlockEvidence,
        cross_node: CrossNodeReport,
    ) -> Result<Self, String> {
        let collected_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock is before UNIX epoch: {error}"))?
            .as_millis();

        Ok(Self {
            schema: ATTESTATION_SCHEMA,
            collected_at_unix_ms,
            network,
            finalized_block,
            cross_node,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SentinelSignature {
    pub algorithm: &'static str,
    pub key_id: String,
    pub public_key_hex: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignedSentinelSnapshot {
    pub schema: &'static str,
    pub payload: SentinelAttestationPayload,
    pub payload_sha256: String,
    pub signature: SentinelSignature,
}

pub struct SentinelSigner {
    key_pair: Ed25519KeyPair,
}

impl SentinelSigner {
    pub fn from_env() -> Result<Option<Self>, String> {
        match std::env::var(SIGNING_KEY_ENV) {
            Ok(value) => Self::from_seed_hex(&value).map(Some),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(error) => Err(format!("failed to read {SIGNING_KEY_ENV}: {error}")),
        }
    }

    fn from_seed_hex(value: &str) -> Result<Self, String> {
        let bytes = decode_hex(value.trim())?;
        let seed: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
            format!(
                "{SIGNING_KEY_ENV} must decode to exactly 32 bytes; got {}",
                bytes.len()
            )
        })?;
        let key_pair = Ed25519KeyPair::from_seed_unchecked(&seed)
            .map_err(|_| format!("{SIGNING_KEY_ENV} did not produce a valid Ed25519 key"))?;
        Ok(Self { key_pair })
    }

    pub fn key_id(&self) -> String {
        let digest = Sha256::digest(self.key_pair.public_key().as_ref());
        format!("sha256:{}", encode_hex(&digest))
    }

    pub fn public_key_hex(&self) -> String {
        encode_hex(self.key_pair.public_key().as_ref())
    }

    pub fn sign(&self, payload: SentinelAttestationPayload) -> Result<SignedSentinelSnapshot, String> {
        let payload_bytes = canonical_payload_bytes(&payload)?;
        let payload_digest = Sha256::digest(&payload_bytes);
        let message = signing_message(&payload_bytes);
        let signature = self.key_pair.sign(&message);

        Ok(SignedSentinelSnapshot {
            schema: ATTESTATION_SCHEMA,
            payload,
            payload_sha256: format!("sha256:{}", encode_hex(&payload_digest)),
            signature: SentinelSignature {
                algorithm: "Ed25519",
                key_id: self.key_id(),
                public_key_hex: encode_hex(self.key_pair.public_key().as_ref()),
                signature_hex: encode_hex(signature.as_ref()),
            },
        })
    }
}

fn canonical_payload_bytes(payload: &SentinelAttestationPayload) -> Result<Vec<u8>, String> {
    let value = serde_json::to_value(payload)
        .map_err(|error| format!("failed to convert attestation payload to JSON: {error}"))?;
    serde_json::to_vec(&canonicalize_json(value))
        .map_err(|error| format!("failed to serialize canonical attestation payload: {error}"))
}

fn canonicalize_json(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(canonicalize_json).collect()),
        Value::Object(map) => {
            let mut entries = map.into_iter().collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            let mut canonical = serde_json::Map::new();
            for (key, value) in entries {
                canonical.insert(key, canonicalize_json(value));
            }
            Value::Object(canonical)
        }
        other => other,
    }
}

fn signing_message(payload_bytes: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(ATTESTATION_SCHEMA.len() + 1 + payload_bytes.len());
    message.extend_from_slice(ATTESTATION_SCHEMA.as_bytes());
    message.push(b'\n');
    message.extend_from_slice(payload_bytes);
    message
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    if value.len() % 2 != 0 {
        return Err("hex input must contain an even number of characters".to_string());
    }

    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = hex_nibble(pair[0])?;
            let low = hex_nibble(pair[1])?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn hex_nibble(value: u8) -> Result<u8, String> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(format!("invalid hex character: {}", value as char)),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        CrossNodeAgreement, RNodeIdentity, RNodeStatusPayload,
    };
    use ring::signature::{UnparsedPublicKey, ED25519};

    fn fixture_payload() -> SentinelAttestationPayload {
        SentinelAttestationPayload {
            schema: ATTESTATION_SCHEMA,
            collected_at_unix_ms: 1_700_000_000_000,
            network: NetworkStatus {
                reachable: true,
                node_url: "http://node-a:40403".to_string(),
                latency_ms: Some(4),
                http_status: Some(200),
                probe: "status".to_string(),
                error: None,
                rnode: Some(RNodeStatusPayload {
                    node: Some(RNodeIdentity {
                        id: Some("node-a".to_string()),
                        host: Some("node-a".to_string()),
                        port: Some(40403),
                    }),
                    address: None,
                    network_id: Some("testnet".to_string()),
                    shard_id: Some("root".to_string()),
                    peers: None,
                    nodes: Some(2),
                    latest_block_number: Some(11),
                    last_finalized_block_number: Some(10),
                    validator: Some(true),
                    read_only: Some(false),
                    ready: Some(true),
                    current_epoch: Some(1),
                }),
            },
            finalized_block: FinalizedBlockEvidence::available(serde_json::json!({
                "blockHash": "abc",
                "blockNumber": 10
            }))
            .with_sha256("deadbeef".to_string())
            .with_block_fields(
                Some("abc".to_string()),
                Some("parent".to_string()),
                Some("validator".to_string()),
                Some("signature".to_string()),
                true,
            )
            .with_protocol_evidence(
                Some(serde_json::json!({"blockHash":"abc","blockNumber":10})),
                Some("abc".to_string()),
                Some(true),
                Some(true),
                Some(true),
                Vec::new(),
                None,
            ),
            cross_node: CrossNodeReport {
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
                verification_basis: "two targets agree".to_string(),
                observations: vec![
                    CrossNodeAgreement {
                        node_url: "http://node-a:40403".to_string(),
                        reachable: true,
                        finalized_height: Some(10),
                        block_hash: Some("abc".to_string()),
                        payload_sha256: Some("a".to_string()),
                        proposer: Some("validator".to_string()),
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
                        proposer: Some("validator".to_string()),
                        signature_present: true,
                        justification_present: true,
                        full_block_available: true,
                        full_block_hash_match: Some(true),
                        node_reported_finalized: Some(true),
                    },
                ],
            },
        }
    }

    #[test]
    fn signs_domain_separated_snapshot_and_verifies() {
        let signer = SentinelSigner::from_seed_hex(
            "0101010101010101010101010101010101010101010101010101010101010101",
        )
        .unwrap();
        let signed = signer.sign(fixture_payload()).unwrap();
        let payload_bytes = canonical_payload_bytes(&signed.payload).unwrap();
        let message = signing_message(&payload_bytes);
        let public_key = decode_hex(&signed.signature.public_key_hex).unwrap();
        let signature = decode_hex(&signed.signature.signature_hex).unwrap();

        assert!(UnparsedPublicKey::new(&ED25519, public_key)
            .verify(&message, &signature)
            .is_ok());
        assert_eq!(signed.signature.key_id, signer.key_id());
        assert!(signed.payload_sha256.starts_with("sha256:"));
    }

    #[test]
    fn tampered_payload_does_not_verify_against_existing_signature() {
        let signer = SentinelSigner::from_seed_hex(
            "0202020202020202020202020202020202020202020202020202020202020202",
        )
        .unwrap();
        let mut signed = signer.sign(fixture_payload()).unwrap();
        signed.payload.network.node_url = "http://tampered".to_string();

        let payload_bytes = canonical_payload_bytes(&signed.payload).unwrap();
        let message = signing_message(&payload_bytes);
        let public_key = decode_hex(&signed.signature.public_key_hex).unwrap();
        let signature = decode_hex(&signed.signature.signature_hex).unwrap();

        assert!(UnparsedPublicKey::new(&ED25519, public_key)
            .verify(&message, &signature)
            .is_err());
    }

    #[test]
    fn rejects_bad_seed_length() {
        assert!(SentinelSigner::from_seed_hex("00").is_err());
    }
}
