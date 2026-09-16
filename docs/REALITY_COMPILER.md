# RChain Reality Compiler

## Purpose

Trace an execution from its semantic origin to independently observable RChain evidence, preserving causal provenance through every transformation.

## Pipeline

```text
Origin Event
  ↓
QLF / ZFA Certificate
  ↓
Rholang Process
  ↓
ρ-Calculus Execution
  ↓
RChain Deploy / Block
  ↓
Sentinel Observation
  ↓
Evidence Graph
  ↓
Sovereign Lattice Analysis
  ↓
Verification Result
```

## Implemented surfaces

- Causality Explorer: `/reality`
- Provenance envelope: `/api/reality/event/{event_id}`
- Hash-linked Reality Diff: `/api/reality/diff/{left}/{right}`
- Semantic Reality Diff: `/api/reality/diff-semantic/{left}/{right}`
- Proof-Carrying Execution: `/api/reality/proof/{event_id}`
- Replay comparison: `/api/reality/replay/{event_id}`
- Counterfactual execution: `/api/reality/counterfactual/{event_id}/{scenario}`
- Adversarial evidence challenge: `/api/reality/challenge/{event_id}/{attack}`
- QLF certificate linkage: `/api/reality/qlf/{event_id}`
- Adapter registry: `/api/reality/adapters`
- Formal invariant trace: `/api/reality/invariants/{event_id}`
- Killer Demo: `/api/reality/demo/{event_id}`

## Principles

1. Evidence before claims.
2. Never claim Sentinel proves Casper finality.
3. Never treat Sovereign-Lattice PBFT as RChain Casper.
4. Preserve provenance across every transformation.
5. Every verification result identifies its evidence basis.
6. Synthetic fixtures are explicitly marked as synthetic.
7. Real integrations are added only when their actual APIs/interfaces have been verified.

## Evidence boundary

The current Reality Compiler surfaces are deterministic synthetic models. Their PASS states, replay matches, hash links, counterfactual results, and invariant traces describe the modeled evidence only. They do not claim live QuantumOS, RChain, Sentinel, or Sovereign Lattice production connectivity, nor do they constitute a machine-checked Lean proof or independent stake-weighted Casper finality proof.

## Architecture roles

- QuantumOS: origin of interaction.
- QLF: logical/certificate representation.
- Rholang / ρ-calculus: executable semantics.
- RChain / RNode: execution and chain state.
- Sentinel: observation and evidence preservation.
- Sovereign Lattice: independent verification analysis.
- Explorer: causal visualization and explanation.
