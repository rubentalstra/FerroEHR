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

- [x] Implement foundation/RM ISO 8601 temporal value semantics needed by
      scalar JSON/XML round-trips — completed 2026-07-02; added the
      `openehr-foundation` BASE time parser, wired `Time_Definitions` and
      `Iso8601_*` accessors/validity/comparison, and delegated
      `DV_DATE`/`DV_TIME`/`DV_DATE_TIME`/`DV_DURATION` magnitude/equality/
      validity to it
- [x] Implement BASE UID and RM URI scalar accessors needed by scalar
      JSON/XML round-trips — completed 2026-07-02; `UID_BASED_ID.root()`,
      `OBJECT_VERSION_ID.object_id()` and `creating_system_id()` now
      classify strings via the official BASE UID grammar, while `DV_URI`
      extracts scheme/path/query/fragment without normalising the stored
      string so plain-text openEHR URI values remain intact
- [ ] Spec-completion pass over P1–P3 crates (prerequisite pulled into this
      branch): implement every `todo!()`/deferred `TODO(port)` in
      `openehr-foundation`, `openehr-base`, `openehr-rm` per the published
      specs (policies fixed in ADR-003), leaving only markers that cite a
      published-spec defect or a named later phase
- [ ] Real-world canonical JSON acceptance: round-trip + ITS-JSON schema
      validation of the in-repo EHRbase fixtures (max-conformance
      composition, Corona composition, config composition/EHR_STATUS)
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

- ADR-003 fixes the policies for every spec-underdetermined behaviour hit by
  the P1–P3 spec-completion pass: definite vs nominal ISO 8601 arithmetic
  (jiff-backed), partial-precision anchoring, `Integer.modulo` truncated
  division, RFC 3986 URI validation via the `url` crate, the `Container<T>`
  iteration primitive, `Any.instance_of` as a documented deviation, and
  invariants-as-working-methods ahead of the P11 validation framework.
- Spec inputs should be cached or vendored before use. `docs/research/spec-cache/README.md`
  is now the inventory/policy for transcription caches, while computable
  validation inputs stay beside the crate that uses them (e.g.
  `openehr-serde/schemas/` for ITS schemas).
- `jiff` is pinned at workspace version `0.2.31` and wired into
  `openehr-foundation`. The first parser-backed ISO 8601 implementation now
  covers BASE date/time/timezone/duration validity, component accessors,
  partial/extended detection, comparisons, duration arithmetic, and the RM
  temporal magnitude/equality/validity delegates. Calendar add/subtract over
  partial dates/date-times remains deliberately TODO(port) until a policy is
  chosen for incomplete precision.
- BASE UID strings are classified from the Release 1.2.0 grammar in
  `master05-identification_package.adoc`: ISO OID, canonical 8-4-4-4-12
  UUID, then reduced INTERNET_ID labels. Spec-named accessors stay total for
  valid model values; fallible Rust accessors are available for unchecked raw
  strings until the Validate framework enforces invariants at construction or
  deserialization boundaries.
- RM `DV_URI` accessors intentionally do not use a normalising URL parser for
  output; the Release 1.1.0 URI package allows plain-text strings and stores
  the URI as a `String`, so scheme/path/query/fragment extraction preserves
  spaces, dot segments, and the absence of an authority path.

## Handoff for next session

Started with dependency/spec-cache prep. `quick-xml` is pinned in the root
`Cargo.toml`; temporal scalar semantics now compile and have focused tests in
`openehr-foundation` and `openehr-rm`, and UID/URI scalar accessors now have
focused tests in `openehr-base` and `openehr-rm`. Confirm `quick-xml` derive
ergonomics against a representative `DV_QUANTITY` or `COMPOSITION` example
before committing to a pattern for the whole crate. Next concrete task is
still vendoring the ITS-XML XSD bundles.
