use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize)]
pub struct EvidenceLink {
    pub kind: String,
    pub id: String,
    pub parent_id: Option<String>,
    pub content_hash: String,
    pub source: String,
    pub synthetic: bool,
    pub claim: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvidenceEnvelope {
    pub event_id: String,
    pub root_hash: String,
    pub synthetic: bool,
    pub links: Vec<EvidenceLink>,
    pub verification_basis: Vec<String>,
}

fn hash(label: &str, parent: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(label.as_bytes());
    if let Some(parent) = parent {
        hasher.update(parent.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

pub fn synthetic_event(event_id: &str) -> EvidenceEnvelope {
    let event_hash = hash(&format!("event:{event_id}"), None);
    let qlf_hash = hash("qlf:zfa-balanced:phase:+-+-", Some(&event_hash));
    let rholang_hash = hash("rholang:exchange-demo:v1", Some(&qlf_hash));
    let trace_hash = hash("rspace:reduction:0,1,2", Some(&rholang_hash));
    let block_hash = hash("rchain:block:synthetic-18492", Some(&trace_hash));
    let observation_hash = hash("sentinel:node-a-observation", Some(&block_hash));
    let lattice_hash = hash("sovereign-lattice:quorum-check", Some(&observation_hash));

    let links = vec![
        EvidenceLink {
            kind: "event".into(),
            id: event_id.into(),
            parent_id: None,
            content_hash: event_hash.clone(),
            source: "QuantumOS event boundary".into(),
            synthetic: true,
            claim: "Interaction origin represented as an event certificate.".into(),
        },
        EvidenceLink {
            kind: "qlf".into(),
            id: "qlf-7A91".into(),
            parent_id: Some(event_id.into()),
            content_hash: qlf_hash,
            source: "QLF event-level representation".into(),
            synthetic: true,
            claim: "ZFA-balanced phase representation is carried as logical evidence.".into(),
        },
        EvidenceLink {
            kind: "rholang".into(),
            id: "deploy-7A91".into(),
            parent_id: Some("qlf-7A91".into()),
            content_hash: rholang_hash,
            source: "Rholang process boundary".into(),
            synthetic: true,
            claim: "A deterministic Rholang exchange process is identified by hash.".into(),
        },
        EvidenceLink {
            kind: "execution".into(),
            id: "trace-7A91".into(),
            parent_id: Some("deploy-7A91".into()),
            content_hash: trace_hash,
            source: "ρ-calculus / rspace execution boundary".into(),
            synthetic: true,
            claim: "Reduction trace is represented without claiming a live replay.".into(),
        },
        EvidenceLink {
            kind: "block".into(),
            id: "18492".into(),
            parent_id: Some("trace-7A91".into()),
            content_hash: block_hash,
            source: "RChain block evidence boundary".into(),
            synthetic: true,
            claim: "Block provenance is linked to the execution trace.".into(),
        },
        EvidenceLink {
            kind: "observation".into(),
            id: "node-a:18492".into(),
            parent_id: Some("18492".into()),
            content_hash: observation_hash,
            source: "Sentinel observation boundary".into(),
            synthetic: true,
            claim: "Node observation is evidence, not an independent finality proof.".into(),
        },
        EvidenceLink {
            kind: "verification".into(),
            id: "lattice-check-7A91".into(),
            parent_id: Some("node-a:18492".into()),
            content_hash: lattice_hash.clone(),
            source: "Sovereign Lattice analysis boundary".into(),
            synthetic: true,
            claim: "Certificate/quorum invariants can be evaluated independently.".into(),
        },
    ];

    EvidenceEnvelope {
        event_id: event_id.into(),
        root_hash: lattice_hash,
        synthetic: true,
        links,
        verification_basis: vec![
            "Synthetic deterministic fixture; no live QuantumOS, RChain or Sentinel integration is claimed.".into(),
            "Each link preserves provenance through a cryptographic hash chain.".into(),
            "Layer boundaries describe evidence claims rather than cross-layer proof of finality.".into(),
        ],
    }
}
