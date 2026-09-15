# RChain Sentinel

<p align="center">
  <img src="https://img.shields.io/badge/RUST-2021-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/RCHAIN-SENTINEL-111827?style=for-the-badge" alt="RChain Sentinel">
  <img src="https://img.shields.io/badge/LICENSE-APACHE--2.0-1f2937?style=for-the-badge" alt="Apache 2.0">
</p>

<p align="center">
  <strong>Evidence-first verification for decentralized infrastructure.</strong>
</p>

<p align="center">
  <em>Observe state. Cross-check evidence. Explain why a block is trusted.</em>
</p>

<p align="center">
  <a href="https://aixaria0.github.io/rchain-sentinel/"><strong>🚀 OPEN SENTINEL SHOWCASE</strong></a>
  &nbsp;&nbsp;·&nbsp;&nbsp;
  <a href="https://github.com/aixaria0/rchain-sentinel/actions">CI / Actions</a>
</p>

> **SHOWCASE MODE:** The public GitHub Pages UI is an explicitly labeled offline demonstration. Synthetic data is never presented as live chain evidence.

---

## What is Sentinel?

**RChain Sentinel** is an independent verification and evidence layer for RChain-compatible infrastructure.

It is designed around a simple question:

> **Don't only ask whether a node is alive. Ask whether the evidence supports what the node claims.**

Sentinel observes RNode state and finalized-block data, preserves the exact observed payload, cross-checks configured nodes, inventories protocol-shaped Casper evidence, and turns the results into deterministic **PASS / WARN / FAIL** verification output.

The core presentation is block-centric: **Why this block?**

Instead of exposing one opaque health number, Sentinel builds an evidence trail from:

```text
RNode state
    │
    ├── identity / network / shard
    ├── readiness / peers / epoch
    └── finalized-block observation
              │
              ▼
      Evidence fingerprint
              │
      ┌───────┴────────┐
      ▼                ▼
Canonical block    Cross-node view
consistency        agreement / conflict
      │                │
      └───────┬────────┘
              ▼
      Casper evidence
      inventory
              │
              ▼
      Verification matrix
              │
              ▼
        WHY THIS BLOCK?
```

---

## 🚀 See it first

### Public Showcase

