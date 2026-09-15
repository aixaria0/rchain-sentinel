use crate::models::{
    NetworkStatus,
    VerificationCheck,
    VerificationReport,
    VerificationStatus,
};

pub struct Verifier;

impl Verifier {
    pub fn verify_network(status: &NetworkStatus) -> VerificationReport {
        let mut checks = Vec::new();

        checks.push(Self::check_reachability(status));
        checks.push(Self::check_http_status(status));
        checks.push(Self::check_latency(status));

        let overall_status = Self::aggregate_status(&checks);

        VerificationReport {
            target: status.node_url.clone(),
            status: overall_status,
            checks,
        }
    }

    fn check_reachability(status: &NetworkStatus) -> VerificationCheck {
        if status.reachable {
            VerificationCheck {
                name: "node_reachable".to_string(),
                status: VerificationStatus::Pass,
                message: "RNode responded successfully.".to_string(),
            }
        } else {
            VerificationCheck {
                name: "node_reachable".to_string(),
                status: VerificationStatus::Fail,
                message: status
                    .error
                    .clone()
                    .unwrap_or_else(|| "RNode is unreachable.".to_string()),
            }
        }
    }

    fn check_http_status(status: &NetworkStatus) -> VerificationCheck {
        match status.http_status {
            Some(code) if (200..300).contains(&code) => VerificationCheck {
                name: "http_status".to_string(),
                status: VerificationStatus::Pass,
                message: format!("RNode returned HTTP {}.", code),
            },

            Some(code) => VerificationCheck {
                name: "http_status".to_string(),
                status: VerificationStatus::Fail,
                message: format!("RNode returned unexpected HTTP {}.", code),
            },

            None => VerificationCheck {
                name: "http_status".to_string(),
                status: VerificationStatus::Warn,
                message: "No HTTP status was received.".to_string(),
            },
        }
    }

    fn check_latency(status: &NetworkStatus) -> VerificationCheck {
        match status.latency_ms {
            Some(latency) if latency <= 500 => VerificationCheck {
                name: "latency".to_string(),
                status: VerificationStatus::Pass,
                message: format!("Response latency is {} ms.", latency),
            },

            Some(latency) if latency <= 1500 => VerificationCheck {
                name: "latency".to_string(),
                status: VerificationStatus::Warn,
                message: format!("Response latency is elevated at {} ms.", latency),
            },

            Some(latency) => VerificationCheck {
                name: "latency".to_string(),
                status: VerificationStatus::Fail,
                message: format!("Response latency is high at {} ms.", latency),
            },

            None => VerificationCheck {
                name: "latency".to_string(),
                status: VerificationStatus::Warn,
                message: "Latency measurement is unavailable.".to_string(),
            },
        }
    }

    fn aggregate_status(checks: &[VerificationCheck]) -> VerificationStatus {
        if checks
            .iter()
            .any(|check| matches!(check.status, VerificationStatus::Fail))
        {
            VerificationStatus::Fail
        } else if checks
            .iter()
            .any(|check| matches!(check.status, VerificationStatus::Warn))
        {
            VerificationStatus::Warn
        } else {
            VerificationStatus::Pass
        }
    }
}use crate::models::{
    NetworkStatus,
    VerificationCheck,
    VerificationReport,
    VerificationStatus,
};

pub struct Verifier;

impl Verifier {
    pub fn verify_network(status: &NetworkStatus) -> VerificationReport {
        let mut checks = Vec::new();

        checks.push(Self::check_reachability(status));
        checks.push(Self::check_http_status(status));
        checks.push(Self::check_latency(status));

        let overall_status = Self::aggregate_status(&checks);

        VerificationReport {
            target: status.node_url.clone(),
            status: overall_status,
            checks,
        }
    }

    fn check_reachability(status: &NetworkStatus) -> VerificationCheck {
        if status.reachable {
            VerificationCheck {
                name: "node_reachable".to_string(),
                status: VerificationStatus::Pass,
                message: "RNode responded successfully.".to_string(),
            }
        } else {
            VerificationCheck {
                name: "node_reachable".to_string(),
                status: VerificationStatus::Fail,
                message: status
                    .error
                    .clone()
                    .unwrap_or_else(|| "RNode is unreachable.".to_string()),
            }
        }
    }

    fn check_http_status(status: &NetworkStatus) -> VerificationCheck {
        match status.http_status {
            Some(code) if (200..300).contains(&code) => VerificationCheck {
                name: "http_status".to_string(),
                status: VerificationStatus::Pass,
                message: format!("RNode returned HTTP {}.", code),
            },

            Some(code) => VerificationCheck {
                name: "http_status".to_string(),
                status: VerificationStatus::Fail,
                message: format!("RNode returned unexpected HTTP {}.", code),
            },

            None => VerificationCheck {
                name: "http_status".to_string(),
                status: VerificationStatus::Warn,
                message: "No HTTP status was received.".to_string(),
            },
        }
    }

    fn check_latency(status: &NetworkStatus) -> VerificationCheck {
        match status.latency_ms {
            Some(latency) if latency <= 500 => VerificationCheck {
                name: "latency".to_string(),
                status: VerificationStatus::Pass,
                message: format!("Response latency is {} ms.", latency),
            },

            Some(latency) if latency <= 1500 => VerificationCheck {
                name: "latency".to_string(),
                status: VerificationStatus::Warn,
                message: format!("Response latency is elevated at {} ms.", latency),
            },

            Some(latency) => VerificationCheck {
                name: "latency".to_string(),
                status: VerificationStatus::Fail,
                message: format!("Response latency is high at {} ms.", latency),
            },

            None => VerificationCheck {
                name: "latency".to_string(),
                status: VerificationStatus::Warn,
                message: "Latency measurement is unavailable.".to_string(),
            },
        }
    }

    fn aggregate_status(checks: &[VerificationCheck]) -> VerificationStatus {
        if checks
            .iter()
            .any(|check| matches!(check.status, VerificationStatus::Fail))
        {
            VerificationStatus::Fail
        } else if checks
            .iter()
            .any(|check| matches!(check.status, VerificationStatus::Warn))
        {
            VerificationStatus::Warn
        } else {
            VerificationStatus::Pass
        }
    }
}
