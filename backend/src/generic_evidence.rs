use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GenericEvidenceEnvelope {
    pub schema: String,
    pub evidence_id: String,
    pub source: String,
    pub kind: String,
    pub collected_at_unix_ms: u64,
    pub subject: String,
    pub payload_sha256: String,
    pub claims: BTreeMap<String, String>,
}

impl GenericEvidenceEnvelope {
    pub fn new(source: impl Into<String>, kind: impl Into<String>, collected_at_unix_ms: u64, subject: impl Into<String>, payload: &[u8], claims: BTreeMap<String, String>) -> Self {
        let source = source.into();
        let kind = kind.into();
        let subject = subject.into();
        let payload_sha256 = format!("sha256:{:x}", Sha256::digest(payload));
        let material = format!("causal-assurance-evidence/v1\0{}\0{}\0{}\0{}\0{}", source, kind, collected_at_unix_ms, subject, payload_sha256);
        let evidence_id = format!("evidence:{:x}", Sha256::digest(material.as_bytes()));
        Self { schema: "causal-assurance-evidence/v1".into(), evidence_id, source, kind, collected_at_unix_ms, subject, payload_sha256, claims }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != "causal-assurance-evidence/v1" { return Err("unexpected evidence schema"); }
        if self.source.trim().is_empty() || self.kind.trim().is_empty() || self.subject.trim().is_empty() { return Err("source, kind, and subject are required"); }
        if !self.payload_sha256.starts_with("sha256:") || self.payload_sha256.len() != 71 { return Err("payload_sha256 must be a SHA-256 fingerprint"); }
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
    fn malformed_digest_is_rejected() {
        let mut envelope = GenericEvidenceEnvelope::new("fixture", "state", 42, "subject-1", b"payload", BTreeMap::new());
        envelope.payload_sha256 = "not-a-digest".into();
        assert!(envelope.validate().is_err());
    }
}
