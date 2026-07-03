# Phase 05 — Canonical XML serialization (ITS-XML)

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): RM 1.1.0 / current ITS-XML 2.0.0 / Layer 5b (legacy 1.0.2 descoped)
- Compile required: no (Phase A)

## Objectives

Implement canonical XML serialization and deserialization for the RM classes
against the RM 1.1.0 / current ITS-XML (2.0.0) schemas, using `quick-xml`,
with a `xmllint --c14n` shell fallback for canonical XML (C14N) where exact
byte-for-byte canonicalization matters. The legacy ITS-XML 1.0.2 dual-support
is descoped (see the PARITY NOTE below) — we serialize the latest openEHR
spec only, matching RM 1.1.0.

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
- [x] Spec-completion pass over P1–P3 crates (prerequisite pulled into this
      branch): implemented every `todo!()`/deferred `TODO(port)` in
      `openehr-foundation`, `openehr-base`, `openehr-rm` per the published
      specs (policies fixed in ADR-003) — completed 2026-07-03. Foundation
      (91 tests), base (34), terminology (12), RM (199) all green; the only
      remaining `todo!()` (13, all in RM) cite a published-spec defect
      (DV_PROPORTION/DV_COUNT arithmetic, HISTORY `as_hierarchy`, ITEM_TABLE
      key columns), spec-TBD (`VERSION.canonical_form`), Security-IM scope
      (`EHR_ACCESS.scheme`), or P11 path-evaluator wiring
- [x] Real-world canonical JSON acceptance: vendor the `ehrbase/openEHR_SDK`
      canonical corpus (72 files @ `22b01e0c`) + rewrite `openehr-serde`
      tests to round-trip real EHRbase data (deserialize → re-serialize →
      equality) and ITS-JSON schema-validate, with explicit class-coverage
      and minimal synthetic gap fixtures for the demographic package —
      completed 2026-07-03. Deleted the circular `full_rm_canonical_json`
      suite + 134 hand-built snapshots; `real_world_round_trip` is now the
      primary oracle (72 corpus files + 4 in-repo EHRbase resources),
      `class_coverage` pins the reached-vs-uncovered partition, `gap_fixtures`
      covers demographic synthetically. Interval boolean flags gained
      `#[serde(default)]` read-leniency for archie's default-omission (output
      unchanged, still ITS-JSON 1.1.0 conformant). serde + all four spec
      crates green.
- [ ] Vendor RM 1.1.0 XSDs (`Common.xsd`, `DataTypes.xsd`, `DataStructures.xsd`, `Ehr.xsd`, `Demographic.xsd`) from `specifications-ITS-XML/components/RM/Release-1.1.0/` into `openehr-serde/schemas/`. TARGET RM 1.1.0 / current ITS-XML (2.0.0) ONLY — the legacy 1.0.2 dual-support is DESCOPED per project direction (2026-07-03): we serialize the latest openEHR spec, not two lineages. See PARITY NOTE below.
- [ ] Implement `quick-xml` serialization for rm.data_types matching `DataTypes.xsd`, namespace `http://schemas.openehr.org/v1`
- [ ] Implement `quick-xml` serialization for rm.data_structures matching `DataStructures.xsd`
- [ ] Implement `quick-xml` serialization for rm.common and rm.ehr matching `Common.xsd` and `Ehr.xsd`
- [ ] Implement `quick-xml` serialization for rm.demographic matching `Demographic.xsd`
- [ ] Implement the `xmllint --c14n` shell-out fallback for canonical XML (C14N) comparison in tests
- [ ] Write insta golden-vector tests for XML round-trip (serialize -> deserialize -> equal) per RM package
- [ ] Add PORT STATUS trailers; update `docs/ROSETTA.md` with XML-specific quirks (namespace handling, attribute vs element choices)

## PARITY NOTE — ITS-XML version (added 2026-07-03)

Project direction: target **RM 1.1.0 / current ITS-XML (2.0.0)** only; the
legacy 1.0.2 dual-round-trip is descoped. RM stays pinned at 1.1.0 everywhere
(it always was — the "1.0.2" formerly in scope was the *ITS-XML* schema
lineage, a separate axis from the Reference Model version, not RM 1.0.2).
Open item to confirm when XML ser/de is actually built: verify which ITS-XML
lineage stock EHRbase (v2.33.0 parity baseline) *emits* for canonical XML —
if it emits `v1`-namespace 1.0.x-style XML, byte-parity at the REST surface
may require reading that shape even though we standardize output on the
current schema. Resolve at P5-XML implementation time, before P18 parity.

## Exit criteria

- [ ] Every RM class round-trips through canonical XML with `insta`-pinned output
- [ ] Canonical XML output validates against the RM 1.1.0 / current ITS-XML (2.0.0) XSDs
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
