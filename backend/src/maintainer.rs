use crate::casper_evidence::CasperEvidenceEngine;
use crate::cross_node::CrossNodeVerificationEngine;
use crate::models::{CasperEvidenceReport, CrossNodeReport, FinalizedBlockEvidence, NetworkStatus};
use crate::rnode::RNodeClient;
use serde::Serialize;
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaintainerEvidencePacket {
    pub subject: Value,
    pub source: Value,
    pub observed_at: String,
    pub observations: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaintainerBrief {
    pub schema: &'static str,
    pub read_only: bool,
    pub observed_at: String,
    pub verdict: &'static str,
    pub first_attention: Option<String>,
    pub source: Value,
    pub network: NetworkStatus,
    pub probes: Value,
    pub finalized: Value,
    pub casper: CasperEvidenceReport,
    pub cross_node: CrossNodeReport,
    pub evidence_packet: MaintainerEvidencePacket,
    pub caveats: Vec<String>,
}

pub struct MaintainerCollector;

impl MaintainerCollector {
    pub async fn collect(rnode: &RNodeClient, rnode_urls: &[String]) -> MaintainerBrief {
        let observed_at = observed_at();
        let network = rnode.status().await;
        let version = rnode.read_text("/version").await;
        let capabilities = rnode.read_json("/api/capabilities").await;
        let shards = rnode.read_json("/api/v1/shards").await;
        let finalized = match rnode.fetch_last_finalized_block_evidence().await {
            Ok(value) => value,
            Err(error) => FinalizedBlockEvidence::unavailable(error),
        };
        let casper = CasperEvidenceEngine::analyze(&finalized);
        let cross_node = CrossNodeVerificationEngine::verify(rnode_urls).await;
        let bonds = finalized.raw.as_ref().map(extract_bonds).unwrap_or_default();

        let status_readable = network.reachable && network.error.is_none() && network.rnode.is_some();
        let version_readable = version.is_ok();
        let capabilities_readable = capabilities.is_ok();
        let shards_readable = shards.is_ok();
        let canonical_consistency = finalized.canonical_consistency;
        let node_reported_finalized = finalized.node_reported_finalized;
        let bond_structure = casper.evidence_available.then_some(casper.bond_structure_valid);
        let cross_node_agreement = (cross_node.target_count >= 2 && cross_node.reachable_count >= 2).then_some(cross_node.agreement);

        let (verdict, first_attention) = classify(
            status_readable,
            version_readable,
            capabilities_readable,
            shards_readable,
            finalized.available,
            canonical_consistency,
            node_reported_finalized,
            bond_structure,
            cross_node_agreement,
        );

        let network_id = network.rnode.as_ref().and_then(|s| s.network_id.clone());
        let shard_id = network.rnode.as_ref().and_then(|s| s.shard_id.clone());
        let latest_block = network.rnode.as_ref().and_then(|s| s.latest_block_number);
        let finalized_height = network
            .rnode
            .as_ref()
            .and_then(|s| s.last_finalized_block_number)
            .or(cross_node.common_finalized_height);
        let version_value = version.as_ref().ok().map(|v| v.trim().to_string());

        let source = json!({
            "nodeUrl": rnode.base_url(),
            "version": version_value,
            "networkId": network_id,
            "shardId": shard_id,
            "sourceSha": std::env::var("RCHAIN_SOURCE_SHA").ok(),
        });

        let observations = json!({
            "health": {
                "statusReadable": status_readable,
                "versionReadable": version_readable,
                "capabilitiesReadable": capabilities_readable,
                "shardsReadable": shards_readable,
                "finalizedBlockAvailable": finalized.available,
                "canonicalConsistency": canonical_consistency,
                "nodeReportedFinalized": node_reported_finalized,
                "bondStructureValid": bond_structure,
                "crossNodeAgreement": cross_node_agreement,
            },
            "node": {
                "latestBlock": latest_block,
                "finalizedBlock": finalized_height,
                "networkId": network_id,
                "shardId": shard_id,
            },
            "finalized": {
                "blockHash": finalized.block_hash,
                "payloadSha256": finalized.payload_sha256,
                "canonicalConsistency": canonical_consistency,
                "canonicalMismatches": finalized.canonical_mismatches,
                "nodeReportedFinalized": node_reported_finalized,
            },
            "casper": {
                "bondCount": casper.bond_count,
                "totalObservedStake": casper.total_observed_stake,
                "bondStructureValid": bond_structure,
                "justificationCount": casper.justification_count,
                "justificationStructureValid": casper.justification_structure_valid,
                "bonds": bonds,
            },
            "crossNode": {
                "targetCount": cross_node.target_count,
                "reachableCount": cross_node.reachable_count,
                "agreeingNodes": cross_node.agreeing_nodes,
                "quorumObserved": cross_node.quorum_observed,
                "agreement": cross_node_agreement,
                "commonFinalizedHeight": cross_node.common_finalized_height,
                "commonBlockHash": cross_node.common_block_hash,
            }
        });

        let evidence_packet = MaintainerEvidencePacket {
            subject: json!({
                "id": "rchain-maintainer-health",
                "label": "Read-only RNode maintainer health snapshot"
            }),
            source: source.clone(),
            observed_at: observed_at.clone(),
            observations,
        };

        let probes = json!({
            "version": probe_summary("/version", &version),
            "capabilities": probe_json_summary("/api/capabilities", &capabilities),
            "shards": probe_json_summary("/api/v1/shards", &shards),
            "apiStatus": {
                "path": "/api/status",
                "ok": status_readable,
                "httpStatus": network.http_status,
                "error": network.error,
            }
        });

        let finalized_summary = json!({
            "available": finalized.available,
            "blockHash": finalized.block_hash,
            "payloadSha256": finalized.payload_sha256,
            "canonicalConsistency": canonical_consistency,
            "canonicalMismatches": finalized.canonical_mismatches,
            "nodeReportedFinalized": node_reported_finalized,
            "finalityHashMatch": finalized.finality_hash_match,
        });

        MaintainerBrief {
            schema: "rchain-maintainer-brief/v1",
            read_only: true,
            observed_at,
            verdict,
            first_attention,
            source,
            network,
            probes,
            finalized: finalized_summary,
            casper,
            cross_node,
            evidence_packet,
            caveats: vec![
                "This collector performs GET-only observation against the configured RNode endpoints.".to_string(),
                "The public HTTP API does not expose every native PoS map; trusted-set and pending-withdrawal state remain unavailable unless an explicit read surface is added upstream.".to_string(),
                "Cross-node agreement is reported only when at least two configured observers are reachable; one node is not treated as independent corroboration.".to_string(),
                "Cross-node agreement is corroborating evidence, not a stake-weighted Casper safety proof.".to_string(),
            ],
        }
    }
}

fn classify(
    status_readable: bool,
    version_readable: bool,
    capabilities_readable: bool,
    shards_readable: bool,
    finalized_available: bool,
    canonical_consistency: Option<bool>,
    node_reported_finalized: Option<bool>,
    bond_structure: Option<bool>,
    cross_node_agreement: Option<bool>,
) -> (&'static str, Option<String>) {
    if !status_readable {
        return ("ATTENTION", Some("RNode /api/status is not readable".to_string()));
    }
    if !finalized_available {
        return ("ATTENTION", Some("last finalized block evidence is unavailable".to_string()));
    }
    if canonical_consistency == Some(false) {
        return ("ATTENTION", Some("canonical block response diverges from observed finalized-block fields".to_string()));
    }
    if node_reported_finalized == Some(false) {
        return ("ATTENTION", Some("RNode rejects finality for the observed finalized block".to_string()));
    }
    if bond_structure == Some(false) {
        return ("ATTENTION", Some("validator bond evidence is structurally inconsistent".to_string()));
    }
    if cross_node_agreement == Some(false) {
        return ("ATTENTION", Some("cross-node finalized-block agreement is incomplete or conflicting".to_string()));
    }

    if !version_readable {
        return ("INCOMPLETE", Some("/version is not readable".to_string()));
    }
    if !capabilities_readable {
        return ("INCOMPLETE", Some("/api/capabilities is not readable".to_string()));
    }
    if !shards_readable {
        return ("INCOMPLETE", Some("/api/v1/shards is not readable".to_string()));
    }
    if canonical_consistency.is_none() {
        return ("INCOMPLETE", Some("canonical block comparison is unavailable".to_string()));
    }
    if node_reported_finalized.is_none() {
        return ("INCOMPLETE", Some("node finality assertion is unavailable".to_string()));
    }
    if bond_structure.is_none() {
        return ("INCOMPLETE", Some("bond-structure evidence is unavailable".to_string()));
    }
    if cross_node_agreement.is_none() {
        return ("INCOMPLETE", Some("cross-node finality evidence is unavailable".to_string()));
    }

    ("PASS", None)
}

fn extract_bonds(value: &Value) -> Vec<Value> {
    match value {
        Value::Object(map) => {
            if let Some(Value::Array(items)) = map.get("bonds") {
                return items.clone();
            }
            map.values().find_map(|nested| {
                let result = extract_bonds(nested);
                (!result.is_empty()).then_some(result)
            }).unwrap_or_default()
        }
        Value::Array(items) => items.iter().find_map(|nested| {
            let result = extract_bonds(nested);
            (!result.is_empty()).then_some(result)
        }).unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn observed_at() -> String {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    format!("unix:{}", seconds)
}

fn probe_summary(path: &str, result: &Result<String, String>) -> Value {
    match result {
        Ok(value) => json!({ "path": path, "ok": true, "value": value.trim() }),
        Err(error) => json!({ "path": path, "ok": false, "error": error }),
    }
}

fn probe_json_summary(path: &str, result: &Result<Value, String>) -> Value {
    match result {
        Ok(value) => json!({ "path": path, "ok": true, "value": value }),
        Err(error) => json!({ "path": path, "ok": false, "error": error }),
    }
}

#[cfg(test)]
mod tests {
    use super::classify;

    #[test]
    fn healthy_surface_passes() {
        let (verdict, attention) = classify(true, true, true, true, true, Some(true), Some(true), Some(true), Some(true));
        assert_eq!(verdict, "PASS");
        assert!(attention.is_none());
    }

    #[test]
    fn semantic_divergence_beats_transport_success() {
        let (verdict, attention) = classify(true, true, true, true, true, Some(false), Some(true), Some(true), Some(true));
        assert_eq!(verdict, "ATTENTION");
        assert!(attention.unwrap().contains("canonical block"));
    }

    #[test]
    fn missing_finality_evidence_is_incomplete() {
        let (verdict, attention) = classify(true, true, true, true, true, Some(true), None, Some(true), Some(true));
        assert_eq!(verdict, "INCOMPLETE");
        assert!(attention.unwrap().contains("finality assertion"));
    }

    #[test]
    fn missing_independent_observer_is_incomplete() {
        let (verdict, attention) = classify(true, true, true, true, true, Some(true), Some(true), Some(true), None);
        assert_eq!(verdict, "INCOMPLETE");
        assert!(attention.unwrap().contains("cross-node"));
    }
}
