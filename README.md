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
- Optional pinned-key Ed25519 attestation snapshots that bind network status, finalized-block evidence, and cross-node observations into one signed payload.
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
| `GET /api/attestation/snapshot` | Signed `rchain-sentinel-attestation/v1` snapshot; returns `503` when signing is not configured |
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

RCHAIN_RNODE_URLS takes precedence for cross-node analysis. If it is unset, Sentinel falls back to RCHAIN_RNODE_URL.

### Enable signed assurance snapshots

Promotion-grade live evidence can be exposed through `GET /api/attestation/snapshot`. The endpoint signs the canonical JSON payload with Ed25519 and includes:

- network status;
- finalized-block evidence;
- cross-node agreement report;
- payload SHA-256;
- public-key fingerprint (`key_id`);
- detached Ed25519 signature.

For deployments, prefer a mounted secret file containing exactly 32 private-key bytes encoded as 64 hexadecimal characters:

```bash
export RCHAIN_SENTINEL_ED25519_PRIVATE_KEY_FILE=/run/secrets/sentinel-ed25519.hex
cargo run
```

For local staging only, the same 64-hex secret can be supplied with `RCHAIN_SENTINEL_ED25519_PRIVATE_KEY_HEX`.

The service never generates or persists a private key. If neither variable is present, the ordinary observation APIs remain available and the signed endpoint returns `503 Service Unavailable`. Invalid key material fails startup rather than silently serving unsigned data.

When signing is enabled, Sentinel also requires an explicit failure-domain declaration for every configured RNode target. Prefer a mounted JSON file:

```bash
export RCHAIN_SENTINEL_FAILURE_DOMAINS_FILE=/run/secrets/rnode-failure-domains.json
```

Example:

```json
[
  {
    "node_url": "http://node-a:40403",
    "operator_id": "operator-a",
    "provider_id": "provider-a",
    "region": "region-a",
    "failure_domain_id": "domain-a"
  },
  {
    "node_url": "http://node-b:40403",
    "operator_id": "operator-b",
    "provider_id": "provider-b",
    "region": "region-b",
    "failure_domain_id": "domain-b"
  }
]
```

`RCHAIN_SENTINEL_FAILURE_DOMAINS_JSON` is also accepted for staging. The declared target set must exactly match `RCHAIN_RNODE_URLS`; missing, extra, duplicate, or partially empty declarations fail startup when signing is enabled. These declarations are included inside the Ed25519-signed snapshot, making the claimed operational topology tamper-evident. They remain operator declarations, not external proof that the operators/providers/regions are truly independent.

Signing also requires the expected genesis block hash:

```bash
export RCHAIN_SENTINEL_GENESIS_HASH=<expected-genesis-block-hash>
```

Sentinel does not merely copy this value into the attestation. For every signed snapshot it queries the configured RNode with `/api/block/{hash}`, records the returned block, extracts its canonical block hash and block number, and checks both **hash equality** and **blockNumber = 0**. The signed snapshot therefore distinguishes a configured genesis trust anchor from an RNode-observed genesis witness. If the block cannot be retrieved or the identity/height does not match, the observation remains signed but cannot satisfy the strict Reality Plane promotion gate.

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
