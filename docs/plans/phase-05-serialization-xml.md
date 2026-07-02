# Phase 05 — Canonical XML serialization (ITS-XML)

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): ITS-XML 1.0.2 + 2.0.0 / Layer 5b
- Compile required: no (Phase A)

## Objectives

Implement canonical XML serialization and deserialization for the RM classes
against the ITS-XML 2.0.0 schemas (TRIAL) while retaining 1.0.2 (STABLE)
round-trip support, using `quick-xml`, with a `xmllint --c14n` shell fallback
for canonical XML (C14N) where exact byte-for-byte canonicalization matters.

## Preconditions

- [x] Phase 03 done: RM classes exist in `openehr-rm` — completed 2026-07-02
- [x] Phase 04 substantially done: JSON serialization decisions (null handling,
      `_type` dispatch equivalents) inform the XML mapping — completed
      2026-07-02; ADR-002 self-tagging is the XML discriminator precedent

## Scope

In: `quick-xml`-based ser/de for every RM class against RM 1.1.0 XSDs
(`Common.xsd`, `DataTypes.xsd`, `DataStructures.xsd`, `Ehr.xsd`,
`Demographic.xsd`), AM 1.4 OPT XSD types consumed later by Phase 09, C14N
fallback.
Out: OPT 1.4 XML parsing proper (Phase 09 owns that; this phase only ensures
the RM-level XML types it needs exist), the legacy EhrExtract XSD (matches
`rm.ehr_extract`, out of scope per Phase 03).

## Tasks

- [ ] Vendor RM 1.1.0 XSDs (`Common.xsd`, `DataTypes.xsd`, `DataStructures.xsd`, `Ehr.xsd`, `Demographic.xsd`) from `specifications-ITS-XML/components/RM/Release-1.1.0/` into `openehr-serde/schemas/`
- [ ] Vendor the legacy 1.0.2 XSD bundle alongside it for round-trip support
- [ ] Implement `quick-xml` serialization for rm.data_types matching `DataTypes.xsd`, namespace `http://schemas.openehr.org/v1`
- [ ] Implement `quick-xml` serialization for rm.data_structures matching `DataStructures.xsd`
- [ ] Implement `quick-xml` serialization for rm.common and rm.ehr matching `Common.xsd` and `Ehr.xsd`
- [ ] Implement `quick-xml` serialization for rm.demographic matching `Demographic.xsd`
- [ ] Implement the `xmllint --c14n` shell-out fallback for canonical XML (C14N) comparison in tests
- [ ] Write insta golden-vector tests for XML round-trip (serialize -> deserialize -> equal) per RM package
- [ ] Write a round-trip test proving 1.0.2 output remains parseable alongside 2.0.0 output
- [ ] Add PORT STATUS trailers; update `docs/ROSETTA.md` with XML-specific quirks (namespace handling, attribute vs element choices)

## Exit criteria

- [ ] Every RM class round-trips through canonical XML with `insta`-pinned output
- [ ] Both ITS-XML 1.0.2 and 2.0.0 bundles are supported for round-trip
- [ ] C14N fallback is invoked successfully in at least one test

## Decisions made this phase

- Spec inputs should be cached or vendored before use. `docs/research/spec-cache/README.md`
  is now the inventory/policy for transcription caches, while computable
  validation inputs stay beside the crate that uses them (e.g.
  `openehr-serde/schemas/` for ITS schemas).
- `jiff` is pinned at workspace version `0.2.31` and wired into
  `openehr-foundation` for the upcoming ISO 8601 temporal implementation;
  existing TODO bodies stay in place until the parser/arithmetic pass fixes
  openEHR partial-precision semantics.

## Handoff for next session

Started with dependency/spec-cache prep. `quick-xml` is pinned in the root
`Cargo.toml`; confirm its derive ergonomics against a representative
`DV_QUANTITY` or `COMPOSITION` example before committing to a pattern for the
whole crate. Next concrete task is still vendoring the ITS-XML XSD bundles.
