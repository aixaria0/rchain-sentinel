use serde::{Deserialize, Serialize};

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
    pub peers: Option<serde_json::Value>,

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
