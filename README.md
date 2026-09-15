# RChain Sentinel

<p align="center">
  <strong>Evidence-based verification for decentralized infrastructure.</strong>
</p>

<p align="center">Observe. Cross-check. Verify. Explain.</p>

<p align="center">
  <a href="https://github.com/aixaria0/rchain-sentinel/actions"><img src="https://img.shields.io/github/actions/workflow/status/aixaria0/rchain-sentinel/ci.yml?label=CI&logo=github" alt="CI"></a>
  <img src="https://img.shields.io/badge/Rust-2021-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/Axum-0.7-black?logo=rust" alt="Axum">
  <img src="https://img.shields.io/badge/license-Apache--2.0-blue" alt="License">
</p>

## Overview

RChain Sentinel is an independent verification service for RChain-compatible infrastructure. It is designed around one principle:

> Don't just ask whether a node is alive. Ask whether independently observed evidence supports what the node claims.

Sentinel collects RNode state and finalized-block evidence, preserves the observed payload, cross-checks multiple nodes, and produces explicit machine-readable verification results.

The system is intentionally conservative: node-count agreement is never presented as stake-weighted Casper finality. Casper-shaped evidence is reported separately until the exact protocol schema and validator/stake semantics are pinned to the target RNode implementation.

## Verification pipeline

```text
RNode
  ├── /api/status
  └── /api/last-finalized-block
          │
          ▼
   Evidence collection
          │
     ┌────┴─────┐
     ▼          ▼
Node checks   Block checks
     │          │
     └────┬─────┘
          ▼
 Cross-node analysis
          │
          ▼
 Casper evidence inventory
          │
          ▼
 PASS / WARN / FAIL + evidence
```

## Current capabilities

Sentinel currently provides:

- RNode reachability, HTTP status, latency, readiness, validator/read-only state, peers, epoch, network and shard identity.
- Finalized-block evidence collection with an SHA-256 fingerprint of the exact JSON payload observed by Sentinel.
- Cross-checking of finalized height and block hash across configured RNodes.
- Concurrent cross-node observation using Tokio/Futures.
- Observed 2/3 node-count quorum calculation with conflict and missing-evidence reporting.
- Block evidence checks for block hash, parent hash, proposer, signature and justification-shaped fields.
- A separate Casper evidence inventory for validator, stake/weight, bet/belief, justification and possible equivocation-shaped fields.
- Deterministic verification output rather than a single opaque health signal.

Important: the payload SHA-256 is an integrity fingerprint of the observed JSON. It is not the RChain block hash and is not itself proof of finality. Likewise, recognized Casper-shaped fields are evidence inventory only; they do not establish Casper finality without protocol-level validation.

## API

`GET /health` — Sentinel service health.

`GET /api/network/status` — current RNode observation.

`GET /api/evidence/last-finalized-block` — raw finalized-block evidence plus extracted fields and payload fingerprint.

`GET /api/verify` — combined network and finalized-block verification report.

`GET /api/verify/block` — finalized-block evidence verification report.

`GET /api/verify/casper` — Casper-shaped evidence inventory. This endpoint explicitly avoids claiming stake-weighted finality.

`GET /api/verify/cross-node` — concurrent cross-node finalized-block agreement analysis.

## Configuration

Single-node mode:

```bash
RCHAIN_RNODE_URL=http://localhost:40403
```

Multi-node mode:

```bash
RCHAIN_RNODE_URL=http://localhost:40403
RCHAIN_RNODE_URLS=http://node-a:40403,http://node-b:40403,http://node-c:40403
```

`RCHAIN_RNODE_URLS` controls the cross-node verification targets. If it is not set, Sentinel falls back to the single `RCHAIN_RNODE_URL` target.

The Sentinel HTTP service listens on `0.0.0.0:8080`.

## Architecture

```text
backend/src/
├── main.rs
├── models.rs
├── rnode.rs
├── verification.rs
├── block_verification.rs
├── cross_node.rs
└── casper_evidence.rs
```

`RNodeClient` communicates with RNode and preserves raw observations.

`VerificationEngine` evaluates node and network consistency.

`BlockVerificationEngine` evaluates finalized-block evidence without conflating payload integrity with protocol finality.

`CrossNodeVerificationEngine` concurrently compares observations and reports agreement, divergence, missing evidence and an observed node-count quorum.

`CasperEvidenceEngine` inventories protocol-shaped evidence while deliberately stopping short of an unsupported finality claim.

## Running locally

From `backend/`:

```bash
cargo run
```

For tests:

```bash
cargo test
```

## Design direction

```text
Observe
   ↓
Collect Evidence
   ↓
Cross-Check
   ↓
Analyze Protocol Evidence
   ↓
Verify
   ↓
Explain
```

The long-term objective is a verification layer that can answer not only what state an RNode reports, but why that state should be trusted, using independently observable protocol evidence.

A future integration target is a block-explorer workflow where a block can be inspected together with the evidence supporting its claimed finality.

## Roadmap

### Completed

- RNode observation
- Structured evidence collection
- Finalized-block integration
- Block evidence verification
- Cross-node verification
- Concurrent cross-node analysis
- Observed quorum calculation
- Conflict detection
- Casper evidence inventory
- Deterministic verification reporting
- CI-backed Rust tests

### Next

- Pin the exact RChain/RNode Casper response schema.
- Parse validator identities and stake weights from authoritative protocol data.
- Parse propositions/bets and justification graphs from authoritative protocol data.
- Detect equivocation using protocol-defined evidence rather than heuristic field names.
- Compute stake-weighted agreement only from an authenticated validator/stake set.
- Add historical evidence and block-by-block verification.
- Build the live Sentinel console / explorer integration.

## Project status

Early development, with the verification architecture actively evolving toward protocol-aware evidence rather than simple node monitoring.

## License

Apache License 2.0
