# Changelog

## v1 assurance baseline — 2026-09-28

### Added
- portable repair assurance observation contract;
- `POST /api/assurance/repair/observe`;
- deterministic Sentinel observation digest;
- fail-closed checks for schema, provenance, digest syntax, and native replay state.

### Boundary
Sentinel observes and validates evidence transport. Acceptance is not an independent proof of Casper finality, repair correctness, or production safety.
