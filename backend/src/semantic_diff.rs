use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize)]
pub struct SemanticChange {
    pub stage: String,
    pub kind: String,
    pub left_value: String,
    pub right_value: String,
    pub impact: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SemanticRealityDiff {
    pub left_event_id: String,
    pub right_event_id: String,
    pub common_semantic_prefix: usize,
    pub first_divergence_stage: Option<String>,
    pub first_divergence_index: Option<usize>,
    pub changes: Vec<SemanticChange>,
    pub left_root_hash: String,
    pub right_root_hash: String,
    pub synthetic: bool,
    pub claim_boundary: String,
}

fn hash(label: &str, parent: Option<&str>) -> String {
    let mut h = Sha256::new();
    h.update(label.as_bytes());
    if let Some(parent) = parent { h.update(parent.as_bytes()); }
    format!("{:x}", h.finalize())
}

fn value(stage: usize, variant: bool) -> String {
    match stage {
        0 => "event:shared-origin".into(),
        1 => "zfa-balanced | phase=+-+-".into(),
        2 => "rholang:exchange-demo:v1".into(),
        3 if variant => "reduction:0,1,3 | state=alternate".into(),
        3 => "reduction:0,1,2 | state=conserved".into(),
        4 if variant => "block:synthetic-18493".into(),
        4 => "block:synthetic-18492".into(),
        5 if variant => "sentinel:node-b-observation".into(),
        5 => "sentinel:node-a-observation".into(),
        6 if variant => "quorum-divergence".into(),
        6 => "quorum-check".into(),
        _ => "unknown".into(),
    }
}

fn stage_name(index: usize) -> &'static str {
    ["Origin Event", "QLF / ZFA", "Rholang Process", "ρ-Calculus Execution", "RChain Block", "Sentinel Observation", "Sovereign Lattice"][index]
}

pub fn semantic_diff(left_event_id: &str, right_event_id: &str) -> SemanticRealityDiff {
    let mut left_hash = hash(&value(0, false), None);
    let mut right_hash = left_hash.clone();
    let mut common = 0usize;
    let mut first = None;
    let mut changes = Vec::new();

    for i in 0..7 {
        let lv = value(i, false);
        let rv = value(i, i >= 3);
        let lh = hash(&lv, Some(&left_hash));
        let rh = hash(&rv, Some(&right_hash));
        let same = lv == rv;
        if same { common += 1; } else if first.is_none() {
            first = Some(i);
        }
        if !same {
            changes.push(SemanticChange {
                stage: stage_name(i).into(),
                kind: if i == 3 { "execution divergence" } else if i == 4 { "state commitment change" } else if i == 5 { "observation divergence" } else { "downstream propagation" }.into(),
                left_value: lv,
                right_value: rv,
                impact: if i == 3 { "Causal source: alternate reduction changes all downstream evidence." } else { "Propagated consequence of the earlier execution divergence." }.into(),
            });
        }
        left_hash = lh;
        right_hash = rh;
    }

    SemanticRealityDiff {
        left_event_id: left_event_id.into(),
        right_event_id: right_event_id.into(),
        common_semantic_prefix: common.min(3),
        first_divergence_stage: first.map(stage_name).map(str::to_string),
        first_divergence_index: first,
        changes,
        left_root_hash: left_hash,
        right_root_hash: right_hash,
        synthetic: true,
        claim_boundary: "Synthetic deterministic semantic diff; it compares modeled evidence fields and hash propagation, not live RSpace replay, node observation, or RChain finality.".into(),
    }
}
