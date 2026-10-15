//! Narrow read-only rchain-assurance/v1 producer. No verifier policy lives here.
use crate::rnode::RNodeClient;
use futures::future::join_all;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub fn now_ms() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64 }
pub fn digest(value: &Value) -> String { format!("{:x}", Sha256::digest(serde_json::to_vec(value).expect("JSON serialization"))) }
fn body(raw: &Value) -> Value { raw["body"].as_str().and_then(|s| serde_json::from_str(s).ok()).unwrap_or(Value::Null) }
fn info(value: &Value) -> &Value { value.get("blockInfo").unwrap_or(value) }
fn hash(value: &Value) -> Option<&str> { info(value)["blockHash"].as_str() }
fn height(value: &Value) -> Option<u64> { info(value)["blockNumber"].as_u64() }
fn finalized(value: &Value) -> bool { value.as_bool() == Some(true) || ["finalized", "isFinalized", "is_finalized", "value"].iter().any(|k| value[*k].as_bool() == Some(true)) }
fn protocol_matches(a: &Value, b: &Value) -> bool { ["blockHash", "blockNumber", "shardId", "sender", "seqNum", "preStateHash", "postStateHash", "justifications", "bonds", "sigAlgorithm", "sig"].iter().all(|k| !info(a)[*k].is_null() && info(a)[*k] == info(b)[*k]) }
fn bool_result(v: &Value) -> Option<bool> {
    let first = v.as_array()?.first()?;
    let items = first["ExprTuple"]["ps"].as_array()?;
    let result = items.first()?.get("ExprBool")?;
    result.as_bool().or_else(|| result["data"].as_bool())
}
fn transfer_recipient(term: &str) -> Option<String> {
    let rest = term.split("revVault!(\"transfer\", *deployerId, \"").nth(1)?;
    let address = rest.split('"').next()?;
    if address.is_empty() || address.len() > 128 || !address.bytes().all(|b| b.is_ascii_alphanumeric()) { return None; }
    Some(address.into())
}
fn balance_term(address: &str) -> String { format!("new return, revVault(`rho:rchain:revVault`), balanceCh in {{ revVault!(\"getBalance\", \"{}\", *balanceCh) | for (@balance <- balanceCh) {{ return!(balance) }} }}", address) }
fn balance_value(v: &Value) -> Option<u64> { let n = &v["expr"][0]["ExprInt"]; n.as_u64().or_else(|| n["data"].as_u64()) }
fn evidence_ok(raw: &Value) -> bool { raw["body"].as_str().is_some_and(|s| raw["digest"] == format!("{:x}", Sha256::digest(s.as_bytes()))) }
fn parents(value: &Value) -> Vec<String> { let b = info(value); ["parents", "parentsHash"].iter().filter_map(|k| b[*k].as_array()).flatten().filter_map(Value::as_str).map(str::to_string).chain(b["parentHash"].as_str().map(str::to_string)).collect() }

async fn observer(deploy: &str, source: &str, domain: Option<&str>) -> Value {
    let client = RNodeClient::new(source);
    let deploy_path = format!("/api/v1/deploy-status/{}", deploy);
    let (status, deploy_raw, tip) = tokio::join!(client.assurance_raw("/api/status", None), client.assurance_raw(&deploy_path, None), client.assurance_raw("/api/last-finalized-block", None));
    let mut evidence = json!({"status": status.ok(), "deploy": deploy_raw.ok(), "block": null, "finalized": tip.ok(), "finalized_block": null, "is_finalized": null, "balance": null, "ancestry": []});
    let state = body(&evidence["deploy"]);
    let included = &state["ProcessedWithSuccess"]["block"];
    let errored = &state["ProcessedWithError"]["block"];
    let block = if hash(included).is_some() { included } else { errored };
    if let Some(h) = hash(block) {
        let block_path = format!("/api/block/{}", urlencoding::encode(h)); let finality_path = format!("/api/is-finalized/{}", urlencoding::encode(h));
        let (canonical, assertion) = tokio::join!(client.assurance_raw(&block_path, None), client.assurance_raw(&finality_path, None));
        evidence["block"] = canonical.ok().unwrap_or(Value::Null); evidence["is_finalized"] = assertion.ok().unwrap_or(Value::Null);
        let canonical = body(&evidence["block"]);
        let deploy_info = canonical["deploys"].as_array().and_then(|ds| ds.iter().find(|d| d["sig"] == deploy));
        if let Some(recipient) = deploy_info.and_then(|d| d["term"].as_str()).and_then(transfer_recipient) {
            evidence["balance"] = client.assurance_raw("/api/explore-deploy", Some(&balance_term(&recipient))).await.ok().unwrap_or(Value::Null);
        }
    }
    let finalized_block = body(&evidence["finalized"]);
    if let Some(h) = hash(&finalized_block) { evidence["finalized_block"] = client.assurance_raw(&format!("/api/block/{}", urlencoding::encode(h)), None).await.ok().unwrap_or(Value::Null); }
    let status = body(&evidence["status"]);
    let identity = status["node"]["id"].as_str().or_else(|| status["address"].as_str());
    json!({"observer_id": source, "source": source, "node_identity": identity, "failure_domain": domain, "observed_at": now_ms(), "finalized_height": height(&finalized_block), "finalized_hash": hash(&finalized_block), "status": if identity.is_some() {"OBSERVED"} else {"UNAVAILABLE"}, "reason": if identity.is_some() {Value::Null} else {json!("Node identity/status unavailable")}, "evidence": evidence, "observation_digest": ""})
}

