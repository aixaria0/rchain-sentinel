//! Offline inspection of an external, pinned M27 research envelope.
//! Hash/shape checks do not authenticate its producer or prove RChain finality.
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SOURCE_COMMIT: &str = "2d2c3d879b1a078693c8551385efb54a811d7172";
const UPSTREAM: &str =
    "rchain-community/rchain-rust@d92f0787a6096cd6d79864ec2d7c1dd9b6912d0b";

#[derive(Debug, Serialize)]
pub struct CbcWitnessObservation {
    pub schema: &'static str,
    pub source_revision: String,
    pub upstream_revision: String,
    pub transport_digest: String,
    pub source_report_digest: String,
    pub justification_ids: Vec<String>,
    pub reported_sender_ids: Vec<String>,
    pub reported_minimum_distance: u64,
    pub source_reported_reachable: bool,
    pub source_reported_finalized: bool,
    pub integrity_checked: bool,
    pub producer_authenticated: bool,
    pub independently_verified_finality: bool,
    pub live_network: bool,
    pub evidence_class: &'static str,
    pub claim_boundary: &'static str,
}

fn field<'a>(parent: &'a Value, key: &str) -> Result<&'a Value, String> {
    parent.get(key).ok_or_else(|| format!("Missing {}", key))
}

fn text<'a>(parent: &'a Value, key: &str) -> Result<&'a str, String> {
    field(parent, key)?
        .as_str()
        .ok_or_else(|| format!("Invalid {}", key))
}

fn hex_sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

