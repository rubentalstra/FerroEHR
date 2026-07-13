---
paths: ["crates/openehr-its/**", "crates/openehr-flat/**"]
---

# Serialization rules — canonical JSON, canonical XML, FLAT/STRUCTURED
(rewritten 2026-07-13; the patched-in ADR-002/005 supersession notes are folded in)

## Spec sources (the oracle)

The vendored normative text is the authority for every wire shape
(spec-adherence.md; `/spec-lookup`):
`docs/specs/openehr/ITS-JSON/components/` (canonical-JSON schemas — the pin is
recorded in `docs/VERSIONS.md`), `docs/specs/openehr/ITS-XML/` + the vendored
XSDs at `crates/openehr-its/schemas/xml/` (canonical XML),
`docs/specs/openehr/SM/docs/serial_data_formats/` + `SM/docs/simplified_im_b/`
(SDT — FLAT/STRUCTURED semantics), and the CNF content chapters
(`docs/specs/openehr/CNF/docs/platform_test_schedule/master15`–`master17.x`)
with the canonical fixtures under `CNF/tests/platform/robot/_resources/` as
reference material.

## The `_type` mechanism (generated — do not improvise)

Canonical-JSON `_type` self-tagging is the **`#[derive(OpenEhrType)]`**
proc-macro (`openehr-derive`), emitted on every generated concrete class by
`openehr-codegen`. The contract (know it; never hand-write it):

- **Serialize** emits `"_type": "<CLASS>"` first, then fields; `Option`
  fields are omitted when `None`, `Vec` fields when empty — no `null`s.
- **Deserialize** accepts input with or without `_type`; present-but-wrong
  `_type` is an error (this is what lets untagged enums dispatch).
- Closed subtype-set enums are `#[serde(untagged)]` (emitter-written);
  never `#[serde(tag = "_type")]`.

Wrong `_type`/serialization behaviour → fix the **emitter or the derive
macro** and regenerate (`/regen-codegen`); never hand-edit `// @generated`.

## Canonical JSON (ITS-JSON)

- Attribute names snake_case; metadata keys `_`-prefixed.
- `_type` is the uppercase RM class name and is **required** whenever the
  statically declared field type is abstract (any `DATA_VALUE`-typed slot);
  a concretely-typed field may omit it.
- Abstract classes flatten into the concrete shape; nulls are omitted
  entirely; UIDs serialize as `{"_type": …, "value": …}`, never a bare
  string; `DV_MULTIMEDIA.data` is inline base64.
- Validation oracle: the vendored `openehr_rm_1.1.0_all.json` schema
  (`crates/openehr-its/schemas/json/`, exact commit pinned in
  `docs/VERSIONS.md` — ITS-JSON has no numbered release). Note the schema's
  RM-1.1.0 ceiling vs our RM 1.2.0 where relevant.

## Canonical XML (ITS-XML) — GENERATED

`ToXml`/`FromXml` impls are emitted by `emit-xml` into
`openehr-its/src/xml/generated/` from the vendored XSDs (wire shape: element
order, attribute/element split, `xsi:type` slots) + the BMM model (Rust
fields). Hand-written parts: the runtime (`xml/runtime.rs`, `quick-xml`) and
entry points.

- Namespace `http://schemas.openehr.org/v1`; **one impl set serves both the
  1.0.2 (STABLE, the target) and 2.0.0 (TRIAL) bundles** — they differ only
  by root `xmlns`, selected at serialize time. Both XSD bundles stay
  vendored.
- `xsi:type` is emitted iff the concrete type differs from the declared slot
  type; inbound dispatch routes deep types through the descendant→variant
  map.
- C14N uses the `xmllint --c14n` shell fallback — never hand-roll C14N.

## FLAT / STRUCTURED / Web Template (`openehr-flat`, hand-written)

- Better's `web-template` semantics
  (`github.com/better-care/web-template` + `web-template-tests`) are the
  primary interop oracle while SDT remains development-status; SDF-normative
  divergences carry a `// PORT NOTE:` and are reconciled by the P17 interop
  audit.
- `DV_QUANTITY` uses `|unit` (singular) — verified against Better + the
  corpus (2026-07); genuine Better-only extras (`|unit_system`,
  `|unit_display_name`) live behind the `ehrbase-quirks` feature flag, never
  in the default path.
- MIME types: `application/openehr.wt+json`, `…wt.flat+json`,
  `…wt.structured+json`.

## Acceptance instruments

- **JSON:** the vendored canonical-JSON corpus round-trip gates in
  `openehr-its/tests/` (deserialize → re-serialize → normalized value
  equality + ITS-JSON schema validation) — real corpus data, not hand-built
  fixtures.
- **XML:** the round-trip + C14N gates; **FLAT:** `insta` golden vectors +
  the Better test corpus.
- Redact volatile fields (timestamps, generated UUIDs) before committing
  any snapshot. Never weaken a gate to get green (testing.md).
- At the wire, the ECC suite (`tools/conformance`) is the end-to-end
  acceptance instrument.

Hand-written files here follow `rust-style.md` (annotation vocabulary; **no
PORT STATUS trailer — that convention is retired**). Generated files carry
`// @generated` and are never hand-edited.
