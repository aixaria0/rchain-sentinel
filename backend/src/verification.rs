use crate::models::{
    CheckSeverity,
    FinalizedBlockEvidence,
    NetworkStatus,
    RNodeObservation,
    VerificationCheck,
    VerificationEvidence,
    VerificationReport,
    VerificationStatus,
};
use serde_json::Value;

#[derive(Debug, Clone, serde::Serialize)]
pub struct EvidenceCheck {
    pub check: VerificationCheck,
    pub severity: CheckSeverity,
    pub evidence: Vec<VerificationEvidence>,
}

pub struct VerificationEngine;

impl VerificationEngine {
    pub fn verify_network(status: &NetworkStatus, finalized_block: &FinalizedBlockEvidence) -> VerificationReport {
        let checks = Self::run_checks(status, finalized_block);
        let overall = Self::aggregate_status(&checks);
        let evidence_count = checks.iter().map(|c| c.evidence.len()).sum();
        VerificationReport {
            target: status.node_url.clone(),
            status: overall,
            evidence_count,
            checks: checks.into_iter().map(|c| c.check).collect(),
        }
    }

    pub fn run_checks(status: &NetworkStatus, finalized_block: &FinalizedBlockEvidence) -> Vec<EvidenceCheck> {
        let mut checks = vec![
            Self::reachability(status),
            Self::http_status(status),
            Self::latency(status),
            Self::probe_integrity(status),
            Self::finalized_block_cross_check(status, finalized_block),
        ];
        if let Some(rnode) = &status.rnode {
            let o = RNodeObservation::from_status(rnode);
            checks.push(Self::node_identity(&o));
            checks.push(Self::network_identity(&o));
            checks.push(Self::readiness(&o));
            checks.push(Self::validator_state(&o));
            checks.push(Self::finalized_block_state(&o));
            checks.push(Self::peer_state(&o));
            checks.push(Self::epoch_state(&o));
            checks.push(Self::observation_integrity(&o));
        } else {
            checks.push(Self::rnode_payload_presence());
        }
        checks
    }

    fn aggregate_status(checks: &[EvidenceCheck]) -> VerificationStatus {
        if checks.iter().any(|c| matches!(c.check.status, VerificationStatus::Fail)) { VerificationStatus::Fail }
        else if checks.iter().any(|c| matches!(c.check.status, VerificationStatus::Warn)) { VerificationStatus::Warn }
        else { VerificationStatus::Pass }
    }

    fn finalized_block_cross_check(status: &NetworkStatus, evidence: &FinalizedBlockEvidence) -> EvidenceCheck {
        let status_height = status.rnode.as_ref().and_then(|r| r.last_finalized_block_number);
        if !evidence.available {
            return Self::check("finalized_block_cross_check", VerificationStatus::Warn, evidence.error.as_deref().unwrap_or("Independent finalized-block evidence is unavailable."), CheckSeverity::Warning, "last-finalized-block", "available", "false");
        }
        let raw = match &evidence.raw {
            Some(v) => v,
            None => return Self::check("finalized_block_cross_check", VerificationStatus::Warn, "Finalized-block endpoint responded, but returned no evidence payload.", CheckSeverity::Warning, "last-finalized-block", "payload", "missing"),
        };
        let evidence_height = Self::extract_block_height(raw);
        match (status_height, evidence_height) {
            (Some(a), Some(b)) if a == b => Self::check("finalized_block_cross_check", VerificationStatus::Pass, &format!("Finalized block height matches across independent RNode evidence paths: {}.", a), CheckSeverity::Info, "cross-check", "finalized_block_height", format!("status={}, evidence={}", a, b)),
            (Some(a), Some(b)) => Self::check("finalized_block_cross_check", VerificationStatus::Fail, &format!("Finalized block height mismatch: status reports {}, while last-finalized-block reports {}.", a, b), CheckSeverity::Critical, "cross-check", "finalized_block_height", format!("status={}, evidence={}", a, b)),
            (Some(a), None) => Self::check("finalized_block_cross_check", VerificationStatus::Warn, &format!("RNode status reports finalized block {}, but no comparable block height could be extracted from the independent evidence.", a), CheckSeverity::Warning, "cross-check", "status_finalized_block", a.to_string()),
            (None, Some(b)) => Self::check("finalized_block_cross_check", VerificationStatus::Warn, &format!("Independent evidence reports finalized block {}, but RNode status does not expose a comparable height.", b), CheckSeverity::Warning, "cross-check", "evidence_finalized_block", b.to_string()),
            (None, None) => Self::check("finalized_block_cross_check", VerificationStatus::Warn, "Both verification paths lack a comparable finalized block height.", CheckSeverity::Warning, "cross-check", "finalized_block_height", "unavailable"),
        }
    }

