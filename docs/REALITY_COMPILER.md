# RChain Reality Compiler

## Purpose

Trace an execution from its semantic origin to independently observable RChain evidence.

## Pipeline

```text
Event
  ↓
Rholang Process
  ↓
Execution Trace
  ↓
RChain Deploy / Block
  ↓
Sentinel Observation
  ↓
Evidence Graph
  ↓
Independent Verification
```

## Principles

1. Evidence before claims.
2. Never claim Sentinel proves Casper finality.
3. Never treat Sovereign-Lattice PBFT as RChain Casper.
4. Preserve provenance across every transformation.
5. Every verification result must identify its evidence basis.
6. Synthetic fixtures must be explicitly marked as synthetic.
7. Real integrations are added only when their actual APIs/interfaces have been verified.

## Planned Components

- EvidenceEnvelope
- Causality Explorer
- Provenance Graph
- Rholang Execution Adapter
- RChain Evidence Adapter
- Sovereign-Lattice Verification Adapter
- Replay / Divergence Analysis
- Counterfactual Analysis
