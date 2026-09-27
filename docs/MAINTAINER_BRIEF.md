# Maintainer Brief

`/maintainer` is a one-screen, read-only operational brief for recurring RNode maintainer checks.

It exists to shorten the repeated path from "the node says something succeeded" to "what did the chain actually expose?" without giving the collector mutation authority.

## Endpoints

- `GET /maintainer` - human-readable one-screen brief.
- `GET /api/maintainer/brief` - full read-only evidence bundle.
- `GET /api/maintainer/packet` - the `MAINTAINER_HEALTH` evidence packet consumed by the Reality Plane Maintainer Loop in `RCHAIN-COMPLIER`.

## Read-only probes

The collector only issues GET requests to the configured RNode:

- `/api/status`
- `/version`
- `/api/capabilities`
- `/api/v1/shards`
- `/api/last-finalized-block`
- `/api/block/{hash}`
- `/api/is-finalized/{hash}`

Cross-node verification repeats the finalized-block read against `RCHAIN_RNODE_URLS`.

No deploy, propose, trust, bond, withdraw, slash, faucet, transaction, or admin endpoint is called.

## Verdict order

The brief reports the first actionable observation in deterministic order:

1. API status unreadable;
2. finalized-block evidence unavailable;
3. canonical block mismatch;
4. node rejects finality;
5. malformed/inconsistent bond evidence;
6. cross-node conflict or insufficient agreement;
7. missing version/capabilities/shard/canonical/finality evidence;
8. PASS.

`PASS` is deliberately bounded. It means the configured read surfaces agree on the observations collected here. It does not mean protocol-wide Casper correctness or a universal safety proof.

## Known visibility boundary

The current public RNode HTTP surface does not expose every native PoS map directly. In particular, trusted-set and pending-withdrawal state cannot be claimed by Sentinel unless RNode exposes an explicit read surface for them. The brief marks unavailable evidence rather than inventing it.

## Reality Plane handoff

`GET /api/maintainer/packet` emits the exact packet shape expected by the `MAINTAINER_HEALTH` scenario in RCHAIN-COMPLIER:

```text
rchain-sentinel GET-only collector
  -> maintainer evidence packet
  -> deterministic Maintainer Loop checks
  -> first divergence
  -> sealed Reality Record
  -> rlsenti / maintainer review
```

This keeps the collector observational and leaves protocol decisions with maintainers.
