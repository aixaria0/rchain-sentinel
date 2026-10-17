use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepairPropagationEnvelope {
    pub schema: String,
    pub repair_problem_id: String,
    pub repair_artifact_digest: String,
    pub native_receipt_digest: String,
    pub native_binding_digest: String,
    pub upstream_repository: String,
    pub upstream_commit: String,
    pub repair_action: String,
    pub native_replay_verified: bool,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SentinelRepairObservation {
    pub schema: &'static str,
    pub accepted: bool,
    pub envelope_digest: String,
    pub repair_problem_id: String,
    pub native_replay_verified: bool,
    pub reasons: Vec<String>,
    pub claim_boundary: &'static str,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

pub fn canonical_envelope_bytes(envelope: &RepairPropagationEnvelope) -> Vec<u8> {
    serde_json::to_vec(envelope).expect("repair propagation envelope must serialize")
}

pub fn observe_repair_envelope(envelope: &RepairPropagationEnvelope) -> SentinelRepairObservation {
    let mut reasons = Vec::new();
    if envelope.schema != "causal-assurance-repair-propagation/v1" {
        reasons.push("unsupported propagation schema".to_string());
    }
    for (name, digest) in [
        ("repairArtifactDigest", &envelope.repair_artifact_digest),
        ("nativeReceiptDigest", &envelope.native_receipt_digest),
        ("nativeBindingDigest", &envelope.native_binding_digest),
    ] {
        if !is_sha256(digest) {
            reasons.push(format!("{name} is not a canonical sha256 digest"));
        }
    }
    if envelope.repair_problem_id.trim().is_empty()
        || envelope.upstream_repository.trim().is_empty()
        || envelope.upstream_commit.trim().is_empty()
        || envelope.repair_action.trim().is_empty()
    {
        reasons.push("required provenance field is empty".to_string());
    }
    if !envelope.native_replay_verified {
        reasons.push("native replay is not verified".to_string());
    }

    let digest = Sha256::digest(canonical_envelope_bytes(envelope));
    SentinelRepairObservation {
        schema: "sentinel-repair-observation/v1",
        accepted: reasons.is_empty(),
        envelope_digest: format!("sha256:{digest:x}"),
        repair_problem_id: envelope.repair_problem_id.clone(),
        native_replay_verified: envelope.native_replay_verified,
        reasons,
        claim_boundary: "Sentinel verifies propagation structure and digest syntax only; acceptance is evidence transport, not protocol safety or repair correctness.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> RepairPropagationEnvelope {
        RepairPropagationEnvelope {
            schema: "causal-assurance-repair-propagation/v1".into(),
            repair_problem_id: "casper-duplicate-minimum-sender-coverage-repair".into(),
            repair_artifact_digest: format!("sha256:{}", "a".repeat(64)),
            native_receipt_digest: format!("sha256:{}", "b".repeat(64)),
            native_binding_digest: format!("sha256:{}", "c".repeat(64)),
            upstream_repository: "rchain-community/rchain-rust".into(),
            upstream_commit: "d92f0787a6096cd6d79864ec2d7c1dd9b6912d0b".into(),
            repair_action: "replace:a2->d3".into(),
            native_replay_verified: true,
            claim_boundary: "bounded pinned replay".into(),
        }
    }

    #[test]
    fn accepts_well_formed_transport_envelope_without_promoting_claim() {
        let observed = observe_repair_envelope(&fixture());
        assert!(observed.accepted);
        assert!(observed.claim_boundary.contains("not protocol safety"));
        assert!(observed.envelope_digest.starts_with("sha256:"));
    }

    #[test]
    fn rejects_missing_native_replay() {
        let mut envelope = fixture();
        envelope.native_replay_verified = false;
        let observed = observe_repair_envelope(&envelope);
        assert!(!observed.accepted);
        assert!(observed.reasons.iter().any(|r| r.contains("native replay")));
    }

    #[test]
    fn rejects_malformed_digest() {
        let mut envelope = fixture();
        envelope.native_binding_digest = "sha256:NOT-A-DIGEST".into();
        assert!(!observe_repair_envelope(&envelope).accepted);
    }
}
