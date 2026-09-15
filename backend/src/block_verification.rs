use crate::models::{CheckSeverity, FinalizedBlockEvidence, VerificationCheck, VerificationEvidence, VerificationReport, VerificationStatus};
use sha2::{Digest, Sha256};

pub struct BlockVerificationEngine;

impl BlockVerificationEngine {
    pub fn verify(evidence: &FinalizedBlockEvidence, target: &str) -> VerificationReport {
        let mut checks = vec![
            Self::availability(evidence),
            Self::payload_integrity(evidence),
            Self::block_hash_presence(evidence),
            Self::parent_hash_presence(evidence),
            Self::proposer_presence(evidence),
            Self::signature_presence(evidence),
            Self::justification_presence(evidence),
            Self::full_block_available(evidence),
            Self::full_block_hash_match(evidence),
            Self::canonical_consistency(evidence),
            Self::node_finality_claim(evidence),
        ];
        let status = if checks.iter().any(|c| matches!(c.status, VerificationStatus::Fail)) { VerificationStatus::Fail } else if checks.iter().any(|c| matches!(c.status, VerificationStatus::Warn)) { VerificationStatus::Warn } else { VerificationStatus::Pass };
        let evidence_count = checks.iter().map(|c| c.evidence.len()).sum();
        VerificationReport { target: target.to_string(), status, checks: std::mem::take(&mut checks), evidence_count }
    }

    fn availability(e: &FinalizedBlockEvidence) -> VerificationCheck { if e.available { Self::check("block_evidence_available", VerificationStatus::Pass, "Finalized-block evidence was retrieved from the RNode.", CheckSeverity::Info, "evidence", "available", "true") } else { Self::check("block_evidence_available", VerificationStatus::Fail, e.error.as_deref().unwrap_or("Finalized-block evidence is unavailable."), CheckSeverity::Critical, "evidence", "available", "false") } }

    fn payload_integrity(e: &FinalizedBlockEvidence) -> VerificationCheck {
        let raw = match &e.raw { Some(v) => v, None => return Self::check_without_evidence("payload_integrity", VerificationStatus::Warn, "No raw payload is available for integrity verification.", CheckSeverity::Warning) };
        let expected = match &e.payload_sha256 { Some(v) => v, None => return Self::check_without_evidence("payload_integrity", VerificationStatus::Warn, "No recorded SHA-256 fingerprint is available.", CheckSeverity::Warning) };
        let serialized = match serde_json::to_vec(raw) { Ok(v) => v, Err(error) => return Self::check("payload_integrity", VerificationStatus::Fail, &format!("Evidence payload could not be serialized: {}", error), CheckSeverity::Critical, "payload", "serialization", "failed") };
        let actual = format!("{:x}", Sha256::digest(&serialized));
        if actual == *expected { Self::check("payload_integrity", VerificationStatus::Pass, "Observed evidence payload matches its recorded SHA-256 fingerprint.", CheckSeverity::Info, "payload", "sha256", actual) } else { Self::check("payload_integrity", VerificationStatus::Fail, "Evidence payload SHA-256 fingerprint does not match the recorded digest.", CheckSeverity::Critical, "payload", "sha256", format!("expected={}, actual={}", expected, actual)) }
    }

    fn block_hash_presence(e: &FinalizedBlockEvidence) -> VerificationCheck { Self::string_presence("block_hash_presence", e.block_hash.as_deref(), "block_hash", "Block hash evidence is available.", "Block hash evidence is missing.") }
    fn parent_hash_presence(e: &FinalizedBlockEvidence) -> VerificationCheck { Self::string_presence("parent_hash_presence", e.parent_hash.as_deref(), "parent_hash", "Parent hash evidence is available.", "Parent hash evidence is missing.") }
    fn proposer_presence(e: &FinalizedBlockEvidence) -> VerificationCheck { Self::string_presence("proposer_presence", e.proposer.as_deref(), "proposer", "Proposer evidence is available.", "Proposer evidence is missing.") }
    fn signature_presence(e: &FinalizedBlockEvidence) -> VerificationCheck { Self::string_presence("signature_presence", e.signature.as_deref(), "signature", "Signature evidence is available.", "Signature evidence is missing.") }
    fn justification_presence(e: &FinalizedBlockEvidence) -> VerificationCheck { if e.justification_present { Self::check("justification_presence", VerificationStatus::Pass, "Justification evidence is present in the finalized-block payload.", CheckSeverity::Info, "block", "justification", "present") } else { Self::check("justification_presence", VerificationStatus::Warn, "No recognized justification field was found in the finalized-block payload.", CheckSeverity::Warning, "block", "justification", "missing") } }