fn canonical_contradiction(o: &Value) -> bool {
    let e = &o["evidence"];
    for key in ["status", "deploy", "block", "finalized", "finalized_block", "is_finalized", "balance"] { if !e[key].is_null() && !evidence_ok(&e[key]) { return true; } }
    if e["ancestry"].as_array().is_some_and(|a| a.iter().any(|r| !evidence_ok(r))) { return true; }
    let tip=body(&e["finalized"]);
    if !e["finalized"].is_null() && (json!(hash(&tip)) != o["finalized_hash"] || json!(height(&tip)) != o["finalized_height"]) { return true; }
    let d = body(&e["deploy"]);
    let b = d["ProcessedWithSuccess"].get("block").or_else(|| d["ProcessedWithError"].get("block"));
    (!e["finalized"].is_null() && !e["finalized_block"].is_null() && !protocol_matches(&body(&e["finalized"]), &body(&e["finalized_block"]))) || b.is_some_and(|b| !e["block"].is_null() && !protocol_matches(b, &body(&e["block"])))
}
fn has_readback(o: &Value, deploy_id: &str) -> bool {
    let e = &o["evidence"]; let tip = body(&e["finalized"]); let balance = body(&e["balance"]); let deploy = body(&e["deploy"]);
    let canonical = body(&e["block"]); let included = canonical["deploys"].as_array().is_some_and(|ds| ds.iter().any(|d| d["sig"] == deploy_id && d["errored"] == false));
    included && !canonical_contradiction(o) && !e["finalized_block"].is_null() && bool_result(&deploy["ProcessedWithSuccess"]["deployResult"]) == Some(true) && finalized(&body(&e["is_finalized"])) && balance_value(&balance).is_some() && hash(&tip).is_some() && hash(&tip) == hash(&balance["block"]) && height(&tip) == height(&balance["block"])
}
fn ancestor(high: &Value, target: &str) -> bool {
    let e = &high["evidence"]; let mut pending = vec![body(&e["finalized_block"])]; let mut seen = std::collections::HashSet::new();
    while let Some(block) = pending.pop() {
        let Some(h) = hash(&block) else {continue}; if h == target {return true;} if !seen.insert(h.to_string()) {continue;}
        for parent in parents(&block) { if parent == target {return true;} if let Some(raw) = e["ancestry"].as_array().and_then(|a| a.iter().find(|raw| hash(&body(raw)) == Some(parent.as_str()))) { if evidence_ok(raw) {pending.push(body(raw));} } }
    }
    false
}
pub fn agreement(observers: &[Value], now: u64) -> &'static str {
    for (i,a) in observers.iter().enumerate() {
        if canonical_contradiction(a) {return "CONFLICT";}
        for b in &observers[i+1..] { if a["finalized_height"].is_u64() && a["finalized_height"] == b["finalized_height"] && a["finalized_hash"].is_string() && b["finalized_hash"].is_string() && a["finalized_hash"] != b["finalized_hash"] {return "CONFLICT";} }
    }
    if observers.len() < 2 || observers.iter().any(|o| o["status"] != "OBSERVED" || o["finalized_height"].is_null() || o["finalized_hash"].is_null() || o["observed_at"].as_u64().is_none_or(|t| now.saturating_sub(t) > 60_000)) { return "INSUFFICIENT_EVIDENCE"; }
    if observers.iter().all(|o| o["finalized_height"] == observers[0]["finalized_height"] && o["finalized_hash"] == observers[0]["finalized_hash"]) {return "EXACT_AGREEMENT";}
    for (i,a) in observers.iter().enumerate() { for b in &observers[i+1..] { if a["finalized_height"] != b["finalized_height"] { let (low, high) = if a["finalized_height"].as_u64() < b["finalized_height"].as_u64() {(a,b)} else {(b,a)}; if !ancestor(high, low["finalized_hash"].as_str().unwrap_or("")) {return "HEIGHT_SKEW";} } } }
    "PROVEN_LAG"
}

