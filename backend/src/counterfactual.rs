use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize)]
pub struct CounterfactualReport {
    pub event_id: String,
    pub scenario: String,
    pub baseline_root_hash: String,
    pub counterfactual_root_hash: String,
    pub first_affected_stage: String,
    pub first_affected_index: usize,
    pub baseline_hash: String,
    pub counterfactual_hash: String,
    pub downstream_affected_stages: Vec<String>,
    pub invariant_status: String,
    pub synthetic: bool,
    pub claim_boundary: String,
}

fn hash(label: &str, parent: &str) -> String {
    let mut h = Sha256::new();
    h.update(label.as_bytes());
    h.update(parent.as_bytes());
    format!("{:x}", h.finalize())
}

pub fn counterfactual_execution(event_id: &str, scenario: &str) -> CounterfactualReport {
    let stages = ["Origin Event", "QLF / ZFA", "Rholang Process", "ρ-Calculus Execution", "RChain Block", "Sentinel Observation", "Sovereign Lattice"];
    let mut baseline = Vec::with_capacity(stages.len());
    let mut parent = hash(&format!("event:{event_id}"), "root");
    baseline.push(parent.clone());
    for stage in stages.iter().skip(1) {
        parent = hash(stage, &parent);
        baseline.push(parent.clone());
    }
    let normalized = scenario.trim().to_ascii_lowercase();
    let (index, premise) = if normalized.contains("capability") {
        (1usize, "counterfactual:capability-absent")
    } else if normalized.contains("replay") {
        (3usize, "counterfactual:replayed-deploy")
    } else {
        (3usize, "counterfactual:alternate-reduction")
    };
    let mut alternate = baseline.clone();
    let previous = if index == 0 { "root" } else { &alternate[index - 1] };
    alternate[index] = hash(premise, previous);
    for i in (index + 1)..alternate.len() {
        alternate[i] = hash(&format!("counterfactual:stage:{i}"), &alternate[i - 1]);
    }
    CounterfactualReport {
        event_id: event_id.into(),
        scenario: scenario.into(),
        baseline_root_hash: baseline.last().cloned().unwrap_or_default(),
        counterfactual_root_hash: alternate.last().cloned().unwrap_or_default(),
        first_affected_stage: stages[index].into(),
        first_affected_index: index,
        baseline_hash: baseline[index].clone(),
        counterfactual_hash: alternate[index].clone(),
        downstream_affected_stages: stages[index..].iter().map(|s| (*s).into()).collect(),
        invariant_status: if index == 1 { "CAPABILITY INVARIANT VIOLATED IN COUNTERFACTUAL".into() } else { "EXECUTION PROVENANCE INVARIANT VIOLATED IN COUNTERFACTUAL".into() },
        synthetic: true,
        claim_boundary: "Synthetic deterministic counterfactual model; it demonstrates causal propagation of a changed premise and does not claim a live node replay or production invariant verdict.".into(),
    }
}
