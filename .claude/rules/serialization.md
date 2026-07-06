---
paths: ["crates/openehr-its/**", "crates/openehr-flat/**"]
---

# Serialization rules — canonical JSON, canonical XML, FLAT/STRUCTURED

## Spec sources (the oracle)

The vendored normative text is the authority for every wire shape
(spec-adherence.md; use `/spec-lookup`):
`docs/specs/openehr/ITS-JSON/components/` (canonical-JSON schemas, same pin as
the fidelity gate), `docs/specs/openehr/ITS-XML/` + the XSDs at
`crates/openehr-its/schemas/xml/` (canonical XML),
`docs/specs/openehr/SM/docs/serial_data_formats/` +
`SM/docs/simplified_im_b/` (SDT — the FLAT/STRUCTURED semantics), and the CNF
content test cases (`docs/specs/openehr/CNF/docs/platform_test_schedule/`
`master15`-`master17.x` + the canonical JSON/XML fixtures under
`CNF/tests/platform/robot/_resources/`) which define the exact shapes a
conformant server must emit/accept.

## The `_type` mechanism (ADR-004 supersedes ADR-002 — do not improvise)

Canonical-JSON `_type` self-tagging is now handled by the
**`#[derive(OpenEhrType)]`** proc-macro (`openehr-derive`), emitted on every
generated concrete class by `openehr-codegen`. There is **no** `TypeTag` field
and no `openehr_foundation::serde_support` module any more (both deleted with
ADR-004). Do not add them back.

What the derive does (so you know the contract, not so you hand-write it):

- **Serialize** emits `"_type": "<CLASS>"` as the first entry, then each field;
  `Option` fields are omitted when `None`, `Vec` fields when empty (no `null`s).
- **Deserialize** accepts input with or without `_type`; a present-but-wrong
  `_type` is an error (this is what lets an untagged enum dispatch on the tag).
- Closed subtype-set enums are `#[serde(untagged)]` (the emitter writes these);
  never `#[serde(tag = "_type")]`.

If a `_type`/serialization behaviour is wrong, fix the **emitter** or the
**derive macro** and regenerate — never hand-edit a `// @generated` file.

`openehr-its` (canonical JSON entry points + canonical **XML** + the interop
fidelity gate) and `openehr-flat` (FLAT/STRUCTURED/Web Template) are
hand-written from specifications and vendor conventions (PORT_MASTER_PLAN.md
Sections 7.3, 7.4). The acceptance instrument for JSON is the **real EHRbase
canonical-JSON corpus** round-trip in `openehr-its/tests/vendor/`
(deserialize → re-serialize → normalized value-equality + ITS-JSON schema
validation), not hand-built fixtures; `insta` golden vectors are the instrument
for XML and FLAT.

## Canonical JSON (ITS-JSON)

- Attribute names are snake_case.
- `_type` discriminator is the uppercase RM class name (`DV_TEXT`,
  `COMPOSITION`) and is **required** whenever the statically declared field
  type is abstract (e.g. any `DATA_VALUE`-typed field must carry `_type`;
  a field statically typed as the concrete `DvText` need not).
- Metadata keys are `_`-prefixed.
- Abstract classes are flattened into the concrete JSON shape, never
  `$ref`-chained.
- Nulls are omitted entirely, not serialized as `null`.
- UIDs serialize as `{"_type": "...", "value": "..."}`, never as a bare
  string.
- `DV_MULTIMEDIA.data` is inline base64, not a separate reference.
- Validate output against `openehr_rm_1.1.0_all.json` (pin the exact commit
  hash used, per Section 7.3 — ITS-JSON has no numbered release).

## Canonical XML (ITS-XML)

**GENERATED (ADR-005 supersedes the "hand-written / not a codegen source" rule
below).** The `ToXml`/`FromXml` impls for the RM/BASE types are emitted by
`openehr-codegen`'s `emit-xml` target into `openehr-its/src/xml/generated/`,
driven by the vendored XSDs (the wire shape) + the BMM model (the Rust fields).
Do **not** hand-edit a `// @generated` XML impl — change the emitter
(`openehr-codegen/src/emit_xml.rs` / `xsd.rs`) and regenerate. The hand-written
part is the runtime (`xml/runtime.rs`: the `ToXml`/`FromXml` traits + `quick-xml`
reader/writer) and the entry points. Regenerate with
`cargo run -p openehr-codegen -- emit-xml`.

- Namespace is `http://schemas.openehr.org/v1` for both schema versions.
- Support both the TRIAL 2.0.0 XSDs and the legacy STABLE 1.0.2 bundle for
  round-trip; do not drop 1.0.2 support once 2.0.0 lands.
- C14N (canonical XML) uses the `xmllint --c14n` shell fallback for now —
  do not hand-roll a C14N implementation.
- Use `quick-xml` as the XML layer. Per ADR-005 the XSD **is** a codegen
  source (it supplies element order + the attribute/element split + `xsi:type`
  slots that BMM does not encode); it remains a validation target too.

## FLAT / STRUCTURED / Web Template

- Target Better's `web-template` semantics
  (`github.com/better-care/web-template` + `web-template-tests`) as the
  primary oracle, since SDT (the standard successor) is still development.
- DV_QUANTITY uses `|unit` (**singular**) in both Better and EHRbase — there is
  no live `|unit`/`|units` divergence (verified against Better `web-template` +
  EHRbase docs + the corpus, 2026-07). Genuine Better-specific extras are
  `|unit_system` / `|unit_display_name`; any such EHRbase/Better-only extension
  lives behind the `ehrbase-quirks` feature flag — never hard-code a quirk into
  the default path.
- MIME types: `application/openehr.wt+json`,
  `application/openehr.wt.flat+json`,
  `application/openehr.wt.structured+json`.

## General

Hand-written files here (the XML layer, the fidelity-gate tests, FLAT) carry
the PORT STATUS trailer (source = the spec section or vendor doc consulted) and
the annotation vocabulary from `rust-style.md`. Generated files (none live here
today) never carry the trailer. Redact volatile fields (timestamps, generated
UUIDs) in `insta` snapshots before committing them.
