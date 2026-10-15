# Empty live deploy result: UPSTREAM/API LIMITATION

The authorized endpoint reports node revision `71c6a513d9785e45c3fbb919125c3314c2d8bf6e`. The retained transfer remains included at height 765 with `deployResult: []`. No new deploy, faucet call, key or infrastructure change was needed.

The exact chain is retained in [source-evidence.json](source-evidence.json), with Git blob identities and source links at the reported build revision:

1. `rholang/src/system_processes.rs:2257-2290`: the classic native REV transfer arm replies with `RhoNil::apply()` after successful `transfer_vault`, not a boolean/tuple. Rejection is a reducer error.
2. The retained signed term correctly binds the deploy-ID channel and forwards the actual result. Forwarding Nil does not create a boolean receipt.
3. `casper/src/api/block_api_impl.rs:233-249,452-461`: deploy-status reads that channel from the inclusion block's post-state.
4. `node/src/api/conversion.rs:113-125` uses `filter_map(expr_from_par)`; `node/src/api/rho_expr.rs:193-216` returns None for Nil/empty Par. The reply therefore becomes an empty result array.

[Read-only controls](read-only-result-probes.json) retain exact requests, response bytes, times and SHA-256. Direct Nil and forwarded Nil both return `expr: []` **with `replySource: firstPrivateName`**, while a boolean returns `ExprBool: true`. All responses identify finalized block 771. An empty expression array therefore does not establish that the return channel was never reached. Exploratory state was discarded. [Current observations](existing-deploy-observation.json) retain status and the existing deploy result. The running binary is not independently attested; the reported revision, exact source and API controls support this bounded classification.

Keep execution UNKNOWN and transfer assurance BLOCKED. Do not synthesize true from Nil, `ProcessedWithSuccess`, balance change or read-only simulation. Under the existing v1 gate, PASS needs a deploy-bound positive typed transfer receipt (supported boolean/tuple success), canonical inclusion/finalized read-back, sufficient independently pinned observers, and an independently supplied package pin. This classic native/API path does not expose that stronger receipt. No verifier gate or transfer term was weakened.
