# `openehr-its` — canonical JSON/XML + the ITS-REST contract + Simplified Formats (MIXED)

Four ITS surfaces in one crate, with a strict generated/hand-written split.
Know which half you are touching before editing anything:

| Part | Status | To change it |
|---|---|---|
| `src/xml/generated/` (`ToXml`/`FromXml`) | **GENERATED** (`emit-xml`, from the XSDs + BMM) | edit the emitter, regenerate |
| `src/json_codec/generated/` (`ToJson`/`FromJson` in `impls.rs`; the `_type` → decode `structural_check` dispatch in `structural.rs`) | **GENERATED** (`emit-json`, from BMM) | edit the emitter, regenerate |
| `src/rest/generated/` (ITS-REST DTOs, server traits, routes) | **GENERATED** (`emit-rest`, from the vendored OAS) | edit the emitter, regenerate |
| `xml/runtime.rs`, `json_codec/runtime.rs`, `rest/runtime.rs`, `json` + `rm_validate` entry points, validation, fidelity gates | hand-written | edit normally, with spec citations |
| `src/flat/` — Simplified Formats (FLAT / STRUCTURED / Web Template / TDD) | hand-written (BMM has no simplified-format model) | edit normally, with spec citations |
| `src/rest/smart_scopes.rs` — the SMART on openEHR scope grammar (master08 resource scopes + master07/09 launch contexts) | hand-written (an ITS-REST sub-spec with no machine-readable model) | edit normally, with spec citations — the ONE grammar the CDR's scope gate AND scope-previewing REST clients (the admin console) parse with |

