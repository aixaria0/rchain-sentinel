# RChain Sentinel

<p align="center">
  <strong>Evidence-based verification for decentralized infrastructure.</strong>
</p>

<p align="center">
  Observe. Cross-check. Verify.
</p>

<p align="center">
  <a href="https://github.com/aixaria0/rchain-sentinel/actions">
    <img src="https://img.shields.io/github/actions/workflow/status/aixaria0/rchain-sentinel/ci.yml?label=CI&logo=github" alt="CI">
  </a>
  <img src="https://img.shields.io/badge/Rust-2021-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/Axum-0.7-black?logo=rust" alt="Axum">
  <img src="https://img.shields.io/badge/license-Apache--2.0-blue" alt="License">
</p>

---

## Overview

**RChain Sentinel** is an independent verification service for RChain-compatible infrastructure.

Sentinel is designed around a simple principle:

> **Don't just ask whether a node is alive. Ask whether the evidence supports what the node claims.**

Instead of reducing node state to a single health signal, Sentinel collects structured observations from RNode endpoints and evaluates them through a dedicated verification engine.

The result is an explicit, machine-readable verification report.

---

## Verification Pipeline

```text
                         RNode
                           │
              ┌────────────┴────────────┐
              │                         │
        /api/status        /api/last-finalized-block
              │                         │
              ▼                         ▼
       NetworkStatus          FinalizedBlockEvidence
              │                         │
              └────────────┬────────────┘
                           ▼
                  Verification Engine
                           │
                           ▼
                  Verification Report


---

What Sentinel Verifies

The current verification engine evaluates evidence including:

Node reachability

HTTP response status

Probe integrity

Node identity

Network identity

Shard identity

Node readiness

Validator state

Peer state

Current epoch

Finalized block state

Finalized block evidence availability

Observation integrity


Each verification check produces an explicit state:

PASS
WARN
FAIL


---

API

Health

GET /health

Returns the health and service metadata of Sentinel.

Network Status

GET /api/network/status

Queries the configured RNode and returns the observed network and node state.

Verification

GET /api/verify

Collects current RNode observations and runs the verification engine.

The verification endpoint combines:

/api/status
        +
/api/last-finalized-block
        ↓
Verification Engine
        ↓
Verification Report


---

Example Verification Report

{
  "target": "http://localhost:40403",
  "status": "Pass",
  "checks": [
    {
      "name": "node_reachable",
      "status": "Pass",
      "message": "Node responded to the verification probe."
    },
    {
      "name": "node_readiness",
      "status": "Pass",
      "message": "RNode reports itself as ready."
    }
  ]
}


---

Architecture

rchain-sentinel/
│
├── backend/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── models.rs
│       ├── rnode.rs
│       └── verification.rs
│
├── docs/
│   └── index.html
│
├── README.md
└── LICENSE

RNodeClient

Handles communication with the RNode HTTP API and collects raw network observations.

Models

Defines structured representations for:

Node status

Network observations

Finalized-block evidence

Verification checks

Verification reports


VerificationEngine

Evaluates collected evidence and converts observations into structured verification results.

HTTP Service

Exposes Sentinel through a lightweight Axum REST API.


---

Evidence-First Design

Traditional node monitoring often reduces infrastructure state to:

ONLINE
OFFLINE

Sentinel is designed to preserve the evidence behind the decision.

Instead of:

Node: ONLINE

the system can reason about:

Reachability
Network identity
Validator state
Readiness
Peer state
Finalized block
Evidence availability
Observation integrity

This makes verification results inspectable rather than opaque.


---

Configuration

Sentinel accepts the RNode endpoint through:

RCHAIN_RNODE_URL

Example:

RCHAIN_RNODE_URL=http://localhost:40403

If the variable is not configured, Sentinel defaults to:

http://localhost:40403

The Sentinel HTTP service listens on:

0.0.0.0:8080


---

Running Locally

From the backend directory:

cargo run

Then query the service:

GET http://localhost:8080/health
GET http://localhost:8080/api/network/status
GET http://localhost:8080/api/verify


---

Technology

Built with:

Rust

Tokio

Axum

Reqwest

Serde

GitHub Actions


The system is intentionally lightweight at the service layer so that deeper verification logic can evolve independently.


---

Roadmap

Phase 1 — Observation

[x] RNode connectivity

[x] Structured RNode status observation

[x] Node identity

[x] Network identity

[x] Shard identity

[x] Validator state

[x] Readiness state

[x] Peer observation

[x] Epoch observation


Phase 2 — Evidence Verification

[x] Finalized-block endpoint integration

[x] Finalized-block evidence model

[x] Evidence-aware verification engine

[ ] Cross-check finalized block height

[ ] Cross-check finalized block hash

[ ] Detect contradictory node evidence


Phase 3 — Multi-Node Verification

[ ] Multiple RNode targets

[ ] Cross-node observation

[ ] Finality agreement analysis

[ ] Divergence detection

[ ] Inconsistent-state detection


Phase 4 — Cryptographic Verification

[ ] Block integrity verification

[ ] Signature verification

[ ] Cryptographic evidence validation

[ ] Trust-minimized verification paths


Phase 5 — Sentinel Console

[ ] Live verification console

[ ] Evidence timeline

[ ] Verification history

[ ] Node comparison

[ ] Machine-readable verification exports



---

Design Direction

Sentinel is intended to evolve from node observation into a verification layer for decentralized infrastructure.

The architectural direction is:

Observe
   ↓
Collect Evidence
   ↓
Cross-Check
   ↓
Verify
   ↓
Explain

The long-term objective is not merely to display network state, but to determine whether independently observable evidence is consistent with the state being claimed.


---

Project Status

Early development

The current implementation provides:

RNode observation

Structured evidence collection

Evidence-aware verification

REST endpoints

Deterministic PASS / WARN / FAIL reporting


The next major verification milestone is cross-validating finalized-block claims against independently retrieved block evidence.


---

License

Apache License 2.0
