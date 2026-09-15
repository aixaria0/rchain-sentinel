use crate::models::{CasperEvidenceReport, FinalizedBlockEvidence};
use serde_json::Value;
use std::collections::BTreeSet;

pub struct CasperEvidenceEngine;

impl CasperEvidenceEngine {
    pub fn analyze(evidence: &FinalizedBlockEvidence) -> CasperEvidenceReport {
        let Some(raw) = evidence.raw.as_ref() else {
            return CasperEvidenceReport {
                evidence_available: false,
                validator_identity_present: false,
                stake_weight_present: false,
                bet_present: false,
                justification_present: false,
                equivocation_signal: false,
                recognized_fields: Vec::new(),
                status: "unavailable".to_string(),
                verification_basis: "no finalized-block payload was available".to_string(),
            };
        };

        let validator_identity_present = Self::contains_any(raw, &[
            "validator", "validatorId", "validator_id", "proposer", "sender",
        ]);
        let stake_weight_present = Self::contains_any(raw, &[
            "stake", "weight", "stakeWeight", "stake_weight", "validatorStake", "validator_stake",
        ]);
        let bet_present = Self::contains_any(raw, &[
            "bet", "bets", "belief", "beliefs", "proposition", "propositions",
            "claim", "source", "target",
        ]);
        let justification_present = Self::contains_any(raw, &[
            "justification", "justifications", "justificationMap", "justification_map",
        ]);
        let equivocation_signal = Self::contains_any(raw, &[
            "equivocation", "equivocated", "doubleVote", "double_vote", "conflictingBets", "conflicting_bets",
        ]);

        let mut recognized = BTreeSet::new();
        Self::collect_keys(raw, &mut recognized);
        let recognized_fields = recognized
            .into_iter()
            .filter(|key| Self::is_casper_key(key))
            .collect::<Vec<_>>();

        let sufficient_for_stake_analysis = validator_identity_present && stake_weight_present;
        let status = if equivocation_signal {
            "warning"
        } else if sufficient_for_stake_analysis && bet_present && justification_present {
            "observed"
        } else {
            "insufficient"
        };

        let verification_basis = if equivocation_signal {
            "a possible equivocation/conflicting-bet signal was observed; this requires protocol-level validation"
        } else if sufficient_for_stake_analysis && bet_present && justification_present {
            "validator, stake, bet, and justification-shaped fields were observed; this is not a Casper finality proof"
        } else {
            "the payload does not expose enough recognized Casper-shaped evidence for stake-weighted analysis"
        };

        CasperEvidenceReport {
            evidence_available: true,
            validator_identity_present,
            stake_weight_present,
            bet_present,
            justification_present,
            equivocation_signal,
            recognized_fields,
            status: status.to_string(),
            verification_basis: verification_basis.to_string(),
        }
    }

    fn contains_any(value: &Value, keys: &[&str]) -> bool {
        match value {
            Value::Object(map) => keys.iter().any(|key| map.contains_key(*key))
                || map.values().any(|nested| Self::contains_any(nested, keys)),
            Value::Array(items) => items.iter().any(|item| Self::contains_any(item, keys)),
            _ => false,
        }
    }

    fn collect_keys(value: &Value, keys: &mut BTreeSet<String>) {
        match value {
            Value::Object(map) => {
                for (key, nested) in map {
                    keys.insert(key.clone());
                    Self::collect_keys(nested, keys);
                }
            }
            Value::Array(items) => {
                for item in items {
                    Self::collect_keys(item, keys);
                }
            }
            _ => {}
        }
    }

    fn is_casper_key(key: &str) -> bool {
        let key = key.to_ascii_lowercase();
        [
            "validator", "validatorid", "validator_id", "proposer", "sender", "stake",
            "weight", "stakeweight", "stake_weight", "validatorstake", "validator_stake",
            "bet", "bets", "belief", "beliefs", "proposition", "propositions", "claim",
            "source", "target", "justification", "justifications", "justificationmap",
            "justification_map", "equivocation", "equivocated", "doublevote", "double_vote",
            "conflictingbets", "conflicting_bets",
        ].contains(&key.as_str())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde_json::json;

        #[test]
        fn detects_casper_shaped_evidence_without_claiming_finality() {
            let evidence = FinalizedBlockEvidence::available(json!({
                "proposer": "validator-1",
                "stakeWeight": 42,
                "bets": [{"source": "a", "target": "b", "claim": "x"}],
                "justification": {"validator": "validator-2"}
            }));

            let report = CasperEvidenceEngine::analyze(&evidence);
            assert!(report.validator_identity_present);
            assert!(report.stake_weight_present);
            assert!(report.bet_present);
            assert!(report.justification_present);
            assert_eq!(report.status, "observed");
            assert!(report.verification_basis.contains("not a Casper finality proof"));
        }

        #[test]
        fn flags_equivocation_signal() {
            let evidence = FinalizedBlockEvidence::available(json!({
                "validator": "validator-1",
                "equivocation": true
            }));

            let report = CasperEvidenceEngine::analyze(&evidence);
            assert!(report.equivocation_signal);
            assert_eq!(report.status, "warning");
        }
    }
}
