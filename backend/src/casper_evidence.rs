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
        let justifications = Self::array_len(block, "justifications");
        let bonds = Self::bond_stats(block);
        let stake_weight_present = bonds.0 > 0;
        let equivocation_signal = Self::contains_any(raw, &["equivocation", "equivocated", "doubleVote", "double_vote", "conflictingBets", "conflicting_bets"]);
        let bet_present = Self::contains_any(raw, &["bet", "bets", "belief", "beliefs", "proposition", "propositions", "claim"]);

        let (status, basis) = if equivocation_signal {
            ("warning", "possible equivocation signal observed; protocol-level validation is required")
        } else if protocol_block_shape && validator_identity_present && stake_weight_present && justifications > 0 {
            ("observed", "RChain BlockMessage-shaped evidence observed: sender, bonds/stakes, justifications and signature; this is not by itself a Casper finality proof")
        } else if validator_identity_present || stake_weight_present || justifications > 0 {
            ("partial", "some RChain Casper block evidence was observed, but the complete protocol evidence set is not present")
        } else {
            ("insufficient", "the payload does not expose enough RChain Casper protocol evidence for stake-weighted analysis")
        };

        CasperEvidenceReport {
            evidence_available: true,
            protocol_block_shape,
            validator_identity_present,
            stake_weight_present,
            bond_count: bonds.0,
            total_observed_stake: bonds.1,
            bet_present,
            justification_present: justifications > 0,
            justification_count: justifications,
            equivocation_signal,
            recognized_fields,
            status: status.to_string(),
            verification_basis: basis.to_string(),
        }
    }

    fn empty(reason: &str) -> CasperEvidenceReport {
        CasperEvidenceReport { evidence_available: false, protocol_block_shape: false, validator_identity_present: false, stake_weight_present: false, bond_count: 0, total_observed_stake: None, bet_present: false, justification_present: false, justification_count: 0, equivocation_signal: false, recognized_fields: Vec::new(), status: "unavailable".to_string(), verification_basis: reason.to_string() }
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

    fn array_len(value: &Value, key: &str) -> usize {
        match value { Value::Object(map) => map.get(key).and_then(Value::as_array).map_or(0, Vec::len), _ => 0 }
    }

    fn bond_stats(value: &Value) -> (usize, Option<i64>) {
        let Some(Value::Array(bonds)) = (match value { Value::Object(map) => map.get("bonds"), _ => None }) else { return (0, None); };
        let mut total = 0i64;
        let mut valid = 0usize;
        for bond in bonds {
            if let Value::Object(map) = bond {
                if map.get("validator").is_some() && map.get("stake").and_then(Value::as_i64).is_some() { valid += 1; total += map.get("stake").and_then(Value::as_i64).unwrap_or(0); }
            }
        }
        (valid, (valid > 0).then_some(total))
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
        assert_eq!(report.status, "observed");
        assert!(report.verification_basis.contains("not by itself a Casper finality proof"));
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
