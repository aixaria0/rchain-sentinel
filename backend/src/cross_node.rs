use crate::models::{CrossNodeAgreement, CrossNodeReport, FinalizedBlockEvidence};
use crate::rnode::RNodeClient;

pub struct CrossNodeVerificationEngine;

impl CrossNodeVerificationEngine {
    pub async fn verify(urls: &[String]) -> CrossNodeReport {
        let mut observations = Vec::new();

        for url in urls {
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

            observations.push(CrossNodeAgreement {
                node_url: url.clone(),
                reachable: status.reachable,
                finalized_height: height,
                block_hash: evidence.block_hash.clone(),
                justification_present: evidence.justification_present,
            });
        }

        let reachable = observations.iter().filter(|item| item.reachable).count();
        let heights: Vec<u64> = observations.iter().filter_map(|item| item.finalized_height).collect();
        let common_height = heights.iter().copied().min();
        let agreeing_nodes = match common_height {
            Some(height) => observations
                .iter()
                .filter(|item| item.finalized_height == Some(height))
                .count(),
            None => 0,
        };

        let agreement = !heights.is_empty() && heights.iter().all(|height| Some(*height) == common_height);

        let status = if observations.is_empty() {
            "warn"
        } else if agreement && reachable == observations.len() {
            "pass"
        } else {
            "warn"
        };

        CrossNodeReport {
            target_count: observations.len(),
            reachable_count: reachable,
            agreeing_nodes,
            common_finalized_height: common_height,
            agreement,
            status: status.to_string(),
            observations,
        }
    }
}
