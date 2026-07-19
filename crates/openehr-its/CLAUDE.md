# `openehr-its` — canonical JSON/XML + the ITS-REST contract (MIXED)

Three surfaces in one crate, with a strict generated/hand-written split.
Know which half you are touching before editing anything:

| Part | Status | To change it |
|---|---|---|
| `src/xml/generated/` (`ToXml`/`FromXml`) | **GENERATED** (`emit-xml`, from the XSDs + BMM) | edit the emitter, regenerate |
| `src/json_codec/generated/` (`ToJson`/`FromJson`) | **GENERATED** (`emit-json`, from BMM) | edit the emitter, regenerate |
| `src/rest/generated/` (ITS-REST DTOs, server traits, routes) | **GENERATED** (`emit-rest`, from the vendored OAS) | edit the emitter, regenerate |
| `xml/runtime.rs`, `json_codec/runtime.rs`, `rest/runtime.rs`, `json` + `rm_validate` entry points, validation, fidelity gates | hand-written | edit normally, with spec citations |

The native canonical-JSON codec (`json_codec`) is THE canonical-JSON
(de)serialization for every spec type (all five crates) — the emitted
`ToJson`/`FromJson` impls over a hand-written writer/reader runtime. The spec
types carry NO serde derive (the `#[derive(OpenEhrType)]` proc-macro is deleted);
`json::to_canonical_json`/`from_canonical_json`/`from_canonical_value` ARE the
codec entry points, and `rm_validate::validate_rm_value` is the wire-boundary
RM class-invariant dispatcher that drives the reader. Proven by
`tests/json_codec_parity.rs` (byte hazards + `FromJson` tolerance) +
`tests/canonical_contract.rs` (the R0 determinism manifest).

- **NEVER hand-edit anything under a `generated/` directory** — the
  `codegen-drift` CI job regenerates and fails on any diff
  (`/regen-codegen` runs emit + emit-xml + emit-rest + the check).
- The vendored inputs are authoritative: XSDs at `schemas/xml/`, ITS-JSON
  schema at `schemas/json/` (validation oracle), REST OAS at
  `vendor/rest-oas/` — pinned to the same commit as the spec text under
  `docs/specs/openehr/ITS-REST/` (a reconciliation guard enforces this).
  Never edit vendored files; re-vendor on a pin bump.
- Canonical-JSON `_type` self-tagging comes from the emitted `ToJson`/`FromJson`
  codec (`emit-json`) — no per-struct tag fields, no serde derive on spec types.
- XML: one impl set serves both namespaces (v1/v2 differ only by root
  `xmlns`); `xsi:type` emitted iff concrete type ≠ declared slot type.
- **The fidelity gates in `tests/` are the crate's acceptance instrument**
  (corpus round-trips, C14N, schema validation) — never weaken or skip one
  to get green; a gate failure means the emitter or runtime is wrong.
