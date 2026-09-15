use serde::Serialize;

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
}
