use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AdapterDescriptor {
    pub name: String,
    pub layer: String,
    pub transport: String,
    pub live: bool,
    pub endpoint: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdapterRegistry {
    pub adapters: Vec<AdapterDescriptor>,
    pub live_ready: bool,
    pub claim_boundary: String,
}

pub fn adapter_registry() -> AdapterRegistry {
    AdapterRegistry {
        adapters: vec![
            AdapterDescriptor { name: "QuantumOS Event Adapter".into(), layer: "origin-event".into(), transport: "trait boundary".into(), live: false, endpoint: None, status: "interface-ready".into() },
            AdapterDescriptor { name: "QLF Certificate Adapter".into(), layer: "qlf-zfa".into(), transport: "certificate boundary".into(), live: false, endpoint: None, status: "interface-ready".into() },
            AdapterDescriptor { name: "RChain RNode Adapter".into(), layer: "rchain-state".into(), transport: "HTTP".into(), live: false, endpoint: Some("RCHAIN_RNODE_URL".into()), status: "configuration-ready".into() },
            AdapterDescriptor { name: "Sentinel Observation Adapter".into(), layer: "evidence".into(), transport: "observation boundary".into(), live: false, endpoint: None, status: "interface-ready".into() },
            AdapterDescriptor { name: "Sovereign Lattice Adapter".into(), layer: "independent-verification".into(), transport: "certificate boundary".into(), live: false, endpoint: None, status: "interface-ready".into() },
        ],
        live_ready: true,
        claim_boundary: "Adapters define stable integration boundaries; they do not imply that upstream production systems are currently connected.".into(),
    }
}