pub fn package(deploy: &str, mut observers: Vec<Value>, generated_at: u64, revision: &str) -> Value {
    let agreement = agreement(&observers, generated_at);
    let first = observers.first().cloned().unwrap_or(Value::Null); let d = body(&first["evidence"]["deploy"]); let status = body(&first["evidence"]["status"]);
    let processed = &d["ProcessedWithSuccess"]; let block = processed.get("block").or_else(|| d["ProcessedWithError"].get("block"));
    let raw_block = body(&first["evidence"]["block"]);
    let deploy_info = raw_block["deploys"].as_array().and_then(|a| a.iter().find(|d| d["sig"] == deploy));
    let included = block.is_some() && deploy_info.is_some();
    let execution = if !d["ProcessedWithError"].is_null() || bool_result(&processed["deployResult"]) == Some(false) {"FAILED"} else if included && bool_result(&processed["deployResult"]) == Some(true) {"OBSERVED"} else {"UNKNOWN"};
    let mut stage = json!({"status": if included {"OBSERVED"} else {"UNKNOWN"}});
    if included { let b=block.unwrap(); stage["block_hash"]=json!(hash(b)); stage["block_height"]=json!(height(b)); stage["observed_at"]=first["evidence"]["deploy"]["observed_at"].clone(); }
    let mut execution_stage=stage.clone(); execution_stage["status"]=json!(execution);
    let finalized_state = if agreement == "CONFLICT" {json!({"status":"CONFLICT", "reason":"Conflicting finalized or canonical evidence"})} else if let Some(o) = observers.iter().find(|o| has_readback(o, deploy)) {json!({"status":"OBSERVED","observed_at":o["evidence"]["balance"]["observed_at"],"finalized_block_hash":o["finalized_hash"],"finalized_block_height":o["finalized_height"]})} else {json!({"status":"UNAVAILABLE", "reason":"Finalized transfer read-back not observed"})};
    for o in &mut observers { let mut copy=o.clone(); copy.as_object_mut().unwrap().remove("observation_digest"); o["observation_digest"]=json!(digest(&copy)); }
    let summaries: Vec<Value> = observers.iter().cloned().map(|mut o| { let m=o.as_object_mut().unwrap(); m.remove("evidence"); m.remove("observation_digest"); o }).collect();
    let agreement = json!({"status":agreement});
    let reproduction = digest(&json!({"execution":execution_stage,"inclusion":stage,"finalized_state":finalized_state,"agreement":agreement,"observers":summaries}));
    let mut submission=json!({"status": if deploy_info.is_some() {"OBSERVED"} else {"UNKNOWN"}}); if let Some(signer)=deploy_info.and_then(|d|d["deployer"].as_str()) {submission["signer"]=json!(signer);}
    let mut result = json!({"version":"rchain-assurance/v1","subject":{"action_type":"REV_TRANSFER","action_id":deploy,"deploy_id":deploy},"network":{"network_id":status["networkId"],"shard_id":status["shardId"]},"submission":submission,"execution":execution_stage,"inclusion":stage,"finalized_state":finalized_state,"observers":observers,"agreement":agreement,"claim":{"claim_id":format!("rev-transfer:{}",deploy),"claim_type":"REV_TRANSFER_FINALIZED"},"verification":{"payload_integrity":"UNKNOWN","network_identity":"UNKNOWN","canonical_consistency":"UNKNOWN","cross_node_agreement":"UNKNOWN","finalized_state_observation":"UNKNOWN","deterministic_reproduction":"UNKNOWN"},"reproduction_digest":reproduction,"provenance":{"producer":"rchain-sentinel","source_revision":revision,"generated_at":generated_at}});
    let digest=digest(&result); result["provenance"]["payload_digest"]=json!(digest); result
}

