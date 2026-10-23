# Sentinel Witness Gate

An offline, deterministic bridge from Rholo's read-only incident snapshots to a reviewable incident report. It consumes JSONL produced by `tools/incident-snapshot.py` in [Rholo PR #421](https://github.com/rhovision-org/rholo/pull/421), not raw untrusted network requests.

This component complements **Sentinel** (network observations), **CHIMERA** (deploy execution classification), **RLSenti / Reality Compiler** (synthetic counterfactuals) and **RCHAIN-COMPLIER** (independent assurance receipts). It does not collapse their different trust boundaries into one claim.

## Run

Python 3.10+, standard library only. No network access or private keys.

```sh
python3 -m unittest discover -s tools/tests -v

python3 tools/sentinel_witness_gate.py \
  examples/sentinel-incident.synthetic.jsonl \
  --topology examples/sentinel-topology.synthetic.json \
  --output /tmp/sentinel-witness.json
```

Both files under `examples/` are **synthetic**. Do not use them as evidence of actual Rholo behavior.

For a real incident, preserve the unchanged JSONL and obtain its SHA-256 from an independent review path:

```sh
sha256sum incident-observations.jsonl
python3 tools/sentinel_witness_gate.py incident-observations.jsonl \
  --expect-sha256 REVIEWED_64_CHARACTER_LOWERCASE_DIGEST \
  --topology operator-reviewed-topology.json \
  --output incident-witness.json
```

To re-check a saved report, supply the same capture, optional topology and
external digest pin, substituting --verify-report incident-witness.json for
--output incident-witness.json. The verifier recomputes the entire report
from raw observations and rejects forged results even if their report
digest was recalculated. It shares the interpretation code with the author,
so this is not an independent consensus implementation.

No output is written to the target path if it already exists. The output contains no raw API bodies or URL fields; it stores selected observations and digests. Caller-supplied pins attest *byte identity only*, not source authorship or host authenticity. Repeating the same input yields byte-identical output.

## Verdict semantics

| Finding | Exact meaning |
| --- | --- |
| `INSUFFICIENT_FINALITY_OBSERVATION` | An expected endpoint was not observable from the client. No conclusion that its node process is down. |
| `REPORTED_CONFLICT_REVIEW_REQUIRED` | Two sources reported different finalized hashes at the **same height and same observation time**, or a source's reported finalized height decreased. Not proof of a Casper safety violation. |
| `OBSERVATIONS_ONLY_NO_FINALITY_PROOF` | Sampled finality endpoints answered. No authenticated finality proof has been verified. |
| `REPORTED_PROGRESS` | A source's reported finalized height increased across successive timestamped samples. |
| `NO_PROGRESS_OBSERVED` | A source's reported height did not increase within the sample window. Not a verified finality stall. |
| `HEALTH_SNAPSHOT_STALE_OR_FUTURE` | Health data was more than 120 seconds old, or over 30 seconds ahead of the probe time. |
| `ILLUSTRATIVE_ONLY` | Caller-supplied host/stake relationships are used for correlated host-failure analysis, not confirmed against a live validator bond set. |

A different block **tip** is not a finalized-state fork witness. Different source labels, HTTP endpoints, or processes do not prove physically independent or authenticated validators. The host counterfactual uses the illustrative arithmetic `3 × remaining stake > 2 × configured stake`; it is not a full Casper CBC finality decision procedure.

No observed HTTP response can establish whether a lost-looking `Root` registration was durably finalized. That question requires exact deploy IDs, inclusion block hash, finality status, and independent reads at the **same finalized state**. CHIMERA's `ProcessedWithSuccess` classification is only execution evidence.

## Trust and threat model

The parser fails closed on duplicate keys, invalid timestamps, non-finite JSON numbers, unsupported response shapes, non-read-only captures, oversized files (5 MiB), and untrusted or invalid host topology structure. It refuses mismatched externally reviewed SHA-256 pins.

The report has a deterministic, canonical `report_sha256` commitment. This checks internal byte consistency; it is **not** a signature, external witness identity, source-to-binary verification, or finality certificate. Its `protocol_finality_proven` and `finalized_write_integrity_proven` fields remain false.

The integration is strictly offline: no deploys, validator management, peer joins, genesis changes or public-network load tests. Do not include raw operational logs, addresses, authorization secrets, or user data in committed fixtures. The goal is a bounded reproducible incident triage layer, not an authority to modify the target network.

## Recommended #420 evidence gate

Preserve the raw observations and their provenance; ask Jim for host/cgroup/kernel logs, deployed binary revisions and validator placement; ask Patrick for specific deploy IDs and finalized-state witnesses. Run an isolated replay at constrained and sufficient memory on a pinned binary before proposing a remediation. Until those facts are available, treat operational conclusions as provisional and keep the incident open.
