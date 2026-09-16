use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DemoStage { pub index: usize, pub stage: String, pub state: String, pub evidence: String, pub status: String }

#[derive(Debug, Clone, Serialize)]
pub struct KillerDemo { pub event_id: String, pub mode: String, pub stages: Vec<DemoStage>, pub live: bool, pub claim_boundary: String }

pub fn killer_demo(event_id: &str) -> KillerDemo {
    let stages = vec![
        ("Origin Event", "QuantumOS interaction received", "event:origin"),
        ("QLF / ZFA", "ZFA-balanced certificate attached", "qlf-7A91"),
        ("Rholang Process", "Normalized process bound to deploy", "deploy-7A91"),
        ("ρ-Calculus Execution", "Deterministic reduction trace recorded", "trace-7A91"),
        ("RChain Block", "Execution committed to block evidence", "18492"),
        ("Sentinel Observation", "Node observation attached", "node-a:18492"),
        ("Sovereign Lattice", "Independent invariant analysis completed", "lattice-check-7A91"),
    ].into_iter().enumerate().map(|(index, (stage, state, evidence))| DemoStage { index, stage: stage.into(), state: state.into(), evidence: evidence.into(), status: "VERIFIED IN MODEL".into() }).collect();
    KillerDemo { event_id: event_id.into(), mode: "KILLER_DEMO".into(), stages, live: false, claim_boundary: "Demonstration mode over deterministic synthetic evidence. It does not claim live QuantumOS, RChain, Sentinel, or Sovereign Lattice production connectivity.".into() }
}