    fn full_block_available(e: &FinalizedBlockEvidence) -> VerificationCheck { if e.full_block_available { Self::check("full_block_available", VerificationStatus::Pass, "RNode returned the block through /api/block/{hash}.", CheckSeverity::Info, "rnode", "full_block_available", "true") } else { Self::check("full_block_available", VerificationStatus::Warn, e.finality_error.as_deref().unwrap_or("Full block response is unavailable."), CheckSeverity::Warning, "rnode", "full_block_available", "false") } }

    fn full_block_hash_match(e: &FinalizedBlockEvidence) -> VerificationCheck { match e.finality_hash_match { Some(true) => Self::check("full_block_hash_match", VerificationStatus::Pass, "The block returned by /api/block/{hash} matches the observed finalized-block hash.", CheckSeverity::Info, "rnode", "block_hash_match", "true"), Some(false) => Self::check("full_block_hash_match", VerificationStatus::Fail, "RNode returned a block whose hash does not match the observed finalized-block hash.", CheckSeverity::Critical, "rnode", "block_hash_match", "false"), None => Self::check_without_evidence("full_block_hash_match", VerificationStatus::Warn, "A full block hash could not be compared.", CheckSeverity::Warning) } }

    fn canonical_consistency(e: &FinalizedBlockEvidence) -> VerificationCheck {
        match e.canonical_consistency {
            Some(true) => Self::check("canonical_block_consistency", VerificationStatus::Pass, "The observed finalized-block payload matches the canonical /api/block/{hash} protocol fields.", CheckSeverity::Info, "rnode", "canonical_consistency", "true"),
            Some(false) => Self::check("canonical_block_consistency", VerificationStatus::Fail, &format!("Canonical block field comparison failed: {}", e.canonical_mismatches.join("; ")), CheckSeverity::Critical, "rnode", "canonical_mismatches", e.canonical_mismatches.join("; ")),
            None => Self::check_without_evidence("canonical_block_consistency", VerificationStatus::Warn, "Canonical protocol fields could not be compared.", CheckSeverity::Warning),
        }
    }

    fn node_finality_claim(e: &FinalizedBlockEvidence) -> VerificationCheck { match e.node_reported_finalized { Some(true) => Self::check("node_reported_finality", VerificationStatus::Pass, "RNode explicitly reports the observed block hash as finalized.", CheckSeverity::Info, "rnode", "is_finalized", "true"), Some(false) => Self::check("node_reported_finality", VerificationStatus::Fail, "RNode explicitly reports the observed block hash as not finalized.", CheckSeverity::Critical, "rnode", "is_finalized", "false"), None => Self::check_without_evidence("node_reported_finality", VerificationStatus::Warn, "RNode finality status was unavailable or could not be parsed.", CheckSeverity::Warning) } }

    fn string_presence(name: &str, value: Option<&str>, field: &str, pass_message: &str, warn_message: &str) -> VerificationCheck { match value { Some(value) if !value.trim().is_empty() => Self::check(name, VerificationStatus::Pass, pass_message, CheckSeverity::Info, "block", field, value.to_string()), _ => Self::check(name, VerificationStatus::Warn, warn_message, CheckSeverity::Warning, "block", field, "missing") } }
    fn check(name: &str, status: VerificationStatus, message: &str, severity: CheckSeverity, source: &str, field: &str, value: impl Into<String>) -> VerificationCheck { VerificationCheck { name: name.to_string(), status, message: message.to_string(), severity, evidence: vec![VerificationEvidence { source: source.to_string(), field: field.to_string(), value: value.into() }] } }
    fn check_without_evidence(name: &str, status: VerificationStatus, message: &str, severity: CheckSeverity) -> VerificationCheck { VerificationCheck { name: name.to_string(), status, message: message.to_string(), severity, evidence: Vec::new() } }
}