pub fn inspect(value: &Value) -> Result<CbcWitnessObservation, String> {
    if text(value, "schema")? != "aria-cbc-witness/v1" {
        return Err("Unsupported witness schema".into());
    }
    let source = field(value, "source")?;
    let report = field(value, "report")?;
    if text(source, "repository")? != "aixaria0/RCHAIN-COMPLIER"
        || text(source, "commit")? != SOURCE_COMMIT
        || text(source, "upstreamRevision")? != UPSTREAM
        || text(report, "upstreamRevision")? != UPSTREAM
        || text(report, "milestone")? != "M27"
    {
        return Err("Unsupported pinned source or milestone".into());
    }
    let reported_digest = text(report, "digest")?;
    if !hex_sha256(reported_digest) {
        return Err("Invalid source-report digest".into());
    }
    // serde_json::Value objects use sorted-key maps in the default build
    // (no preserve_order feature): interoperable with JavaScript canonicalJson.
    let body = json!({
        "schema": field(value, "schema")?,
        "source": source,
        "report": report,
    });
    let canonical = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    if canonical.len() > 1_000_000 {
        return Err("Witness exceeds size limit".into());
    }
    let expected = format!("{:x}", Sha256::digest(canonical.as_bytes()));
    let supplied = text(value, "payloadSha256")?;
    if !hex_sha256(supplied) || supplied != expected {
        return Err("CBC transport digest mismatch".into());
    }
    if field(report, "deterministic")?.as_bool() != Some(true)
        || field(report, "minimalFinalizingDistance")?.as_u64() != Some(1)
    {
        return Err("Unsupported bounded M27 report".into());
    }
    let candidates = field(report, "minimalFinalizingWitnesses")?
        .as_array().ok_or("Missing witness array")?;
    let candidate = candidates.first().ok_or("Empty minimum-witness array")?;
    let ids = field(candidate, "justifications")?
        .as_array().ok_or("Invalid justification IDs")?;
    let senders = field(candidate, "minimumMessageSenders")?
        .as_array().ok_or("Invalid sender IDs")?;
    if ids.len() != 4 || senders.len() != 4 {
        return Err("Invalid four-message witness cardinality".into());
    }
    let mut justifications = Vec::with_capacity(4);
    let mut sender_ids = Vec::with_capacity(4);
    for (id, sender) in ids.iter().zip(senders.iter()) {
        let id = id.as_str().ok_or("Invalid justification ID")?;
        let sender = sender.as_str().ok_or("Invalid sender ID")?;
        let expected_sender = match id {
            "a3" => "v0",
            "b3" => "v1",
            "c3" => "v2",
            "d3" => "v3",
            _ => return Err("Unknown source-justification ID".into()),
        };
        if sender != expected_sender {
            return Err("Sender/justification mapping mismatch".into());
        }
        justifications.push(id.to_owned());
        sender_ids.push(sender.to_owned());
    }
    let distinct: BTreeSet<_> = sender_ids.iter().collect();
    let distance = justifications.iter()
        .zip(["a3", "b3", "c3", "d3"])
        .filter(|(actual, control)| actual != control)
        .count() as u64;
    if distinct.len() != 3 || distance != 1
        || field(candidate, "distanceFromControl")?.as_u64() != Some(1)
        || field(candidate, "distinctMinimumSenders")?.as_u64() != Some(3)
        || text(candidate, "classification")? != "UNDER_CARDINALITY"
        || field(candidate, "senderCoverage")?.as_bool() != Some(false)
        || field(candidate, "currentCountGate")?.as_bool() != Some(true)
        || field(candidate, "reachable")?.as_bool() != Some(true)
        || field(candidate, "finalized")?.as_bool() != Some(true)
    {
        return Err("Unsupported bounded minimum-witness structure".into());
    }

    Ok(CbcWitnessObservation {
        schema: "aria-sentinel-cbc-observation/v1",
        source_revision: SOURCE_COMMIT.to_owned(),
        upstream_revision: UPSTREAM.to_owned(),
        transport_digest: expected,
        source_report_digest: reported_digest.to_owned(),
        justification_ids: justifications,
        reported_sender_ids: sender_ids,
        reported_minimum_distance: distance,
        source_reported_reachable: true,
        source_reported_finalized: true,
        integrity_checked: true,
        producer_authenticated: false,
        independently_verified_finality: false,
        live_network: false,
        evidence_class: "EXTERNAL_RESEARCH_REPORT_INTEGRITY_ONLY",
        claim_boundary: "Offline import of source-reported bounded M27 results. A matching hash and structurally valid witness do not authenticate the producer or independently verify RChain Casper finality or live-network safety.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope() -> Value {
        let source = json!({
            "repository": "aixaria0/RCHAIN-COMPLIER",
            "commit": SOURCE_COMMIT,
            "upstreamRevision": UPSTREAM,
        });
        let report = json!({
            "milestone": "M27", "upstreamRevision": UPSTREAM, "digest": "a".repeat(64),
            "deterministic": true, "minimalFinalizingDistance": 1,
            "minimalFinalizingWitnesses": [{
                "justifications": ["a3", "a3", "c3", "d3"],
                "minimumMessageSenders": ["v0", "v0", "v2", "v3"],
                "distanceFromControl": 1, "distinctMinimumSenders": 3,
                "classification": "UNDER_CARDINALITY",
                "reachable": true, "finalized": true,
                "currentCountGate": true, "senderCoverage": false
            }]
        });
        let body = json!({"schema": "aria-cbc-witness/v1", "source": source, "report": report});
        let digest = format!("{:x}", Sha256::digest(serde_json::to_string(&body).unwrap().as_bytes()));
        let mut envelope = body;
        envelope["payloadSha256"] = Value::String(digest);
        envelope
    }

    #[test]
    fn imports_bounded_witness_without_elevating_reported_claims() {
        let result = inspect(&envelope()).unwrap();
        assert_eq!(result.reported_sender_ids, vec!["v0", "v0", "v2", "v3"]);
        assert_eq!(result.reported_minimum_distance, 1);
        assert!(result.integrity_checked);
        assert!(!result.producer_authenticated);
        assert!(!result.independently_verified_finality);
        assert!(!result.live_network);
    }

    #[test]
    fn rejects_tampered_bytes() {
        let mut value = envelope();
        value["report"]["minimalFinalizingWitnesses"][0]["justifications"][0] = json!("b3");
        assert!(inspect(&value).unwrap_err().contains("digest mismatch"));
    }

    #[test]
    fn rejects_rehashed_inconsistent_sender_mapping() {
        let mut value = envelope();
        value["report"]["minimalFinalizingWitnesses"][0]["minimumMessageSenders"][0] = json!("v2");
        let body = json!({"schema": value["schema"], "source": value["source"], "report": value["report"]});
        value["payloadSha256"] = json!(format!("{:x}", Sha256::digest(serde_json::to_string(&body).unwrap().as_bytes())));
        assert!(inspect(&value).unwrap_err().contains("mapping mismatch"));
    }

    #[test]
    fn rejects_forged_source_revision_even_if_rehashed() {
        let mut value = envelope();
        value["source"]["commit"] = json!("f".repeat(40));
        let body = json!({"schema": value["schema"], "source": value["source"], "report": value["report"]});
        value["payloadSha256"] = json!(format!("{:x}", Sha256::digest(serde_json::to_string(&body).unwrap().as_bytes())));
        assert!(inspect(&value).unwrap_err().contains("Unsupported pinned source"));
    }
}
