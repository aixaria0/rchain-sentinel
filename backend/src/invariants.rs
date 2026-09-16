use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct InvariantCheck {
    pub id: String,
    pub stage: String,
    pub predicate: String,
    pub status: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InvariantTrace {
    pub event_id: String,
    pub checks: Vec<InvariantCheck>,
    pub all_passed: bool,
    pub synthetic: bool,
    pub claim_boundary: String,
}

pub fn invariant_trace(event_id: &str) -> InvariantTrace {
    let checks = vec![
        InvariantCheck { id: "INV-001".into(), stage: "QLF / ZFA".into(), predicate: "ZFA balance is preserved by the certificate representation".into(), status: "PASS".into(), evidence: "zfa-balanced | phase=+-+-".into() },
        InvariantCheck { id: "INV-002".into(), stage: "Rholang Process".into(), predicate: "Normalized process is bound to the QLF certificate context".into(), status: "PASS".into(), evidence: "qlf-7A91 → deploy-7A91".into() },
        InvariantCheck { id: "INV-003".into(), stage: "ρ-Calculus Execution".into(), predicate: "Execution trace is hash-linked to the normalized process".into(), status: "PASS".into(), evidence: "trace-7A91".into() },
        InvariantCheck { id: "INV-004".into(), stage: "RChain Block".into(), predicate: "Block evidence is causally linked to the execution trace".into(), status: "PASS".into(), evidence: "18492".into() },
        InvariantCheck { id: "INV-005".into(), stage: "Sentinel Observation".into(), predicate: "Observation references the represented block evidence".into(), status: "PASS".into(), evidence: "node-a:18492".into() },
        InvariantCheck { id: "INV-006".into(), stage: "Sovereign Lattice".into(), predicate: "Independent analysis receives an intact evidence chain".into(), status: "PASS".into(), evidence: "lattice-check-7A91".into() },
    ];
    InvariantTrace { event_id: event_id.into(), all_passed: true, checks, synthetic: true, claim_boundary: "Synthetic invariant trace over modeled provenance; PASS means the modeled predicates hold, not that production RChain finality or a live Lean proof has been established.".into() }
}
