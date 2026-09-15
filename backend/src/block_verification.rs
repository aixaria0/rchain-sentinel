use crate::models::{
    CheckSeverity,
    FinalizedBlockEvidence,
    VerificationCheck,
    VerificationEvidence,
    VerificationReport,
    VerificationStatus,
};
use sha2::{Digest, Sha256};
use serde_json::Value;

pub struct BlockVerificationEngine;

impl BlockVerificationEngine {
    pub fn verify(evidence: &FinalizedBlockEvidence, target: &str) -> VerificationReport {
        let mut checks = Vec::new();
        checks.push(Self::availability(evidence));
        checks.push(Self::payload_integrity(evidence));
        checks.push(Self::block_hash_presence(evidence));
        checks.push(Self::parent_hash_presence(evidence));
        checks.push(Self::proposer_presence(evidence));
        checks.push(Self::signature_presence(evidence));
        checks.push(Self::justification_presence(evidence));

        let status = if checks.iter().any(|c| matches!(c.status, VerificationStatus::Fail)) {
            VerificationStatus::Fail
        } else if checks.iter().any(|c| matches!(c.status, VerificationStatus::Warn)) {
            VerificationStatus::Warn
        } else {
            VerificationStatus::Pass
        };

        let evidence_count = checks.iter().map(|c| c.evidence.len()).sum();

        VerificationReport {
            target: target.to_string(),
            status,
            checks,
            evidence_count,
        }
    }

    fn availability(evidence: &FinalizedBlockEvidence) -> VerificationCheck {
        if evidence.available {
            Self::check(
                "block_evidence_available",
                VerificationStatus::Pass,
                "Finalized-block evidence was retrieved from the RNode.",
                CheckSeverity::Info,
                "evidence",
                "available",
                "true",
            )
        } else {
            Self::check(
                "block_evidence_available",
                VerificationStatus::Fail,
                evidence.error.as_deref().unwrap_or("Finalized-block evidence is unavailable."),
                CheckSeverity::Critical,
                "evidence",
                "available",
                "false",
            )
        }
    }

    fn payload_integrity(evidence: &FinalizedBlockEvidence) -> VerificationCheck {
        let raw = match &evidence.raw {
            Some(raw) => raw,
            None => return Self::check_without_evidence(
                "payload_integrity",
                VerificationStatus::Warn,
                "No raw payload is available for integrity verification.",
                CheckSeverity::Warning,
            ),
        };

        let expected = match &evidence.payload_sha256 {
            Some(value) => value,
            None => return Self::check_without_evidence(
                "payload_integrity",
                VerificationStatus::Warn,
                "No recorded SHA-256 fingerprint is available for the evidence payload.",
                CheckSeverity::Warning,
            ),
        };

        let serialized = match serde_json::to_vec(raw) {
            Ok(bytes) => bytes,
            Err(error) => return Self::check(
                "payload_integrity",
                VerificationStatus::Fail,
                &format!("Evidence payload could not be serialized: {}", error),
                CheckSeverity::Critical,
                "payload",
                "serialization",
                "failed",
            ),
        };

        let actual = format!("{:x}", Sha256::digest(&serialized));
        if actual == *expected {
            Self::check(
                "payload_integrity",
                VerificationStatus::Pass,
                "Observed evidence payload matches its recorded SHA-256 fingerprint.",
                CheckSeverity::Info,
                "payload",
                "sha256",
                actual,
            )
        } else {
            Self::check(
                "payload_integrity",
                VerificationStatus::Fail,
                "Evidence payload SHA-256 fingerprint does not match the recorded digest.",
                CheckSeverity::Critical,
                "payload",
                "sha256",
                format!("expected={}, actual={}", expected, actual),
            )
        }
    }

    fn block_hash_presence(evidence: &FinalizedBlockEvidence) -> VerificationCheck {
        Self::string_presence(
            "block_hash_presence",
            evidence.block_hash.as_deref(),
            "block_hash",
            "Block hash evidence is available.",
            "Block hash evidence is missing.",
        )
    }

    fn parent_hash_presence(evidence: &FinalizedBlockEvidence) -> VerificationCheck {
        Self::string_presence(
            "parent_hash_presence",
            evidence.parent_hash.as_deref(),
            "parent_hash",
            "Parent hash evidence is available.",
            "Parent hash evidence is missing.",
        )
    }

    fn proposer_presence(evidence: &FinalizedBlockEvidence) -> VerificationCheck {
        Self::string_presence(
            "proposer_presence",
            evidence.proposer.as_deref(),
            "proposer",
            "Proposer evidence is available.",
            "Proposer evidence is missing.",
        )
    }

    fn signature_presence(evidence: &FinalizedBlockEvidence) -> VerificationCheck {
        Self::string_presence(
            "signature_presence",
            evidence.signature.as_deref(),
            "signature",
            "Signature evidence is available.",
            "Signature evidence is missing.",
        )
    }

    fn justification_presence(evidence: &FinalizedBlockEvidence) -> VerificationCheck {
        if evidence.justification_present {
            Self::check(
                "justification_presence",
                VerificationStatus::Pass,
                "Justification evidence is present in the finalized-block payload.",
                CheckSeverity::Info,
                "block",
                "justification",
                "present",
            )
        } else {
            Self::check(
                "justification_presence",
                VerificationStatus::Warn,
                "No recognized justification field was found in the finalized-block payload.",
                CheckSeverity::Warning,
                "block",
                "justification",
                "missing",
            )
        }
    }

    fn string_presence(
        name: &str,
        value: Option<&str>,
        field: &str,
        pass_message: &str,
        warn_message: &str,
    ) -> VerificationCheck {
        match value {
            Some(value) if !value.trim().is_empty() => Self::check(
                name,
                VerificationStatus::Pass,
                pass_message,
                CheckSeverity::Info,
                "block",
                field,
                value.to_string(),
            ),
            _ => Self::check(
                name,
                VerificationStatus::Warn,
                warn_message,
                CheckSeverity::Warning,
                "block",
                field,
                "missing",
            ),
        }
    }

    fn check(
        name: &str,
        status: VerificationStatus,
        message: &str,
        severity: CheckSeverity,
        source: &str,
        field: &str,
        value: impl Into<String>,
    ) -> VerificationCheck {
        VerificationCheck {
            name: name.to_string(),
            status,
            message: message.to_string(),
            severity,
            evidence: vec![VerificationEvidence {
                source: source.to_string(),
                field: field.to_string(),
                value: value.into(),
            }],
        }
    }

    fn check_without_evidence(
        name: &str,
        status: VerificationStatus,
        message: &str,
        severity: CheckSeverity,
    ) -> VerificationCheck {
        VerificationCheck {
            name: name.to_string(),
            status,
            message: message.to_string(),
            severity,
            evidence: Vec::new(),
        }
    }
}

#[allow(dead_code)]
fn _keep_value_import(value: &Value) -> bool {
    !value.is_null()
}
