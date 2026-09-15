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

        let overall = if checks
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
        };

        VerificationReport {
            target: status.node_url.clone(),
            status: overall,
            checks: checks.into_iter().map(|c| c.check).collect(),
        }
    }

    pub fn run_checks(status: &NetworkStatus) -> Vec<EvidenceCheck> {
        vec![
            Self::reachability(status),
            Self::http_status(status),
            Self::latency(status),
            Self::probe_integrity(status),
        ]
    }

    fn reachability(status: &NetworkStatus) -> EvidenceCheck {
        if status.reachable {
            EvidenceCheck {
                check: VerificationCheck {
                    name: "node_reachable".into(),
                    status: VerificationStatus::Pass,
                    message: "Node responded to the verification probe.".into(),
                },
                severity: CheckSeverity::Info,
                evidence: vec![
                    VerificationEvidence {
                        source: "rnode".into(),
                        field: "reachable".into(),
                        value: status.reachable.to_string(),
                    },
                ],
            }
        } else {
            EvidenceCheck {
                check: VerificationCheck {
                    name: "node_reachable".into(),
                    status: VerificationStatus::Fail,
                    message: status
                        .error
                        .clone()
                        .unwrap_or_else(|| "Node did not respond.".into()),
                },
                severity: CheckSeverity::Critical,
                evidence: vec![
                    VerificationEvidence {
                        source: "rnode".into(),
                        field: "reachable".into(),
                        value: status.reachable.to_string(),
                    },
                ],
            }
        }
    }

    fn http_status(status: &NetworkStatus) -> EvidenceCheck {
        match status.http_status {
            Some(code) if (200..300).contains(&code) => EvidenceCheck {
                check: VerificationCheck {
                    name: "http_status".into(),
                    status: VerificationStatus::Pass,
                    message: format!("HTTP response {} is successful.", code),
                },
                severity: CheckSeverity::Info,
                evidence: vec![
                    VerificationEvidence {
                        source: "http".into(),
                        field: "status_code".into(),
                        value: code.to_string(),
                    },
                ],
            },

            Some(code) => EvidenceCheck {
                check: VerificationCheck {
                    name: "http_status".into(),
                    status: VerificationStatus::Fail,
                    message: format!("Unexpected HTTP response {}.", code),
                },
                severity: CheckSeverity::Critical,
                evidence: vec![
                    VerificationEvidence {
                        source: "http".into(),
                        field: "status_code".into(),
                        value: code.to_string(),
                    },
                ],
            },

            None => EvidenceCheck {
                check: VerificationCheck {
                    name: "http_status".into(),
                    status: VerificationStatus::Warn,
                    message: "No HTTP response status was captured.".into(),
                },
                severity: CheckSeverity::Warning,
                evidence: Vec::new(),
            },
        }
    }

    fn latency(status: &NetworkStatus) -> EvidenceCheck {
        match status.latency_ms {
            Some(ms) if ms <= 500 => EvidenceCheck {
                check: VerificationCheck {
                    name: "latency".into(),
                    status: VerificationStatus::Pass,
                    message: format!("Probe latency is {} ms.", ms),
                },
                severity: CheckSeverity::Info,
                evidence: vec![
                    VerificationEvidence {
                        source: "rnode".into(),
                        field: "latency_ms".into(),
                        value: ms.to_string(),
                    },
                ],
            },

            Some(ms) if ms <= 1500 => EvidenceCheck {
                check: VerificationCheck {
                    name: "latency".into(),
                    status: VerificationStatus::Warn,
                    message: format!("Probe latency is elevated at {} ms.", ms),
                },
                severity: CheckSeverity::Warning,
                evidence: vec![
                    VerificationEvidence {
                        source: "rnode".into(),
                        field: "latency_ms".into(),
                        value: ms.to_string(),
                    },
                ],
            },

            Some(ms) => EvidenceCheck {
                check: VerificationCheck {
                    name: "latency".into(),
                    status: VerificationStatus::Fail,
                    message: format!("Probe latency is critically high at {} ms.", ms),
                },
                severity: CheckSeverity::Critical,
                evidence: vec![
                    VerificationEvidence {
                        source: "rnode".into(),
                        field: "latency_ms".into(),
                        value: ms.to_string(),
                    },
                ],
            },

            None => EvidenceCheck {
                check: VerificationCheck {
                    name: "latency".into(),
                    status: VerificationStatus::Warn,
                    message: "Latency measurement is unavailable.".into(),
                },
                severity: CheckSeverity::Warning,
                evidence: Vec::new(),
            },
        }
    }

    fn probe_integrity(status: &NetworkStatus) -> EvidenceCheck {
        let valid = status.probe.starts_with("http://")
            || status.probe.starts_with("https://");

        if valid {
            EvidenceCheck {
                check: VerificationCheck {
                    name: "probe_integrity".into(),
                    status: VerificationStatus::Pass,
                    message: "Verification probe uses a valid HTTP(S) endpoint.".into(),
                },
                severity: CheckSeverity::Info,
                evidence: vec![
                    VerificationEvidence {
                        source: "sentinel".into(),
                        field: "probe".into(),
                        value: status.probe.clone(),
                    },
                ],
            }
        } else {
            EvidenceCheck {
                check: VerificationCheck {
                    name: "probe_integrity".into(),
                    status: VerificationStatus::Fail,
                    message: "Verification probe is not a valid HTTP(S) endpoint.".into(),
                },
                severity: CheckSeverity::Critical,
                evidence: vec![
                    VerificationEvidence {
                        source: "sentinel".into(),
                        field: "probe".into(),
                        value: status.probe.clone(),
                    },
                ],
            }
        }
    }
}