**[Open the Sentinel Dashboard →](https://aixaria0.github.io/rchain-sentinel/)**

The dashboard presents the verification surface without requiring an RNode connection. It is intentionally marked **DEMO DATA / SHOWCASE** when operating on synthetic data.

### What the dashboard exposes

| Surface | Purpose |
| --- | --- |
| **Network Overview** | Node identity, readiness, network/shard state and finalized height |
| **Finality Evidence** | RNode finality assertion, canonical consistency and hash matching |
| **Block Verification** | Deterministic evidence checks with PASS / WARN / FAIL outcomes |
| **Casper Evidence** | Validator, bond, stake and justification-shaped protocol evidence |
| **Why this block?** | Human-readable explanation assembled from the evidence trail |
| **Cross-node quorum** | Observed node-count agreement, explicitly separated from stake-weighted finality |

---

## 🔬 Verification model

Sentinel deliberately separates observation from proof.

| Layer | Sentinel checks | What it means |
| --- | --- | --- |
| **Node** | Reachability, HTTP status, latency, readiness, identity | The node can be observed and described |
| **Block** | Hash, parent, proposer, signature, justifications | Expected block evidence is present |
| **Canonical** | Observed block vs canonical `/api/block/{hash}` | The node's returned representations agree |
| **Cross-node** | Height/hash agreement across configured RNodes | Independent node observations converge |
| **Casper** | Bonds, stake, validators, justifications, protocol-shaped fields | Protocol evidence inventory is available |
| **Integrity** | SHA-256 fingerprint of observed JSON | Exact observed payload can be fingerprinted |
| **Finality** | RNode `/is-finalized/{hash}` assertion | Node-reported finality is recorded, not independently proven |

### The important boundary

**Sentinel does not currently claim independent stake-weighted Casper finality.**

An observed 2/3 node-count agreement is not equivalent to a 2/3 stake-weighted Casper proof. Recognized protocol fields are evidence inventory until validator identity, stake weights, propositions/bets, justifications and signatures can be authenticated against the exact target RNode protocol.

That distinction is intentional. The system is built to make unsupported claims harder, not easier.

---

## ⚙️ Current capabilities

- Browser-based black-chain verification dashboard.
- Explicit offline showcase mode for public deployments.
- Unified `/api/explorer/block` evidence package.
- Human-readable **Why this block?** explanation.
- RNode network, identity, readiness, validator/read-only, peer, epoch and shard observations.
- Finalized-block collection with SHA-256 payload fingerprinting.
- Canonical block retrieval and protocol-field consistency comparison.
- Concurrent cross-node observation.
- Observed 2/3 node-count quorum calculation with conflict reporting.
- Block evidence checks for hash, parent, proposer, signature and justification-shaped fields.
- Protocol-aware Casper evidence inventory.
- Bond/stake structure analysis and duplicate/invalid bond detection.
- Equivocation-shaped signal detection without treating heuristics as authenticated proof.
- Deterministic machine-readable verification results.
- Rust/Axum service with deployment-ready container configuration.

---

## 🧭 API surface

| Endpoint | Purpose |
| --- | --- |
| `GET /` | Sentinel verification dashboard |
| `GET /health` | Service health |
| `GET /api/network/status` | Current RNode observation |
| `GET /api/evidence/last-finalized-block` | Raw finalized-block evidence + fingerprint |
| `GET /api/verify` | Combined network + block verification |
| `GET /api/verify/block` | Finalized-block verification |
| `GET /api/verify/casper` | Casper evidence inventory |
| `GET /api/verify/cross-node` | Cross-node agreement analysis |
| `GET /api/explorer/block` | Unified block-centric evidence package |
| `GET /api/block/{hash}` | Direct block proxy |
| `GET /api/is-finalized/{hash}` | Direct finality assertion proxy |

---

## 🏗️ Architecture

```text
                    ┌─────────────────────┐
                    │       RNode(s)      │
                    └──────────┬──────────┘
                               │
                               ▼
                    ┌─────────────────────┐
                    │   Evidence Layer    │
                    │ raw observations    │
                    │ payload fingerprints│
                    └──────────┬──────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                ▼
        Node/network      Block checks     Cross-node
         verification      + canonical      analysis
              │                │                │
              └────────────────┼────────────────┘
                               ▼
                    ┌─────────────────────┐
                    │ Casper Evidence     │
                    │ inventory           │
                    └──────────┬──────────┘
                               ▼
                    ┌─────────────────────┐
                    │ Verification Engine │
                    │ PASS / WARN / FAIL │
                    └──────────┬──────────┘
                               ▼
                    ┌─────────────────────┐
                    │ Explorer / Dashboard│
                    │    Why this block?  │
                    └─────────────────────┘
```

### Repository layout

```text
backend/
├── src/
│   ├── main.rs
│   ├── models.rs
│   ├── rnode.rs
│   ├── verification.rs
│   ├── block_verification.rs
│   ├── cross_node.rs
│   └── casper_evidence.rs
└── console.html

Dockerfile
render.yaml
```

---

## 🧪 Run locally

From `backend/`:

```bash
cargo run
```

Open:

```text
http://localhost:8080/
```

Run tests:

```bash
cargo test
```

### Connect one RNode

```bash
RCHAIN_RNODE_URL=http://localhost:40403 cargo run
```

### Connect multiple RNodes

```bash
RCHAIN_RNODE_URLS=http://node-a:40403,http://node-b:40403,http://node-c:40403 cargo run
```

`RCHAIN_RNODE_URLS` takes precedence for cross-node analysis. If it is unset, Sentinel falls back to `RCHAIN_RNODE_URL`.

The service uses `PORT` when supplied by a deployment platform and otherwise listens on `8080`.

---

## 🌐 Deployment

The repository includes:

- `Dockerfile` for container deployment.
- `render.yaml` for deployment on a container-capable platform such as Render.
- `.github/workflows/pages.yml` for the static GitHub Pages showcase.

GitHub Pages serves the visual dashboard only. It does **not** run the Rust backend, so the public Pages deployment intentionally falls back to clearly labeled synthetic showcase data.

For live evidence, deploy the Rust service and configure `RCHAIN_RNODE_URL` or `RCHAIN_RNODE_URLS`.

---

## 🔭 Explorer integration

Sentinel is designed as a verification layer behind an RChain black-chain explorer.

The current implementation deliberately does not invent a third-party explorer API contract. Instead, `/api/explorer/block` exposes a stable block-centric evidence package that an explorer can consume.

A future block-detail surface can render:

```text
BLOCK #HEIGHT
│
├── Identity
├── Parent
├── Proposer / Validator
├── Bonds / Observed Stake
├── Justifications
├── Canonical Consistency
├── Cross-node Agreement
├── Verification Matrix
└── WHY THIS BLOCK?
```

The explorer should always preserve four distinct categories:

1. **RNode assertion** — what the node reports.
2. **Sentinel observation** — what Sentinel independently observes and cross-checks.
3. **Authenticated protocol evidence** — what can be cryptographically validated.
4. **Finality claim** — what the evidence actually justifies saying.

---

## 🗺️ Roadmap

### Completed

- RNode observation
- Structured evidence collection
- Finalized-block integration
- Block evidence verification
- Canonical block consistency checks
- Cross-node verification
- Concurrent cross-node analysis
- Observed quorum calculation
- Conflict detection
- Protocol-aware Casper evidence inventory
- Deterministic verification reporting
- Browser verification dashboard
- Unified explorer evidence endpoint
- Human-readable block explanation
- Offline showcase mode
- Container deployment configuration
- GitHub Pages showcase deployment
- CI-backed Rust tests

### Next

- Pin the exact Casper response schema from the target RNode runtime.
- Parse authenticated validator identities and stake weights.
- Parse propositions/bets and justification graphs from authoritative protocol data.
- Replace heuristic equivocation signals with protocol-defined evidence.
- Compute stake-weighted agreement from an authenticated validator/stake set.
- Add historical block-by-block verification.
- Integrate the unified evidence package with the concrete explorer block-detail interface.

---

## Project status

**Early development — verification architecture actively evolving toward protocol-aware evidence and explorer-grade verification.**

The project favors explicit evidence boundaries over inflated finality claims.

---

## Author

**AixAria** — Architect

Built for evidence-driven verification of decentralized infrastructure.

## License

Apache License 2.0