**Feature `full` (default = everything).** Every dependency and every surface in
the table above rides `full`, which is on by default — consumers are unaffected.
`default-features = false` compiles `rest::smart_scopes` ALONE, with zero
dependencies, so a REST client that must parse SMART scope strings on
`wasm32-unknown-unknown` (the admin console's scope previewer) uses the same
grammar the CDR's gate enforces instead of a second parser. Keep that island
dependency-free: nothing under `rest::smart_scopes` may reach for serde, axum, a
spec crate, or any other dep.

The native canonical-JSON codec (`json_codec`) is THE canonical-JSON
(de)serialization for every spec type (all five crates) — the emitted
`ToJson`/`FromJson` impls over a hand-written writer/reader runtime. The spec
types carry NO serde derive (the `#[derive(OpenEhrType)]` proc-macro is deleted);
`json::to_canonical_json`/`from_canonical_json`/`from_canonical_value` ARE the
codec entry points, and `rm_validate::validate_rm_value` is the wire-boundary
RM class-invariant dispatcher that drives the reader — its hand-written table
holds the invariant-bearing classes, and everything else falls through to the
generated `structural_check` dispatch, so the codec is the structural-conformance
authority for EVERY emitted class (a defective node of a class with no invariant
is refused too). Proven by
`tests/it/json_codec_parity.rs` (byte hazards + `FromJson` tolerance) +
`tests/it/canonical_contract.rs` (the R0 determinism manifest).

- **NEVER hand-edit anything under a `generated/` directory** — the
  `codegen-drift` CI job regenerates and fails on any diff
  (`/regen-codegen` runs emit + emit-xml + emit-rest + the check). The three
  XSD-driven ARCHETYPE modules `opt14/`, `aom2/` and `aom2_model/` are generated
  wholesale (every file carries the `@generated` banner) and are drift-guarded
  the same way, even though they are not under a `generated/` dir.
- **Three XSD-driven archetype codecs, one pipeline** (`render/emit_opt.rs`,
  one curated closure each in `load/xsd.rs`):

  | module | subcommand | closure | root |
  |---|---|---|---|
  | `opt14` | `emit-opt` | `AM_FILES_V1` (`Template.xsd`) | `<template>` = `OPERATIONAL_TEMPLATE` |
  | `aom2` | `emit-aom2` | `AOM2_FILES` (`P_Archetype.xsd`) | `<archetype>` = `P_AUTHORED_ARCHETYPE` |
  | `aom2_model` | `emit-aom2` | `AOM2_MODEL_FILES` (`Archetype.xsd`) | `<archetype>` = `AUTHORED_ARCHETYPE` |

  Everything module-specific is an `emit_opt::ModelTarget` parameter (module
  path, `@generated` banner, doc labels), never a constant: a hardcoded value
  silently stamps one module's identity onto another's files.
  The two AOM2 serializations stay SEPARATE closures — both schemas declare the
  top-level element `archetype` with different root types and define same-named
  supporting types, so merging them resolves the abstract slots inconsistently.
  `aom2_model`'s entry points are typed to `AUTHORED_ARCHETYPE`, not to the
  `ARCHETYPE` its global element names: `ARCHETYPE` is `abstract="true"` with no
  derived type in the closure (`AUTHORED_ARCHETYPE` extends `AUTHORED_RESOURCE`
  and re-uses the body via `<xs:group ref="ARCHETYPE"/>`).
- `openehr-its` has NO model-form instance corpus and cannot get one (all 8
  vendored `AOM2/examples/*.xml` are persistent-form; upstream publishes ADL text
  only). `tests/it/aom2_model_xml.rs` is therefore a construct → serialize →
  parse self-consistency gate, and `tests/it/aom2_xml.rs` reads all 8 examples.
  Both assert the archetype BODY is non-empty, because that body sits behind
  `xs:group` references — a codec that dropped group refs would round-trip every
  document vacuously over an empty envelope.
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
  (canonical-JSON corpus round-trips, C14N, schema validation, the R0
  determinism manifest) — never weaken or skip one to get green; a gate failure
  means the emitter or runtime is wrong.
- `tests/it/opt14_corpus.rs` carries TWO parse gates: (1) every vendored `.opt`
  under `app/ferroehr/tests/resources/service` parses + round-trips through the
  generated `opt14::OperationalTemplate`; (2) every official CNF robot
  VALID-template fixture (`docs/specs/openehr/CNF/tests/platform/robot/
  _resources/test_data_sets/valid_templates`) parses, with exactly two fixtures
  adjudicated XSD-invalid (a missing mandatory `OPERATIONAL_TEMPLATE.language`
  per Template.xsd; a missing mandatory `DV_PROPORTION.type` per BaseTypes.xsd)
  — each pinned as an EXPECTED rejection with its citation; a fixture that
  starts parsing must be re-adjudicated, never silently dropped.

## The `flat` module — Simplified Formats (`openehr_its::flat`, hand-written)

FLAT + STRUCTURED data instances, the Web Template model, and the
TDD → COMPOSITION converter (`flat::tdd::from_tdd`, corpus-verified). This is
the ITS-REST **Formats** sub-specification (STABLE) living beside the other
ITS surfaces; it is hand-written because the BMM has no simplified-format
model.

- **The wire oracle is the ITS-REST Simplified Formats specification**
  (`docs/specs/openehr/ITS-REST/docs/simplified_formats/`, STABLE):
  `master04` (field identifiers, node-id algorithm, level removal, `|raw`,
  `|other`, FLAT⇄STRUCTURED algorithms), `master05` (per-RM-type mapping
  tables), `master06` (the `ctx/` vocabulary). SM SIM-B / SDF are
  DEVELOPMENT-state model documents — never implement their terse string
  encodings; SDT is retired. No vendor implementation is an oracle.
- **Architecture: one internal tree** (`flat::sim::SimNode`) — FLAT
  (`flat::sim::flat`) and STRUCTURED (`flat::sim::structured`) are pure codecs
  over it; the template-driven RM conversion is written once (`flat::flatten`
  RM→sim, `flat::build` sim→RM, entry points in `flat::convert`). Datum codecs
  from the `master05` tables live in `flat::map`; the `ctx/` vocabulary in
  `flat::ctx`; the Web Template model/builder in `flat::webtemplate` (node ids
  per `master04 §Node ID Generation Rules`; the document shape serves
  `application/openehr.wt+json`).
- Path/key encoding (`a/b:0/c|unit`) is load-bearing wire surface — no
  ad-hoc changes; every accepted/emitted form needs a spec citation and a
  round-trip test. Spec-example JSON blocks are the primary test vectors;
  the OPT corpus is regression.
- Consumes `openehr-rm`/`openehr-am` types directly (canonical JSON with
  `_type` tagging); never re-models the RM. Carries the crate's ITS 1.1.0
  spec version (the Simplified Formats spec is part of ITS-REST 1.1.0; no
  separate pin).
- Fidelity gates for it live in the crate's `tests/` — never weaken or skip
  one. Two complementary ones: `spec_vectors.rs` replays every
  `simplified_formats` example block for **syntax** + FLAT⇄STRUCTURED
  stability, and `master05_tables.rs` is the **semantic** battery — one test
  per `master05` section, one assertion per mapping-table row (Flat Path +
  Flat type against a minimal RM value through `composition_to_flat`), with
  every row the implementation relocates or does not emit recorded explicitly
  rather than skipped. Plus the `insta` goldens and the OPT-corpus
  round-trips. A new/changed `master05` row lands with its battery row.
