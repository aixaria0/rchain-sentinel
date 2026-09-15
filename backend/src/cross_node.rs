use crate::models::{CrossNodeAgreement, CrossNodeReport, FinalizedBlockEvidence};
use crate::rnode::RNodeClient;

pub struct CrossNodeVerificationEngine;

impl CrossNodeVerificationEngine {
    pub async fn verify(urls: &[String]) -> CrossNodeReport {
        let mut observations = Vec::with_capacity(urls.len());

        for url in urls {
            let client = RNodeClient::new(url.clone());
            let status = client.status().await;
            let evidence = match client.fetch_last_finalized_block_evidence().await {
                Ok(value) => value,
                Err(error) => FinalizedBlockEvidence::unavailable(error),
            };

            let height = status.rnode.as_ref().and_then(|value| value.last_finalized_block_number);
            observations.push(CrossNodeAgreement {
                node_url: url.clone(),
                reachable: status.reachable,
                finalized_height: height,
                block_hash: evidence.block_hash.clone(),
                payload_sha256: evidence.payload_sha256.clone(),
                proposer: evidence.proposer.clone(),
                signature_present: evidence.signature.as_ref().is_some_and(|v| !v.is_empty()),
                justification_present: evidence.justification_present,
            });
        }

        let reachable_count = observations.iter().filter(|item| item.reachable).count();
        let evidence_count = observations.iter().filter(|item| item.block_hash.is_some()).count();

        let mut candidates: Vec<(u64, Option<String>, usize)> = Vec::new();
        for item in &observations {
            if item.reachable {
                if let Some(height) = item.finalized_height {
                    let key = (height, item.block_hash.clone());
                    if let Some(existing) = candidates.iter_mut().find(|candidate| candidate.0 == key.0 && candidate.1 == key.1) {
                        existing.2 += 1;
                    } else {
                        candidates.push((key.0, key.1, 1));
                    }
                }
            }
        }
        candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.2.cmp(&a.2)));

        let (common_finalized_height, common_block_hash, agreeing_nodes) = candidates
            .first()
            .map(|(height, hash, count)| (Some(*height), hash.clone(), *count))
            .unwrap_or((None, None, 0));

        let quorum_required = if reachable_count == 0 { 0 } else { (2 * reachable_count + 2) / 3 };
        let agreement_ratio = if reachable_count == 0 { 0.0 } else { agreeing_nodes as f64 / reachable_count as f64 };
        let quorum_observed = quorum_required > 0 && agreeing_nodes >= quorum_required && common_block_hash.is_some();

        let height_values: Vec<u64> = observations.iter().filter(|item| item.reachable).filter_map(|item| item.finalized_height).collect();
        let height_agreement = !height_values.is_empty() && height_values.iter().all(|height| Some(*height) == common_finalized_height);

        let hash_values: Vec<&String> = observations.iter().filter(|item| item.reachable).filter_map(|item| item.block_hash.as_ref()).collect();
        let hash_agreement = common_block_hash.is_some() && !hash_values.is_empty() && hash_values.iter().all(|hash| Some((*hash).clone()) == common_block_hash);
        let agreement = quorum_observed && height_agreement && hash_agreement;

        let status = if observations.is_empty() { "warn" } else if agreement { "pass" } else { "warn" };

        CrossNodeReport {
            target_count: observations.len(), reachable_count, evidence_count, agreeing_nodes,
            quorum_required, quorum_observed, agreement_ratio,
            common_finalized_height, common_block_hash, height_agreement, hash_agreement,
            agreement, status: status.to_string(), observations,
        }
    }
}
