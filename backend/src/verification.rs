use crate::models::{
    NetworkStatus,
    VerificationCheck,
    VerificationReport,
    VerificationStatus,
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct VerificationEvidence {
    pub source: String,
    pub field: String,
    pub value: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum CheckSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EvidenceCheck {
    pub check: VerificationCheck,
    pub severity: CheckSeverity,
    pub evidence: Vec<VerificationEvidence>,
}

pub struct VerificationEngine;

impl VerificationEngine {
    pub fn verify_network(status: &NetworkStatus) -> VerificationReport {
        let checks = Self::run_checks(status);

        let overall = Self::aggregate_status(&checks);

        VerificationReport {
            target: status.node_url.clone(),
            status: overall,
            checks: checks
                .into_iter()
                .map(|check| check.check)
                .collect(),
        }
    }

    pub fn run_checks(status: &NetworkStatus) -> Vec<EvidenceCheck> {
        let mut checks = vec![
            Self::reachability(status),
            Self::http_status(status),
            Self::latency(status),
            Self::probe_integrity(status),
        ];

        if let Some(rnode) = &status.rnode {
            checks.push(Self::node_identity(rnode));
            checks.push(Self::network_identity(rnode));
            checks.push(Self::readiness(rnode));
            checks.push(Self::validator_state(rnode));
            checks.push(Self::finalized_block_state(rnode));
            checks.push(Self::peer_state(rnode));
        } else {
            checks.push(Self::rnode_payload_presence(status));
        }

        checks
    }

    fn aggregate_status(checks: &[EvidenceCheck]) -> VerificationStatus {
        if checks
            .iter()
            .any(|c| matches!(c.check.status, VerificationStatus::Fail))
        {
            VerificationStatus::Fail
        } else if checks
            .iter()
            .any(|c| matches!(c.check.status, VerificationStatus::Warn))
        {
            VerificationStatus::Warn
        } else {
            VerificationStatus::Pass
        }
    }

    fn reachability(status: &NetworkStatus) -> EvidenceCheck {
        if status.reachable {
            Self::check(
                "node_reachable",
                VerificationStatus::Pass,
                "Node responded to the verification probe.",
                CheckSeverity::Info,
                "rnode",
                "reachable",
                status.reachable.to_string(),
            )
        } else {
            Self::check(
                "node_reachable",
                VerificationStatus::Fail,
                status
                    .error
                    .as_deref()
                    .unwrap_or("Node did not respond."),
                CheckSeverity::Critical,
                "rnode",
                "reachable",
                status.reachable.to_string(),
            )
        }
    }

    fn http_status(status: &NetworkStatus) -> EvidenceCheck {
        match status.http_status {
            Some(code) if (200..300).contains(&code) => Self::check(
                "http_status",
                VerificationStatus::Pass,
                &format!("HTTP response {} is successful.", code),
                CheckSeverity::Info,
                "http",
                "status_code",
                code.to_string(),
            ),

            Some(code) => Self::check(
                "http_status",
                VerificationStatus::Fail,
                &format!("Unexpected HTTP response {}.", code),
                CheckSeverity::Critical,
                "http",
                "status_code",
                code.to_string(),
            ),

            None => Self::check_without_evidence(
                "http_status",
                VerificationStatus::Warn,
                "No HTTP response status was captured.",
                CheckSeverity::Warning,
            ),
        }
    }

    fn latency(status: &NetworkStatus) -> EvidenceCheck {
        match status.latency_ms {
            Some(ms) if ms <= 500 => Self::check(
                "latency",
                VerificationStatus::Pass,
                &format!("Probe latency is {} ms.", ms),
                CheckSeverity::Info,
                "rnode",
                "latency_ms",
                ms.to_string(),
            ),

            Some(ms) if ms <= 1500 => Self::check(
                "latency",
                VerificationStatus::Warn,
                &format!("Probe latency is elevated at {} ms.", ms),
                CheckSeverity::Warning,
                "rnode",
                "latency_ms",
                ms.to_string(),
            ),

            Some(ms) => Self::check(
                "latency",
                VerificationStatus::Fail,
                &format!("Probe latency is critically high at {} ms.", ms),
                CheckSeverity::Critical,
                "rnode",
                "latency_ms",
                ms.to_string(),
            ),

            None => Self::check_without_evidence(
                "latency",
                VerificationStatus::Warn,
                "Latency measurement is unavailable.",
                CheckSeverity::Warning,
            ),
        }
    }

    fn probe_integrity(status: &NetworkStatus) -> EvidenceCheck {
        let valid = status.probe.starts_with("http://")
            || status.probe.starts_with("https://");

        if valid {
            Self::check(
                "probe_integrity",
                VerificationStatus::Pass,
                "Verification probe uses a valid HTTP(S) endpoint.",
                CheckSeverity::Info,
                "sentinel",
                "probe",
                status.probe.clone(),
            )
        } else {
            Self::check(
                "probe_integrity",
                VerificationStatus::Fail,
                "Verification probe is not a valid HTTP(S) endpoint.",
                CheckSeverity::Critical,
                "sentinel",
                "probe",
                status.probe.clone(),
            )
        }
    }

    fn rnode_payload_presence(status: &NetworkStatus) -> EvidenceCheck {
        Self::check(
            "rnode_payload",
            VerificationStatus::Warn,
            "Node responded, but no structured RNode status payload was available.",
            CheckSeverity::Warning,
            "rnode",
            "payload",
            "missing",
        )
    }

    fn node_identity(
        status: &crate::models::RNodeStatusPayload,
    ) -> EvidenceCheck {
        match &status.node {
            Some(node) => {
                let identity = node
                    .id
                    .as_deref()
                    .unwrap_or("unknown");

                Self::check(
                    "node_identity",
                    VerificationStatus::Pass,
                    "RNode identity information is available.",
                    CheckSeverity::Info,
                    "rnode",
                    "node.id",
                    identity.to_string(),
                )
            }

            None => Self::check_without_evidence(
                "node_identity",
                VerificationStatus::Warn,
                "RNode identity information is unavailable.",
                CheckSeverity::Warning,
            ),
        }
    }

    fn network_identity(
        status: &crate::models::RNodeStatusPayload,
    ) -> EvidenceCheck {
        let network = status.network_id.as_deref();
        let shard = status.shard_id.as_deref();

        match (network, shard) {
            (Some(network), Some(shard)) => Self::check(
                "network_identity",
                VerificationStatus::Pass,
                "Network and shard identity are available.",
                CheckSeverity::Info,
                "rnode",
                "network/shard",
                format!("network_id={}, shard_id={}", network, shard),
            ),

            (Some(network), None) => Self::check(
                "network_identity",
                VerificationStatus::Warn,
                "Network identity is available, but shard identity is missing.",
                CheckSeverity::Warning,
                "rnode",
                "network_id",
                network.to_string(),
            ),

            (None, Some(shard)) => Self::check(
                "network_identity",
                VerificationStatus::Warn,
                "Shard identity is available, but network identity is missing.",
                CheckSeverity::Warning,
                "rnode",
                "shard_id",
                shard.to_string(),
            ),

            (None, None) => Self::check_without_evidence(
                "network_identity",
                VerificationStatus::Warn,
                "Network and shard identity are unavailable.",
                CheckSeverity::Warning,
            ),
        }
    }

    fn readiness(
        status: &crate::models::RNodeStatusPayload,
    ) -> EvidenceCheck {
        match status.ready {
            Some(true) => Self::check(
                "node_readiness",
                VerificationStatus::Pass,
                "RNode reports itself as ready.",
                CheckSeverity::Info,
                "rnode",
                "ready",
                "true",
            ),

            Some(false) => Self::check(
                "node_readiness",
                VerificationStatus::Fail,
                "RNode reports that it is not ready.",
                CheckSeverity::Critical,
                "rnode",
                "ready",
                "false",
            ),

            None => Self::check_without_evidence(
                "node_readiness",
                VerificationStatus::Warn,
                "RNode readiness state is unavailable.",
                CheckSeverity::Warning,
            ),
        }
    }

    fn validator_state(
        status: &crate::models::RNodeStatusPayload,
    ) -> EvidenceCheck {
        match status.validator {
            Some(true) => Self::check(
                "validator_state",
                VerificationStatus::Pass,
                "RNode reports validator mode enabled.",
                CheckSeverity::Info,
                "rnode",
                "validator",
                "true",
            ),

            Some(false) => Self::check(
                "validator_state",
                VerificationStatus::Warn,
                "RNode is reachable but is not operating as a validator.",
                CheckSeverity::Warning,
                "rnode",
                "validator",
                "false",
            ),

            None => Self::check_without_evidence(
                "validator_state",
                VerificationStatus::Warn,
                "Validator state is unavailable.",
                CheckSeverity::Warning,
            ),
        }
    }

    fn finalized_block_state(
        status: &crate::models::RNodeStatusPayload,
    ) -> EvidenceCheck {
        match status.last_finalized_block_number {
            Some(block) if block > 0 => Self::check(
                "finalized_block",
                VerificationStatus::Pass,
                &format!(
                    "RNode reports finalized block {}.",
                    block
                ),
                CheckSeverity::Info,
                "rnode",
                "last_finalized_block_number",
                block.to_string(),
            ),

            Some(0) => Self::check(
                "finalized_block",
                VerificationStatus::Warn,
                "RNode reports no finalized block yet.",
                CheckSeverity::Warning,
                "rnode",
                "last_finalized_block_number",
                "0",
            ),

            None => Self::check_without_evidence(
                "finalized_block",
                VerificationStatus::Warn,
                "Finalized block height is unavailable.",
                CheckSeverity::Warning,
            ),
        }
    }

    fn peer_state(
        status: &crate::models::RNodeStatusPayload,
    ) -> EvidenceCheck {
        match &status.peers {
            Some(peers) => {
                let count = match peers {
                    serde_json::Value::Array(items) => items.len(),
                    serde_json::Value::Object(map) => map.len(),
                    _ => 0,
                };

                Self::check(
                    "peer_state",
                    if count > 0 {
                        VerificationStatus::Pass
                    } else {
                        VerificationStatus::Warn
                    },
                    &format!("RNode reports {} peer entries.", count),
                    if count > 0 {
                        CheckSeverity::Info
                    } else {
                        CheckSeverity::Warning
                    },
                    "rnode",
                    "peers",
                    count.to_string(),
                )
            }

            None => Self::check_without_evidence(
                "peer_state",
                VerificationStatus::Warn,
                "Peer information is unavailable.",
                CheckSeverity::Warning,
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
    ) -> EvidenceCheck {
        EvidenceCheck {
            check: VerificationCheck {
                name: name.to_string(),
                status,
                message: message.to_string(),
            },
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
    ) -> EvidenceCheck {
        EvidenceCheck {
            check: VerificationCheck {
                name: name.to_string(),
                status,
                message: message.to_string(),
            },
            severity,
            evidence: Vec::new(),
        }
    }
}