    fn extract_block_height(value: &Value) -> Option<u64> {
        match value {
            Value::Number(n) => n.as_u64(),
            Value::Object(map) => {
                for key in ["blockNumber", "block_number", "height", "blockHeight", "block_height", "seqNum", "seq_num"] {
                    if let Some(v) = map.get(key).and_then(Self::value_as_u64) { return Some(v); }
                }
                for key in ["block", "header", "metadata"] {
                    if let Some(v) = map.get(key).and_then(Self::extract_block_height) { return Some(v); }
                }
                None
            }
            Value::Array(items) => items.iter().find_map(Self::extract_block_height),
            _ => None,
        }
    }

    fn value_as_u64(value: &Value) -> Option<u64> {
        match value { Value::Number(n) => n.as_u64(), Value::String(s) => s.parse().ok(), _ => None }
    }

    fn reachability(s: &NetworkStatus) -> EvidenceCheck {
        if s.reachable { Self::check("node_reachable", VerificationStatus::Pass, "Node responded to the verification probe.", CheckSeverity::Info, "rnode", "reachable", s.reachable.to_string()) }
        else { Self::check("node_reachable", VerificationStatus::Fail, s.error.as_deref().unwrap_or("Node did not respond."), CheckSeverity::Critical, "rnode", "reachable", s.reachable.to_string()) }
    }

    fn http_status(s: &NetworkStatus) -> EvidenceCheck {
        match s.http_status {
            Some(code) if (200..300).contains(&code) => Self::check("http_status", VerificationStatus::Pass, &format!("HTTP response {} is successful.", code), CheckSeverity::Info, "http", "status_code", code.to_string()),
            Some(code) => Self::check("http_status", VerificationStatus::Fail, &format!("Unexpected HTTP response {}.", code), CheckSeverity::Critical, "http", "status_code", code.to_string()),
            None => Self::check_without_evidence("http_status", VerificationStatus::Warn, "No HTTP response status was captured.", CheckSeverity::Warning),
        }
    }

    fn latency(s: &NetworkStatus) -> EvidenceCheck {
        match s.latency_ms {
            Some(ms) if ms <= 500 => Self::check("latency", VerificationStatus::Pass, &format!("Probe latency is {} ms.", ms), CheckSeverity::Info, "rnode", "latency_ms", ms.to_string()),
            Some(ms) if ms <= 1500 => Self::check("latency", VerificationStatus::Warn, &format!("Probe latency is elevated at {} ms.", ms), CheckSeverity::Warning, "rnode", "latency_ms", ms.to_string()),
            Some(ms) => Self::check("latency", VerificationStatus::Fail, &format!("Probe latency is critically high at {} ms.", ms), CheckSeverity::Critical, "rnode", "latency_ms", ms.to_string()),
            None => Self::check_without_evidence("latency", VerificationStatus::Warn, "Latency measurement is unavailable.", CheckSeverity::Warning),
        }
    }

    fn probe_integrity(s: &NetworkStatus) -> EvidenceCheck {
        if s.probe.starts_with("http://") || s.probe.starts_with("https://") { Self::check("probe_integrity", VerificationStatus::Pass, "Verification probe uses a valid HTTP(S) endpoint.", CheckSeverity::Info, "sentinel", "probe", s.probe.clone()) }
        else { Self::check("probe_integrity", VerificationStatus::Fail, "Verification probe is not a valid HTTP(S) endpoint.", CheckSeverity::Critical, "sentinel", "probe", s.probe.clone()) }
    }

    fn rnode_payload_presence() -> EvidenceCheck { Self::check("rnode_payload", VerificationStatus::Warn, "Node responded, but no structured RNode status payload was available.", CheckSeverity::Warning, "rnode", "payload", "missing") }

    fn node_identity(o: &RNodeObservation) -> EvidenceCheck {
        match &o.node_id {
            Some(id) if !id.is_empty() => Self::check("node_identity", VerificationStatus::Pass, "RNode identity is available.", CheckSeverity::Info, "observation", "node_id", id.clone()),
            Some(_) => Self::check_without_evidence("node_identity", VerificationStatus::Warn, "RNode identity is empty.", CheckSeverity::Warning),
            None => Self::check_without_evidence("node_identity", VerificationStatus::Warn, "RNode identity is unavailable.", CheckSeverity::Warning),
        }
    }

    fn network_identity(o: &RNodeObservation) -> EvidenceCheck {
        match (o.network_id.as_deref(), o.shard_id.as_deref()) {
            (Some(n), Some(sh)) => Self::check("network_identity", VerificationStatus::Pass, "Network and shard identity are available.", CheckSeverity::Info, "observation", "network/shard", format!("network_id={}, shard_id={}", n, sh)),
            (Some(n), None) => Self::check("network_identity", VerificationStatus::Warn, "Network identity is available but shard identity is missing.", CheckSeverity::Warning, "observation", "network_id", n),
            (None, Some(sh)) => Self::check("network_identity", VerificationStatus::Warn, "Shard identity is available but network identity is missing.", CheckSeverity::Warning, "observation", "shard_id", sh),
            (None, None) => Self::check_without_evidence("network_identity", VerificationStatus::Warn, "Network and shard identity are unavailable.", CheckSeverity::Warning),
        }
    }

