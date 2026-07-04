# Phase 04 — Canonical JSON serialization (ITS-JSON)

- Status: **done**
- Build order: complete (spec foundation)
- Decisions: ADR-004 (`OpenEhrType` derive), ADR-005 (validation gate)

## Outcome

Canonical-JSON `_type` self-tagging is carried by `#[derive(OpenEhrType)]`
(`openehr-derive`) on every generated concrete class (first-field `_type`,
`None`/empty omitted, tolerant deserialize, untagged enums for closed slots).
`openehr-its::json` provides the named entry points (`to_canonical_json`,
`from_canonical_json`) and `validate_canonical` (validates against the vendored
ITS-JSON schema, whose root dispatches on `_type`).

## Verification

`openehr-its/tests/fidelity.rs`: the openEHR_SDK corpus reads (readability
gate), round-trips losslessly (`semantic_eq`), and **validates against the
ITS-JSON schema** (53 conformant, 0 failed) — all green.
