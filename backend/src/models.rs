use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct HealthResponse { pub status: &'static str, pub service: &'static str, pub version: &'static str }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RNodeStatusPayload { #[serde(default)] pub node: Option<RNodeIdentity>, #[serde(default)] pub network_id: Option<String>, #[serde(default)] pub shard_id: Option<String>, #[serde(default)] pub peers: Option<Value>, #[serde(default)] pub last_finalized_block_number: Option<u64>, #[serde(default)] pub validator: Option<bool>, #[serde(default)] pub read_only: Option<bool>, #[serde(default)] pub ready: Option<bool>, #[serde(default)] pub current_epoch: Option<u64> }
#[derive(Debug, Serialize)]
pub struct NetworkStatus { pub reachable: bool, pub node_url: String, pub latency_ms: Option<u128>, pub http_status: Option<u16>, pub probe: String, pub error: Option<String>, pub rnode: Option<RNodeStatusPayload> }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RNodeIdentity { #[serde(default)] pub id: Option<String>, #[serde(default)] pub host: Option<String>, #[serde(default)] pub port: Option<u16> }
#[derive(Debug, Clone, Serialize)]
pub struct RNodeObservation { pub node_id: Option<String>, pub host: Option<String>, pub port: Option<u16>, pub network_id: Option<String>, pub shard_id: Option<String>, pub ready: Option<bool>, pub validator: Option<bool>, pub read_only: Option<bool>, pub current_epoch: Option<u64>, pub finalized_block: Option<u64>, pub peer_count: Option<usize> }
impl RNodeObservation { pub fn from_status(status: &RNodeStatusPayload) -> Self { let peer_count = status.peers.as_ref().map(|peers| match peers { Value::Array(items) => items.len(), Value::Object(map) => map.len(), _ => 0 }); let (node_id, host, port) = match &status.node { Some(node) => (node.id.clone(), node.host.clone(), node.port), None => (None, None, None) }; Self { node_id, host, port, network_id: status.network_id.clone(), shard_id: status.shard_id.clone(), ready: status.ready, validator: status.validator, read_only: status.read_only, current_epoch: status.current_epoch, finalized_block: status.last_finalized_block_number, peer_count } } }
#[derive(Debug, Clone, Serialize)]
pub struct FinalizedBlockEvidence { pub available: bool, pub raw: Option<Value>, pub payload_sha256: Option<String>, pub block_hash: Option<String>, pub parent_hash: Option<String>, pub proposer: Option<String>, pub signature: Option<String>, pub justification_present: bool, pub error: Option<String> }
impl FinalizedBlockEvidence { pub fn unavailable(error: impl Into<String>) -> Self { Self { available: false, raw: None, payload_sha256: None, block_hash: None, parent_hash: None, proposer: None, signature: None, justification_present: false, error: Some(error.into()) } } pub fn available(raw: Value) -> Self { Self { available: true, raw: Some(raw), payload_sha256: None, block_hash: None, parent_hash: None, proposer: None, signature: None, justification_present: false, error: None } } pub fn with_sha256(mut self, digest: String) -> Self { self.payload_sha256 = Some(digest); self } pub fn with_block_fields(mut self, block_hash: Option<String>, parent_hash: Option<String>, proposer: Option<String>, signature: Option<String>, justification_present: bool) -> Self { self.block_hash = block_hash; self.parent_hash = parent_hash; self.proposer = proposer; self.signature = signature; self.justification_present = justification_present; self } }
#[derive(Debug, Clone, Serialize)]
pub struct CrossNodeAgreement { pub node_url: String, pub reachable: bool, pub finalized_height: Option<u64>, pub block_hash: Option<String>, pub payload_sha256: Option<String>, pub proposer: Option<String>, pub signature_present: bool, pub justification_present: bool }
#[derive(Debug, Clone, Serialize)]
pub struct CrossNodeReport { pub target_count: usize, pub reachable_count: usize, pub evidence_count: usize, pub agreeing_nodes: usize, pub quorum_required: usize, pub quorum_observed: bool, pub agreement_ratio: f64, pub common_finalized_height: Option<u64>, pub common_block_hash: Option<String>, pub height_agreement: bool, pub hash_agreement: bool, pub missing_height_nodes: usize, pub missing_hash_nodes: usize, pub conflicting_nodes: usize, pub agreement: bool, pub status: String, pub verification_basis: String, pub observations: Vec<CrossNodeAgreement> }
#[derive(Debug, Clone, Serialize)]
pub struct CasperEvidenceReport { pub evidence_available: bool, pub protocol_block_shape: bool, pub validator_identity_present: bool, pub stake_weight_present: bool, pub bond_count: usize, pub total_observed_stake: Option<i64>, pub bet_present: bool, pub justification_present: bool, pub justification_count: usize, pub equivocation_signal: bool, pub recognized_fields: Vec<String>, pub status: String, pub verification_basis: String }
#[derive(Debug, Clone, Serialize)]
pub enum VerificationStatus { Pass, Warn, Fail }
#[derive(Debug, Clone, Serialize)]
pub enum CheckSeverity { Info, Warning, Critical }
#[derive(Debug, Clone, Serialize)]
pub struct VerificationEvidence { pub source: String, pub field: String, pub value: String }
#[derive(Debug, Clone, Serialize)]
pub struct VerificationCheck { pub name: String, pub status: VerificationStatus, pub message: String, pub severity: CheckSeverity, pub evidence: Vec<VerificationEvidence> }
#[derive(Debug, Clone, Serialize)]
pub struct VerificationReport { pub target: String, pub status: VerificationStatus, pub checks: Vec<VerificationCheck>, pub evidence_count: usize }