    fn readiness(o: &RNodeObservation) -> EvidenceCheck {
        match o.ready {
            Some(true) => Self::check("node_readiness", VerificationStatus::Pass, "RNode reports itself as ready.", CheckSeverity::Info, "observation", "ready", "true"),
            Some(false) => Self::check("node_readiness", VerificationStatus::Fail, "RNode reports that it is not ready.", CheckSeverity::Critical, "observation", "ready", "false"),
            None => Self::check_without_evidence("node_readiness", VerificationStatus::Warn, "RNode readiness state is unavailable.", CheckSeverity::Warning),
        }
    }

    fn validator_state(o: &RNodeObservation) -> EvidenceCheck {
        match o.validator {
            Some(true) => Self::check("validator_state", VerificationStatus::Pass, "RNode reports validator mode enabled.", CheckSeverity::Info, "observation", "validator", "true"),
            Some(false) => Self::check("validator_state", VerificationStatus::Warn, "RNode is reachable but is not operating as a validator.", CheckSeverity::Warning, "observation", "validator", "false"),
            None => Self::check_without_evidence("validator_state", VerificationStatus::Warn, "Validator state is unavailable.", CheckSeverity::Warning),
        }
    }

    fn finalized_block_state(o: &RNodeObservation) -> EvidenceCheck {
        match o.finalized_block {
            Some(block) if block > 0 => Self::check("finalized_block", VerificationStatus::Pass, &format!("RNode reports finalized block {}.", block), CheckSeverity::Info, "observation", "finalized_block", block.to_string()),
            Some(_) => Self::check("finalized_block", VerificationStatus::Warn, "RNode reports no finalized block yet.", CheckSeverity::Warning, "observation", "finalized_block", "0"),
            None => Self::check_without_evidence("finalized_block", VerificationStatus::Warn, "Finalized block height is unavailable.", CheckSeverity::Warning),
        }
    }

    fn peer_state(o: &RNodeObservation) -> EvidenceCheck {
        match o.peer_count {
            Some(count) if count > 0 => Self::check("peer_state", VerificationStatus::Pass, &format!("RNode reports {} peer entries.", count), CheckSeverity::Info, "observation", "peer_count", count.to_string()),
            Some(_) => Self::check("peer_state", VerificationStatus::Warn, "RNode reports no peer entries.", CheckSeverity::Warning, "observation", "peer_count", "0"),
            None => Self::check_without_evidence("peer_state", VerificationStatus::Warn, "Peer information is unavailable.", CheckSeverity::Warning),
        }
    }

    fn epoch_state(o: &RNodeObservation) -> EvidenceCheck {
        match o.current_epoch {
            Some(epoch) => Self::check("epoch_state", VerificationStatus::Pass, &format!("RNode reports current epoch {}.", epoch), CheckSeverity::Info, "observation", "current_epoch", epoch.to_string()),
            None => Self::check_without_evidence("epoch_state", VerificationStatus::Warn, "Current epoch is unavailable.", CheckSeverity::Warning),
        }
    }

    fn observation_integrity(o: &RNodeObservation) -> EvidenceCheck {
        let complete = o.node_id.is_some() && o.network_id.is_some() && (o.ready.is_some() || o.validator.is_some() || o.finalized_block.is_some());
        if complete { Self::check("observation_integrity", VerificationStatus::Pass, "Observation contains identity, network, and node-state evidence.", CheckSeverity::Info, "observation", "integrity", "complete") }
        else { Self::check("observation_integrity", VerificationStatus::Warn, "Observation is incomplete and cannot provide a full node-state picture.", CheckSeverity::Warning, "observation", "integrity", "partial") }
    }

    fn check(name: &str, status: VerificationStatus, message: &str, severity: CheckSeverity, source: &str, field: &str, value: impl Into<String>) -> EvidenceCheck {
        let evidence = vec![VerificationEvidence { source: source.to_string(), field: field.to_string(), value: value.into() }];
        let check = VerificationCheck { name: name.to_string(), status, message: message.to_string(), severity: severity.clone(), evidence: evidence.clone() };
        EvidenceCheck { check, severity, evidence }
    }

    fn check_without_evidence(name: &str, status: VerificationStatus, message: &str, severity: CheckSeverity) -> EvidenceCheck {
        let check = VerificationCheck { name: name.to_string(), status, message: message.to_string(), severity: severity.clone(), evidence: Vec::new() };
        EvidenceCheck { check, severity, evidence: Vec::new() }
    }
}
