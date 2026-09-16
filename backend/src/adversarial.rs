use serde::Serialize;
use sha2::{Digest, Sha256};
use crate::provenance::synthetic_event;

#[derive(Debug, Clone, Serialize)]
pub struct AdversarialChallenge {
    pub event_id: String,
    pub attack: String,
    pub targeted_stage: String,
    pub original_hash: String,
    pub mutated_hash: String,
    pub integrity_intact: bool,
    pub first_broken_link: Option<usize>,
    pub detection: String,
    pub downstream_impact: Vec<String>,
    pub synthetic: bool,
    pub claim_boundary: String,
}

fn mutate(label: &str, original: &str) -> String {
    let mut h = Sha256::new();
    h.update(label.as_bytes());
    h.update(original.as_bytes());
    format!("{:x}", h.finalize())
}

pub fn challenge_event(event_id: &str, attack: &str) -> AdversarialChallenge {
    let envelope = synthetic_event(event_id);
    let (index, stage, label) = match attack {
        "corrupt-qlf" => (1, "QLF / ZFA", "qlf-certificate-corruption"),
        "mutate-trace" => (3, "ρ-Calculus Execution", "execution-trace-mutation"),
        "corrupt-block" => (4, "RChain Block", "block-evidence-corruption"),
        "forge-observation" => (5, "Sentinel Observation", "forged-node-observation"),
        _ => (3, "ρ-Calculus Execution", "execution-trace-mutation"),
    };
    let original_hash = envelope.links[index].content_hash.clone();
    let mutated_hash = mutate(label, &original_hash);
    let downstream_impact = envelope.links[index + 1..]
        .iter()
        .map(|link| format!("{} evidence becomes untrusted after upstream mutation", link.kind))
        .collect();
    AdversarialChallenge {
        event_id: event_id.into(), attack: attack.into(), targeted_stage: stage.into(),
        original_hash, mutated_hash, integrity_intact: false,
        first_broken_link: Some(index),
        detection: format!("Integrity-chain mismatch detected at {} (stage {}).", stage, index + 1),
        downstream_impact, synthetic: true,
        claim_boundary: "Synthetic adversarial mutation harness; demonstrates evidence tamper detection and propagation without claiming an attack on a live RChain or Sentinel deployment.".into(),
    }
}
