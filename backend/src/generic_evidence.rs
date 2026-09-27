use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const EVIDENCE_SCHEMA: &str = "causal-assurance-evidence/v2";

fn push_length_prefixed(output: &mut Vec<u8>, bytes: &[u8]) {
    let len = u32::try_from(bytes.len()).expect("evidence field exceeds u32 length");
    output.extend_from_slice(&len.to_be_bytes());
    output.extend_from_slice(bytes);
}

fn canonical_claims_bytes(claims: &BTreeMap<String, String>) -> Vec<u8> {
    let mut output = Vec::new();
    output.extend_from_slice(&(claims.len() as u32).to_be_bytes());
    for (key, value) in claims {
        push_length_prefixed(&mut output, key.as_bytes());
        push_length_prefixed(&mut output, value.as_bytes());
    }
    output
}

fn claims_digest(claims: &BTreeMap<String, String>) -> String {
    format!("sha256:{:x}", Sha256::digest(canonical_claims_bytes(claims)))
}

fn evidence_id_material(
    source: &str,
    kind: &str,
    collected_at_unix_ms: u64,
    subject: &str,
    payload_sha256: &str,
    claims_sha256: &str,
) -> Vec<u8> {
    let mut output = Vec::new();
    for field in [
        EVIDENCE_SCHEMA.to_string(),
        source.to_string(),
        kind.to_string(),
        collected_at_unix_ms.to_string(),
        subject.to_string(),
        payload_sha256.to_string(),
        claims_sha256.to_string(),
    ] {
        push_length_prefixed(&mut output, field.as_bytes());
    }
    output
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GenericEvidenceEnvelope {
    pub schema: String,
    pub evidence_id: String,
    pub source: String,
    pub kind: String,
    pub collected_at_unix_ms: u64,
    pub subject: String,
    pub payload_sha256: String,
    pub claims_sha256: String,
    pub claims: BTreeMap<String, String>,
}

impl GenericEvidenceEnvelope {
    pub fn new(
        source: impl Into<String>,
        kind: impl Into<String>,
        collected_at_unix_ms: u64,
        subject: impl Into<String>,
        payload: &[u8],
        claims: BTreeMap<String, String>,
    ) -> Self {
        let source = source.into();
        let kind = kind.into();
        let subject = subject.into();
        let payload_sha256 = format!("sha256:{:x}", Sha256::digest(payload));
        let claims_sha256 = claims_digest(&claims);
        let material = evidence_id_material(
            &source,
            &kind,
            collected_at_unix_ms,
            &subject,
            &payload_sha256,
            &claims_sha256,
        );
        let evidence_id = format!("evidence:{:x}", Sha256::digest(material));
        Self {
            schema: EVIDENCE_SCHEMA.into(),
            evidence_id,
            source,
            kind,
            collected_at_unix_ms,
            subject,
            payload_sha256,
            claims_sha256,
            claims,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != EVIDENCE_SCHEMA {
            return Err("unexpected evidence schema");
        }
        if self.source.trim().is_empty() || self.kind.trim().is_empty() || self.subject.trim().is_empty() {
            return Err("source, kind, and subject are required");
        }
        if !valid_sha256(&self.payload_sha256) || !valid_sha256(&self.claims_sha256) {
            return Err("payload and claims digests must be SHA-256 fingerprints");
        }
        let actual_claims = claims_digest(&self.claims);
        if actual_claims != self.claims_sha256 {
            return Err("claims digest mismatch");
        }
        let material = evidence_id_material(
            &self.source,
            &self.kind,
            self.collected_at_unix_ms,
            &self.subject,
            &self.payload_sha256,
            &self.claims_sha256,
        );
        let actual_id = format!("evidence:{:x}", Sha256::digest(material));
        if actual_id != self.evidence_id {
            return Err("evidence id mismatch");
        }
        Ok(())
    }

    pub fn verify_payload(&self, payload: &[u8]) -> Result<(), &'static str> {
        self.validate()?;
        let actual = format!("sha256:{:x}", Sha256::digest(payload));
        if actual != self.payload_sha256 {
            return Err("payload digest mismatch");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_envelope_is_deterministic_and_protocol_neutral() {
        let mut claims = BTreeMap::new();
        claims.insert("status".into(), "observed".into());
        let a = GenericEvidenceEnvelope::new("fixture", "state", 42, "subject-1", b"payload", claims.clone());
        let b = GenericEvidenceEnvelope::new("fixture", "state", 42, "subject-1", b"payload", claims);
        assert_eq!(a, b);
        assert!(a.validate().is_ok());
        assert!(a.verify_payload(b"payload").is_ok());
    }

    #[test]
    fn tampered_payload_is_rejected() {
        let envelope = GenericEvidenceEnvelope::new("fixture", "state", 42, "subject-1", b"payload", BTreeMap::new());
        assert!(envelope.verify_payload(b"tampered").is_err());
    }

    #[test]
    fn tampered_claims_are_rejected_and_bound_to_identity() {
        let mut claims = BTreeMap::new();
        claims.insert("status".into(), "observed".into());
        let mut envelope = GenericEvidenceEnvelope::new("fixture", "state", 42, "subject-1", b"payload", claims);
        let original_id = envelope.evidence_id.clone();
        envelope.claims.insert("status".into(), "rewritten".into());
        assert_eq!(envelope.validate(), Err("claims digest mismatch"));

        let mut rewritten = BTreeMap::new();
        rewritten.insert("status".into(), "rewritten".into());
        let rebuilt = GenericEvidenceEnvelope::new("fixture", "state", 42, "subject-1", b"payload", rewritten);
        assert_ne!(rebuilt.evidence_id, original_id);
    }

    #[test]
    fn malformed_digest_is_rejected() {
        let mut envelope = GenericEvidenceEnvelope::new("fixture", "state", 42, "subject-1", b"payload", BTreeMap::new());
        envelope.payload_sha256 = "not-a-digest".into();
        assert!(envelope.validate().is_err());
    }
}

#[cfg(test)]
mod cross_repo_fixture_tests {
    use super::*;
    const FIXTURE: &[u8] = b"causal-assurance-fixture/v1\nrun=fixture-001\nsubject=finite-state-model:dual-refinement-fixture\nwitness=valid-down,valid-finish\ncost=2,1\n";
    const DIGEST: &str = "sha256:4f4fd3c2715b193a78d79ac0be11c893aa7bfdc6f9a52ca7de1240d64f2a1703";

    #[test]
    fn canonical_cross_repo_fixture_matches_frozen_digest() {
        let envelope = GenericEvidenceEnvelope::new(
            "cross-repo-fixture",
            "verification-artifact",
            1,
            "dual-refinement-fixture",
            FIXTURE,
            BTreeMap::new(),
        );
        assert_eq!(envelope.payload_sha256, DIGEST);
        assert!(envelope.verify_payload(FIXTURE).is_ok());
        assert!(envelope.verify_payload(b"tampered").is_err());
    }
}
