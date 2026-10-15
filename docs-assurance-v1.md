# REV transfer observations

`GET /api/assurance/action/{deploy_id}` is read-only. It uses configured `RCHAIN_RNODE_URLS`; it never accepts target URLs from requests. `RCHAIN_ASSURANCE_FAILURE_DOMAINS` supplies comma-separated operator-defined failure domains in target order. Set `RCHAIN_SENTINEL_REVISION` to the exact deployed source commit. Unpinned/unversioned producers cannot satisfy the verifier's external revision pin.

Canonical contract: aixaria0/RCHAIN-COMPLIER, `contracts/rchain-assurance/v1.schema.json`. The fixture in backend/tests is a synthetic contract example. This repository does not own the schema or trust policy.

Exact endpoint response bodies and read-only explore request terms are retained with SHA-256. The producer observes deploy status, the exact signature's inclusion, actual contract result, canonical block fields, explicit node finality, and recipient balance at the observed finalized state. It does not infer external signing/submission times. Missing data remains unknown/unavailable. Parent traversal is bounded to 32 blocks per observer. Missing parent representations remain HEIGHT_SKEW. A tip moving during the read can conservatively defer read-back evidence until refresh.

Node observations and parent references are evidence under externally configured observer trust, not a stake-weighted Casper proof. The verifier must validate all raw digests, exact task/claim/network binding and external producer/observer/freshness pins. Observation and package digests do not authenticate a producer by themselves.

RC hardening classifies the retained live empty result as UPSTREAM/API LIMITATION: the reported build's classic native transfer replies Nil and JSON expression conversion drops Nil. Exact source and read-only Nil/boolean controls are in [evidence/rc-hardening](evidence/rc-hardening/README.md). Empty/Nil remains UNKNOWN, never positive transfer-execution proof. Received package digests are integrity bookkeeping; only independently supplied operator/reviewer pins can bind a whole package. Producer data cannot supply those pins.