pub async fn observe(deploy: &str, sources: &[String]) -> Value {
    let domains = std::env::var("RCHAIN_ASSURANCE_FAILURE_DOMAINS").unwrap_or_default(); let domains:Vec<&str>=domains.split(',').collect();
    let mut observations = join_all(sources.iter().take(16).enumerate().map(|(i,s)| observer(deploy,s,domains.get(i).copied().filter(|d|!d.is_empty())))).await;
    // Bounded parent traversal; unsupported block representations stay HEIGHT_SKEW.
    let targets:Vec<String>=observations.iter().filter_map(|o|o["finalized_hash"].as_str().map(str::to_string)).collect();
    for o in &mut observations {
        let client=RNodeClient::new(o["source"].as_str().unwrap_or("")); let mut frontier=parents(&body(&o["evidence"]["finalized_block"])); let mut seen=std::collections::HashSet::new();
        for _ in 0..32 { let Some(h)=frontier.pop() else {break}; if targets.contains(&h) || !seen.insert(h.clone()) {continue;}
            if let Ok(raw)=client.assurance_raw(&format!("/api/block/{}",urlencoding::encode(&h)),None).await {frontier.extend(parents(&body(&raw)));o["evidence"]["ancestry"].as_array_mut().unwrap().push(raw);}
        }
    }
    package(deploy, observations, now_ms(), &std::env::var("RCHAIN_SENTINEL_REVISION").unwrap_or_else(|_|"unversioned".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<Value> { let p:Value=serde_json::from_str(include_str!("../tests/fixtures/assurance-pass.json")).unwrap();p["observers"].as_array().unwrap().clone() }
    fn replace_raw(raw:&mut Value, value:Value) { let body=serde_json::to_string(&value).unwrap();raw["digest"]=json!(format!("{:x}",Sha256::digest(body.as_bytes())));raw["body"]=json!(body); }
    const NOW:u64=1792080001000;
    #[test] fn same_height_different_hash_is_conflict() {let mut o=fixture();o[1]["finalized_hash"]=json!("other");assert_eq!(agreement(&o,NOW),"CONFLICT");}
    #[test] fn valid_parent_path_proves_lag() {let mut o=fixture();let low=o[0]["finalized_hash"].clone();let mut b=body(&o[1]["evidence"]["finalized_block"]);b["blockInfo"]["blockHash"]=json!("new");b["blockInfo"]["blockNumber"]=json!(11);b["blockInfo"]["parents"]=json!([low]);o[1]["finalized_height"]=json!(11);o[1]["finalized_hash"]=json!("new");replace_raw(&mut o[1]["evidence"]["finalized"],b.clone());replace_raw(&mut o[1]["evidence"]["finalized_block"],b);assert_eq!(agreement(&o,NOW),"PROVEN_LAG");}
    #[test] fn unresolved_height_is_skew() {let mut o=fixture();let mut b=body(&o[1]["evidence"]["finalized_block"]);b["blockInfo"]["blockHash"]=json!("new");b["blockInfo"]["blockNumber"]=json!(11);o[1]["finalized_height"]=json!(11);o[1]["finalized_hash"]=json!("new");replace_raw(&mut o[1]["evidence"]["finalized"],b.clone());replace_raw(&mut o[1]["evidence"]["finalized_block"],b);assert_eq!(agreement(&o,NOW),"HEIGHT_SKEW");}
    #[test] fn unavailable_required_observer_is_insufficient() {let mut o=fixture();o[1]["status"]=json!("UNAVAILABLE");assert_eq!(agreement(&o,NOW),"INSUFFICIENT_EVIDENCE");}
    #[test] fn stale_cannot_agree() {let o=fixture();assert_eq!(agreement(&o,NOW+60_000),"INSUFFICIENT_EVIDENCE");}
    #[test] fn canonical_mismatch_is_contradiction() {let mut o=fixture();let mut b=body(&o[0]["evidence"]["finalized_block"]);b["blockInfo"]["postStateHash"]=json!("tamper");replace_raw(&mut o[0]["evidence"]["finalized_block"],b);assert_eq!(agreement(&o,NOW),"CONFLICT");}
    #[test] fn tampered_bytes_rejected() {let mut o=fixture();o[0]["evidence"]["status"]["body"]=json!("{}");assert!(!evidence_ok(&o[0]["evidence"]["status"]));assert_eq!(agreement(&o,NOW),"CONFLICT");}
    #[test] fn fixtures_reproduce_canonical_contract() {let o=fixture();let p=package("fixture-deploy",o,NOW,"test");assert_eq!(p["agreement"]["status"],"EXACT_AGREEMENT");assert_eq!(p["execution"]["status"],"UNKNOWN");let mut copy=p.clone();copy["provenance"].as_object_mut().unwrap().remove("payload_digest");assert_eq!(p["provenance"]["payload_digest"],digest(&copy));}
    #[test] fn conflict_survives_unavailable_observer() {let mut o=fixture();let mut missing=o[0].clone();missing["status"]=json!("UNAVAILABLE");o[1]["finalized_hash"]=json!("conflict");o.push(missing);assert_eq!(agreement(&o,NOW),"CONFLICT");}
}
