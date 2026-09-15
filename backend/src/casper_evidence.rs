use crate::models::{CasperEvidenceReport, FinalizedBlockEvidence};
use serde_json::Value;
use std::collections::BTreeSet;

pub struct CasperEvidenceEngine;

impl CasperEvidenceEngine {
    pub fn analyze(evidence: &FinalizedBlockEvidence) -> CasperEvidenceReport {
        let Some(raw) = evidence.raw.as_ref() else {
            return Self::empty("no finalized-block payload was available");
        };

        let mut recognized = BTreeSet::new();
        Self::collect_keys(raw, &mut recognized);
        let recognized_fields = recognized.into_iter().filter(|k| Self::is_protocol_key(k)).collect::<Vec<_>>();

        let block = Self::find_block(raw).unwrap_or(raw);
        let protocol_block_shape = Self::has(block, "blockHash")
            && Self::has(block, "blockNumber")
            && Self::has(block, "sender")
            && Self::has(block, "justifications")
            && Self::has(block, "bonds")
            && Self::has(block, "sig");

        let validator_identity_present = Self::has(block, "sender") || Self::has(block, "proposer") || Self::has(block, "validator");
        let justifications = Self::justification_stats(block);
        let bonds = Self::bond_stats(block);
        let stake_weight_present = bonds.valid_count > 0;
        let equivocation_signal = Self::contains_any(raw, &["equivocation", "equivocated", "doubleVote", "double_vote", "conflictingBets", "conflicting_bets"]);
        let bet_present = Self::contains_any(raw, &["bet", "bets", "belief", "beliefs", "proposition", "propositions", "claim"]);

        let (status, basis) = if equivocation_signal {
            ("warning", "possible equivocation signal observed; protocol-level validation is required")
        } else if protocol_block_shape && validator_identity_present && stake_weight_present && justifications.count > 0 && bonds.structure_valid && justifications.structure_valid {
            ("observed", "RChain BlockMessage-shaped evidence observed with structurally valid bonds and justifications; this is not by itself a Casper finality proof")
        } else if validator_identity_present || stake_weight_present || justifications.count > 0 {
            ("partial", "some RChain Casper evidence was observed, but the complete or structurally consistent protocol evidence set is not present")
        } else {
            ("insufficient", "the payload does not expose enough RChain Casper protocol evidence for stake-weighted analysis")
        };

        CasperEvidenceReport {
            evidence_available: true,
            protocol_block_shape,
            validator_identity_present,
            stake_weight_present,
            bond_count: bonds.valid_count,
            total_observed_stake: bonds.total_stake,
            duplicate_validator_count: bonds.duplicate_validator_count,
            invalid_bond_count: bonds.invalid_count,
            bond_structure_valid: bonds.structure_valid,
            bet_present,
            justification_present: justifications.count > 0,
            justification_count: justifications.count,
            justification_structure_valid: justifications.structure_valid,
            malformed_justification_count: justifications.malformed_count,
            equivocation_signal,
            recognized_fields,
            status: status.to_string(),
            verification_basis: basis.to_string(),
        }
    }

    fn empty(reason: &str) -> CasperEvidenceReport {
        CasperEvidenceReport {
            evidence_available: false,
            protocol_block_shape: false,
            validator_identity_present: false,
            stake_weight_present: false,
            bond_count: 0,
            total_observed_stake: None,
            duplicate_validator_count: 0,
            invalid_bond_count: 0,
            bond_structure_valid: false,
            bet_present: false,
            justification_present: false,
            justification_count: 0,
            justification_structure_valid: false,
            malformed_justification_count: 0,
            equivocation_signal: false,
            recognized_fields: Vec::new(),
            status: "unavailable".to_string(),
            verification_basis: reason.to_string(),
        }
    }

