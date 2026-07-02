---
paths: ["crates/openehr-serde/**", "crates/openehr-flat/**"]
---

# Serialization rules — canonical JSON, canonical XML, FLAT/STRUCTURED

## The `_type` mechanism (ADR-002 — normative, do not improvise)

Every concrete RM/BASE class self-tags. The mechanism is
`openehr_foundation::serde_support::{TypeName, TypeTag}`:

- `impl TypeName for Foo { const NAME: &'static str = TYPE_NAME; }`
  (single-source the string from the file's existing `TYPE_NAME` const).
- First struct field:
  `#[serde(rename = "_type", default = "TypeTag::new")] pub type_tag:
  TypeTag<Self>` (generics: `TypeTag<Foo<T>>` + `impl<T> TypeName for
  Foo<T>`). The function-path `default = "TypeTag::new"` is mandatory —
  bare `default` adds a spurious `T: Default` bound on generic containers.
- Closed subtype-set enums are `#[serde(untagged)]`; never
  `#[serde(tag = "_type")]` (it duplicates the payload's own tag). List
  structurally richer variants first (`DvCodedText` before bare `DvText`).
- Abstract classes and embedded `*Data` structs get **no** tag; a `*Data`
  struct doubling as a bare concrete parent (`DvTextData` ≙ plain
  `DV_TEXT`) implements `TypeName` so the enum's bare variant can carry
  `TypeTag<FooData>` beside the `#[serde(flatten)]`ed data.
- Struct-level `#[serde(rename = "CLASS")]` is a verified no-op on the wire
  — delete it wherever found; it must never stand in for a `_type` tag.

`openehr-serde` (canonical JSON + canonical XML) and `openehr-flat`
(FLAT/STRUCTURED/Web Template) have no Java to port — both are written from
specifications and vendor conventions (PORT_MASTER_PLAN.md Sections 7.3,
7.4, 14.4). `insta` golden vectors are the acceptance instrument for both:
a serializer is not done until its output matches a pinned snapshot.

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

- Namespace is `http://schemas.openehr.org/v1` for both schema versions.
- Support both the TRIAL 2.0.0 XSDs and the legacy STABLE 1.0.2 bundle for
  round-trip; do not drop 1.0.2 support once 2.0.0 lands.
- C14N (canonical XML) uses the `xmllint --c14n` shell fallback for now —
  do not hand-roll a C14N implementation.
- Use `quick-xml` (`serialize` feature) as the XML layer; JAXB/XSD-generated
  Java classes have no Rust equivalent to bind to — the schema is a
  validation target, not a codegen source.

## FLAT / STRUCTURED / Web Template

- Target Better's `web-template` semantics
  (`github.com/better-care/web-template` + `web-template-tests`) as the
  primary oracle, since SDT (the standard successor) is still development.
- EHRbase-specific quirks (e.g. Better's `|unit` vs SDT's `|units`) live
  behind the `ehrbase-quirks` feature flag on this crate — never hard-code a
  quirk into the default path.
- MIME types: `application/openehr.wt+json`,
  `application/openehr.wt.flat+json`,
  `application/openehr.wt.structured+json`.

## General

Every file here still needs the PORT STATUS trailer (source = the spec
section or vendor doc consulted, e.g. "ITS-JSON commit `<hash>`, §DV_TEXT")
and the annotation vocabulary from `rust-style.md`. Redact volatile fields
(timestamps, generated UUIDs) in `insta` snapshots before committing them.
