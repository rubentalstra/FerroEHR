# Phase 05 — Canonical XML serialization (ITS-XML)

- Status: **done** — GENERATED from XSD + BMM (ADR-005)
- Build order: complete (spec foundation)
- Decisions: ADR-005

## Outcome

`openehr-codegen`'s `emit-xml` target generates `impl ToXml`/`impl FromXml` for
every RM/BASE type into `openehr-its/src/xml/generated/`, over a hand-written
`quick-xml` runtime (`openehr-its/src/xml/runtime.rs`). Wire shape (element
order, attribute-vs-element split, `xsi:type` dispatch) comes from the vendored
XSD reader; Rust field facts from the BMM model. One impl set serves both ITS-XML
namespaces (v1 = parity target, v2 vendored) — the namespace is a serialize-time
param. `Hash<String,String>` = openEHR `StringDictionaryItem`. Prefixed
`xsi:type` and archie-omitted `Interval` flags are handled on read.

## Verification

`openehr-its` XML gates green: 48-composition round-trip
(`tests/xml_roundtrip.rs`), real EHRbase XML fixtures read + round-trip
(`tests/xml_ehrbase.rs`), `Hash` round-trip (`tests/xml_hash.rs`). Regenerate
with `cargo run -p openehr-codegen -- emit-xml`.