    fn find_block<'a>(value: &'a Value) -> Option<&'a Value> {
        match value {
            Value::Object(map) => {
                if map.contains_key("blockHash") && (map.contains_key("bonds") || map.contains_key("justifications")) { return Some(value); }
                map.values().find_map(Self::find_block)
            }
            Value::Array(items) => items.iter().find_map(Self::find_block),
            _ => None,
        }
    }

    fn has(value: &Value, key: &str) -> bool { matches!(value, Value::Object(map) if map.contains_key(key)) }

    fn justification_stats(value: &Value) -> JustificationStats {
        let Some(Value::Array(items)) = (match value { Value::Object(map) => map.get("justifications"), _ => None }) else {
            return JustificationStats::default();
        };
        let mut malformed_count = 0usize;
        for item in items {
            let valid = match item {
                Value::Object(map) => !map.is_empty(),
                Value::String(text) => !text.trim().is_empty(),
                Value::Array(_) => true,
                _ => false,
            };
            if !valid { malformed_count += 1; }
        }
        JustificationStats {
            count: items.len(),
            malformed_count,
            structure_valid: !items.is_empty() && malformed_count == 0,
        }
    }

    fn bond_stats(value: &Value) -> BondStats {
        let Some(Value::Array(bonds)) = (match value { Value::Object(map) => map.get("bonds"), _ => None }) else {
            return BondStats::default();
        };
        let mut total = 0i64;
        let mut valid = 0usize;
        let mut invalid = 0usize;
        let mut validators = BTreeSet::new();
        let mut duplicate_validator_count = 0usize;
        for bond in bonds {
            if let Value::Object(map) = bond {
                let validator = map.get("validator").and_then(Value::as_str).filter(|v| !v.trim().is_empty());
                let stake = map.get("stake").and_then(Value::as_i64);
                if let (Some(validator), Some(stake)) = (validator, stake) {
                    valid += 1;
                    total += stake;
                    if !validators.insert(validator.to_string()) { duplicate_validator_count += 1; }
                } else {
                    invalid += 1;
                }
            } else {
                invalid += 1;
            }
        }
        BondStats {
            valid_count: valid,
            total_stake: (valid > 0).then_some(total),
            duplicate_validator_count,
            invalid_count: invalid,
            structure_valid: !bonds.is_empty() && invalid == 0 && duplicate_validator_count == 0 && valid == bonds.len(),
        }
    }

    fn contains_any(value: &Value, keys: &[&str]) -> bool {
        match value { Value::Object(map) => keys.iter().any(|k| map.contains_key(*k)) || map.values().any(|v| Self::contains_any(v, keys)), Value::Array(items) => items.iter().any(|v| Self::contains_any(v, keys)), _ => false }
    }

    fn collect_keys(value: &Value, keys: &mut BTreeSet<String>) {
        match value { Value::Object(map) => { for (k, v) in map { keys.insert(k.clone()); Self::collect_keys(v, keys); } }, Value::Array(items) => { for v in items { Self::collect_keys(v, keys); } }, _ => {} }
    }

    fn is_protocol_key(key: &str) -> bool {
        ["blockHash", "blockNumber", "sender", "seqNum", "shardId", "preStateHash", "postStateHash", "justifications", "bonds", "validator", "stake", "sigAlgorithm", "sig", "fringe", "memberOfFringe", "validated", "validationFailed"].contains(&key)
    }
}

#[derive(Default)]
struct JustificationStats { count: usize, malformed_count: usize, structure_valid: bool }

#[derive(Default)]
struct BondStats {
    valid_count: usize,
    total_stake: Option<i64>,
    duplicate_validator_count: usize,
    invalid_count: usize,
    structure_valid: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_rchain_block_message_shape() {
        let evidence = FinalizedBlockEvidence::available(json!({"block": {
            "blockHash": "abc", "blockNumber": 42, "sender": "validator-1", "seqNum": 7,
            "justifications": ["j1", "j2"], "bonds": [
                {"validator": "v1", "stake": 100}, {"validator": "v2", "stake": 200}
            ], "sigAlgorithm": "ed25519", "sig": "signature"
        }}));
        let report = CasperEvidenceEngine::analyze(&evidence);
        assert!(report.protocol_block_shape);
        assert!(report.validator_identity_present);
        assert!(report.stake_weight_present);
        assert_eq!(report.bond_count, 2);
        assert_eq!(report.total_observed_stake, Some(300));
        assert_eq!(report.justification_count, 2);
        assert!(report.bond_structure_valid);
        assert!(report.justification_structure_valid);
        assert_eq!(report.status, "observed");
        assert!(report.verification_basis.contains("not by itself a Casper finality proof"));
    }

    #[test]
    fn detects_duplicate_and_invalid_bonds() {
        let evidence = FinalizedBlockEvidence::available(json!({"blockHash":"abc", "bonds":[
            {"validator":"v1","stake":100}, {"validator":"v1","stake":200}, {"validator":"v2"}
        ]}));
        let report = CasperEvidenceEngine::analyze(&evidence);
        assert_eq!(report.bond_count, 2);
        assert_eq!(report.duplicate_validator_count, 1);
        assert_eq!(report.invalid_bond_count, 1);
        assert!(!report.bond_structure_valid);
    }

    #[test]
    fn detects_malformed_justifications() {
        let evidence = FinalizedBlockEvidence::available(json!({"blockHash":"abc", "justifications":["ok", null, {"validator":"v1"}]}));
        let report = CasperEvidenceEngine::analyze(&evidence);
        assert_eq!(report.justification_count, 3);
        assert_eq!(report.malformed_justification_count, 1);
        assert!(!report.justification_structure_valid);
    }

    #[test]
    fn does_not_treat_bonds_as_finality() {
        let evidence = FinalizedBlockEvidence::available(json!({"blockHash":"abc", "bonds":[{"validator":"v1","stake":100}]}));
        let report = CasperEvidenceEngine::analyze(&evidence);
        assert!(report.stake_weight_present);
        assert_ne!(report.status, "observed");
    }

    #[test]
    fn flags_equivocation_signal() {
        let evidence = FinalizedBlockEvidence::available(json!({"validator": "validator-1", "equivocation": true}));
        let report = CasperEvidenceEngine::analyze(&evidence);
        assert!(report.equivocation_signal);
        assert_eq!(report.status, "warning");
    }
}
