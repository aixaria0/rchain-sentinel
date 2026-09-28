# Repair Assurance API

## Endpoint

`POST /api/assurance/repair/observe`

Consumes `causal-assurance-repair-propagation/v1` and returns `sentinel-repair-observation/v1`.

Required transport fields include repair problem identity, repair artifact digest, native receipt digest, native binding digest, pinned upstream repository/commit, selected repair action, native replay status, and claim boundary.

The endpoint fails closed on unsupported schema, malformed canonical SHA-256 digests, missing provenance fields, or absent native replay verification.

The resulting observation digest identifies the received envelope bytes. It does not establish the semantic truth of the upstream repair claim.
