use crate::models::{CrossNodeAgreement, CrossNodeReport, FinalizedBlockEvidence};
use crate::rnode::RNodeClient;
use futures::future::join_all;

pub struct CrossNodeVerificationEngine;

impl CrossNodeVerificationEngine {
    pub async fn verify(urls: &[String]) -> CrossNodeReport {
        let observations = join_all(urls.iter().cloned().map(|url| async move {
            let client = RNodeClient::new(url.clone());
            let status = client.status().await;
            let evidence = match client.fetch_last_finalized_block_evidence().await {
                Ok(value) => value,
                Err(error) => FinalizedBlockEvidence::unavailable(error),
            };

            let height = evidence.raw.as_ref().and_then(|raw| find_u64(raw, &["blockNumber", "block_number", "height"]));
            CrossNodeAgreement {
                node_url: url,
                reachable: status.reachable,
                finalized_height: height,
                block_hash: evidence.block_hash.clone(),
                payload_sha256: evidence.payload_sha256.clone(),
                proposer: evidence.proposer.clone(),
                signature_present: evidence.signature.as_ref().is_some_and(|value| !value.is_empty()),
                justification_present: evidence.justification_present,
                full_block_available: evidence.full_block_available,
                full_block_hash_match: evidence.finality_hash_match,
                node_reported_finalized: evidence.node_reported_finalized,
            }
        })).await;

        let reachable: Vec<&CrossNodeAgreement> = observations.iter().filter(|item| item.reachable).collect();
        let reachable_count = reachable.len();
        let evidence_count = reachable.iter().filter(|item| item.block_hash.is_some()).count();

        let mut candidates: Vec<(u64, Option<String>, usize)> = Vec::new();
        for item in &reachable {
            if let Some(height) = item.finalized_height {
                let key = (height, item.block_hash.clone());
                if let Some(existing) = candidates.iter_mut().find(|candidate| candidate.0 == key.0 && candidate.1 == key.1) {
                    existing.2 += 1;
                } else {
                    candidates.push((key.0, key.1, 1));
                }
            }
        }
        candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.2.cmp(&a.2)));

        let (common_finalized_height, common_block_hash, agreeing_nodes) = candidates.first()
            .map(|(height, hash, count)| (Some(*height), hash.clone(), *count))
            .unwrap_or((None, None, 0));

        let quorum_required = required_quorum(reachable_count);
        let agreement_ratio = if reachable_count == 0 { 0.0 } else { agreeing_nodes as f64 / reachable_count as f64 };
        let quorum_observed = quorum_required > 0 && agreeing_nodes >= quorum_required && common_block_hash.is_some();

        let missing_height_nodes = reachable.iter().filter(|item| item.finalized_height.is_none()).count();
        let missing_hash_nodes = reachable.iter().filter(|item| item.block_hash.is_none()).count();
        let height_agreement = reachable_count > 0 && missing_height_nodes == 0 && reachable.iter().all(|item| item.finalized_height == common_finalized_height);
        let hash_agreement = reachable_count > 0 && missing_hash_nodes == 0 && common_block_hash.is_some() && reachable.iter().all(|item| item.block_hash.as_deref() == common_block_hash.as_deref());
        let conflicting_nodes = reachable.iter().filter(|item| item.finalized_height.is_some() && item.block_hash.is_some() && (item.finalized_height != common_finalized_height || item.block_hash.as_deref() != common_block_hash.as_deref())).count();

        let full_block_verified_nodes = reachable.iter().filter(|item| item.full_block_available && item.full_block_hash_match == Some(true)).count();
        let finality_asserted_nodes = reachable.iter().filter(|item| item.node_reported_finalized == Some(true)).count();
        let finality_rejected_nodes = reachable.iter().filter(|item| item.node_reported_finalized == Some(false)).count();
        let finality_consistent = reachable_count > 0 && finality_rejected_nodes == 0 && finality_asserted_nodes == reachable_count;
        let block_identity_consistent = reachable_count > 0 && full_block_verified_nodes == reachable_count;
        let agreement = quorum_observed && height_agreement && hash_agreement && block_identity_consistent && finality_consistent;

        let (status, verification_basis) = if observations.is_empty() {
            ("warn", "no cross-node observations")
        } else if agreement {
            ("pass", "all reachable nodes agree on height/hash and independently assert finality; full block identity verified on each node; not a stake-weighted Casper finality proof")
        } else if finality_rejected_nodes > 0 {
            ("warn", "at least one reachable node rejects finality for the observed block")
        } else if conflicting_nodes > 0 {
            ("warn", "conflicting finalized-block observations detected across reachable nodes")
        } else if !block_identity_consistent {
            ("warn", "one or more reachable nodes did not provide a matching canonical block response")
        } else {
            ("warn", "insufficient cross-node finality evidence; stake-weighted Casper proof not established")
        };

        CrossNodeReport {
            target_count: observations.len(), reachable_count, evidence_count, agreeing_nodes,
            quorum_required, quorum_observed, agreement_ratio, common_finalized_height, common_block_hash,
            height_agreement, hash_agreement, missing_height_nodes, missing_hash_nodes, conflicting_nodes,
            agreement, status: status.to_string(), verification_basis: verification_basis.to_string(), observations,
        }
    }
}

fn required_quorum(nodes: usize) -> usize {
    if nodes == 0 { 0 } else { (2 * nodes + 2) / 3 }
}

fn find_u64(value: &serde_json::Value, keys: &[&str]) -> Option<u64> {
    match value {
        serde_json::Value::Object(map) => {
            for key in keys {
                if let Some(number) = map.get(*key).and_then(|v| v.as_u64()) { return Some(number); }
            }
            map.values().find_map(|nested| find_u64(nested, keys))
        }
        serde_json::Value::Array(items) => items.iter().find_map(|item| find_u64(item, keys)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::required_quorum;

    #[test]
    fn quorum_matches_two_thirds_ceiling() {
        assert_eq!(required_quorum(0), 0);
        assert_eq!(required_quorum(1), 1);
        assert_eq!(required_quorum(2), 2);
        assert_eq!(required_quorum(3), 2);
        assert_eq!(required_quorum(4), 3);
        assert_eq!(required_quorum(7), 5);
    }
}
