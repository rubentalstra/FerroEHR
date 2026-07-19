# `openehr-its` — canonical JSON/XML + the ITS-REST contract (MIXED)

Three surfaces in one crate, with a strict generated/hand-written split.
Know which half you are touching before editing anything:

| Part | Status | To change it |
|---|---|---|
| `src/xml/generated/` (`ToXml`/`FromXml`) | **GENERATED** (`emit-xml`, from the XSDs + BMM) | edit the emitter, regenerate |
| `src/json_codec/generated/` (`ToJson`) | **GENERATED** (`emit-json`, from BMM) | edit the emitter, regenerate |
| `src/rest/generated/` (ITS-REST DTOs, server traits, routes) | **GENERATED** (`emit-rest`, from the vendored OAS) | edit the emitter, regenerate |
| `xml/runtime.rs`, `json_codec/runtime.rs`, `rest/runtime.rs`, canonical-JSON entry points, validation, fidelity gates | hand-written | edit normally, with spec citations |

The native canonical-JSON codec (`json_codec`) is the emitted Serialize-side
replacement for `#[derive(OpenEhrType)]`: a hand-written writer runtime under
emitted `ToJson` impls for EVERY spec type (all five crates), byte-identical to
the serde path — proven by `tests/json_codec_parity.rs`. It is not yet the
`json::to_canonical_json` entry point (a later rewrite phase switches it and
retires the derive).

- **NEVER hand-edit anything under a `generated/` directory** — the
  `codegen-drift` CI job regenerates and fails on any diff
  (`/regen-codegen` runs emit + emit-xml + emit-rest + the check).
- The vendored inputs are authoritative: XSDs at `schemas/xml/`, ITS-JSON
  schema at `schemas/json/` (validation oracle), REST OAS at
  `vendor/rest-oas/` — pinned to the same commit as the spec text under
  `docs/specs/openehr/ITS-REST/` (a reconciliation guard enforces this).
  Never edit vendored files; re-vendor on a pin bump.
- Canonical-JSON `_type` self-tagging comes from `#[derive(OpenEhrType)]`
  (`openehr-derive`) — no per-struct tag fields, no manual serializers.
- XML: one impl set serves both namespaces (v1/v2 differ only by root
  `xmlns`); `xsi:type` emitted iff concrete type ≠ declared slot type.
- **The fidelity gates in `tests/` are the crate's acceptance instrument**
  (corpus round-trips, C14N, schema validation) — never weaken or skip one
  to get green; a gate failure means the emitter or runtime is wrong.
