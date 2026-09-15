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

            let height = status
                .rnode
                .as_ref()
                .and_then(|value| value.last_finalized_block_number);

            CrossNodeAgreement {
                node_url: url,
                reachable: status.reachable,
                finalized_height: height,
                block_hash: evidence.block_hash.clone(),
                payload_sha256: evidence.payload_sha256.clone(),
                proposer: evidence.proposer.clone(),
                signature_present: evidence
                    .signature
                    .as_ref()
                    .is_some_and(|value| !value.is_empty()),
                justification_present: evidence.justification_present,
            }
        }))
        .await;

        let reachable: Vec<&CrossNodeAgreement> = observations
            .iter()
            .filter(|item| item.reachable)
            .collect();
        let reachable_count = reachable.len();
        let evidence_count = reachable
            .iter()
            .filter(|item| item.block_hash.is_some())
            .count();

        let mut candidates: Vec<(u64, Option<String>, usize)> = Vec::new();
        for item in &reachable {
            if let Some(height) = item.finalized_height {
                let key = (height, item.block_hash.clone());
                if let Some(existing) = candidates
                    .iter_mut()
                    .find(|candidate| candidate.0 == key.0 && candidate.1 == key.1)
                {
                    existing.2 += 1;
                } else {
                    candidates.push((key.0, key.1, 1));
                }
            }
        }
        candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.2.cmp(&a.2)));

        let (common_finalized_height, common_block_hash, agreeing_nodes) = candidates
            .first()
            .map(|(height, hash, count)| (Some(*height), hash.clone(), *count))
            .unwrap_or((None, None, 0));

        let quorum_required = if reachable_count == 0 {
            0
        } else {
            (2 * reachable_count + 2) / 3
        };
        let agreement_ratio = if reachable_count == 0 {
            0.0
        } else {
            agreeing_nodes as f64 / reachable_count as f64
        };
        let quorum_observed = quorum_required > 0
            && agreeing_nodes >= quorum_required
            && common_block_hash.is_some();

        let missing_height_nodes = reachable
            .iter()
            .filter(|item| item.finalized_height.is_none())
            .count();
        let missing_hash_nodes = reachable
            .iter()
            .filter(|item| item.block_hash.is_none())
            .count();

        let height_agreement = reachable_count > 0
            && missing_height_nodes == 0
            && reachable
                .iter()
                .all(|item| item.finalized_height == common_finalized_height);

        let hash_agreement = reachable_count > 0
            && missing_hash_nodes == 0
            && common_block_hash.is_some()
            && reachable
                .iter()
                .all(|item| item.block_hash == common_block_hash.as_ref());

        let conflicting_nodes = reachable
            .iter()
            .filter(|item| {
                item.finalized_height.is_some()
                    && item.block_hash.is_some()
                    && (item.finalized_height != common_finalized_height
                        || item.block_hash != common_block_hash.as_ref())
            })
            .count();

        let agreement = quorum_observed && height_agreement && hash_agreement;

        let (status, verification_basis) = if observations.is_empty() {
            ("warn", "no cross-node observations")
        } else if agreement {
            (
                "pass",
                "observed 2/3 agreement across reachable nodes; not a stake-weighted Casper finality proof",
            )
        } else if conflicting_nodes > 0 {
            (
                "warn",
                "conflicting finalized-block observations detected; Casper stake/validator evidence not yet established",
            )
        } else {
            (
                "warn",
                "insufficient observed agreement or block evidence; Casper stake/validator evidence not yet established",
            )
        };

        CrossNodeReport {
            target_count: observations.len(),
            reachable_count,
            evidence_count,
            agreeing_nodes,
            quorum_required,
            quorum_observed,
            agreement_ratio,
            common_finalized_height,
            common_block_hash,
            height_agreement,
            hash_agreement,
            missing_height_nodes,
            missing_hash_nodes,
            conflicting_nodes,
            agreement,
            status: status.to_string(),
            verification_basis: verification_basis.to_string(),
            observations,
        }
    }
}
