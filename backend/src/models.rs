use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Serialize)]
pub struct NetworkStatus {
    pub reachable: bool,
    pub node_url: String,
    pub latency_ms: Option<u128>,
    pub http_status: Option<u16>,
    pub probe: String,
    pub error: Option<String>,
    pub rnode: Option<RNodeStatusPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RNodeStatusPayload {
    #[serde(default)]
    pub node: Option<RNodeIdentity>,

    #[serde(default)]
    pub network_id: Option<String>,

    #[serde(default)]
    pub shard_id: Option<String>,

    #[serde(default)]
    pub peers: Option<Value>,

    #[serde(default)]
    pub last_finalized_block_number: Option<u64>,

    #[serde(default)]
    pub validator: Option<bool>,

    #[serde(default)]
    pub read_only: Option<bool>,

    #[serde(default)]
    pub ready: Option<bool>,

    #[serde(default)]
    pub current_epoch: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RNodeIdentity {
    #[serde(default)]
    pub id: Option<String>,

    #[serde(default)]
    pub host: Option<String>,

    #[serde(default)]
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RNodeObservation {
    pub node_id: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub network_id: Option<String>,
    pub shard_id: Option<String>,
    pub ready: Option<bool>,
    pub validator: Option<bool>,
    pub read_only: Option<bool>,
    pub current_epoch: Option<u64>,
    pub finalized_block: Option<u64>,
    pub peer_count: Option<usize>,
}

impl RNodeObservation {
    pub fn from_status(status: &RNodeStatusPayload) -> Self {
        let peer_count = status.peers.as_ref().map(|peers| match peers {
            Value::Array(items) => items.len(),
            Value::Object(map) => map.len(),
            _ => 0,
        });

        let (node_id, host, port) = match &status.node {
            Some(node) => (
                node.id.clone(),
                node.host.clone(),
                node.port,
            ),
            None => (None, None, None),
        };

        Self {
            node_id,
            host,
            port,
            network_id: status.network_id.clone(),
            shard_id: status.shard_id.clone(),
            ready: status.ready,
            validator: status.validator,
            read_only: status.read_only,
            current_epoch: status.current_epoch,
            finalized_block: status.last_finalized_block_number,
            peer_count,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FinalizedBlockEvidence {
    pub available: bool,
    pub raw: Option<Value>,
    pub error: Option<String>,
}

impl FinalizedBlockEvidence {
    pub fn unavailable(error: impl Into<String>) -> Self {
        Self {
            available: false,
            raw: None,
            error: Some(error.into()),
        }
    }

    pub fn available(raw: Value) -> Self {
        Self {
            available: true,
            raw: Some(raw),
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum VerificationStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationCheck {
    pub name: String,
    pub status: VerificationStatus,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationReport {
    pub target: String,
    pub status: VerificationStatus,
    pub checks: Vec<VerificationCheck>,
}
