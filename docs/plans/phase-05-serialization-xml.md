# Phase 05 — Canonical XML serialization (ITS-XML)

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): RM 1.1.0 model / ITS-XML 1.0.2 STABLE, namespace `http://schemas.openehr.org/v1` / Layer 5b
- Compile required: no (Phase A)

## Objectives

Implement canonical XML serialization and deserialization for the RM classes
against the **ITS-XML 1.0.2 STABLE** schemas (namespace
`http://schemas.openehr.org/v1`), using `quick-xml`, with a `xmllint --c14n`
shell fallback for canonical XML (C14N) where exact byte-for-byte
canonicalization matters. v1/1.0.2 is the target because it is (a) the latest
*stable* ITS-XML release — 2.0.0 is TRIAL/in-development — and (b) exactly
what stock EHRbase emits (`archie` uses the v1-namespace schemas), so it is
what a 1:1 faithful port requires. The RM *model* stays 1.1.0 internally; only
the XML *wire format* is the v1 lineage. See the DECISION note below.

## Preconditions

- [x] Phase 03 done: RM classes exist in `openehr-rm` — completed 2026-07-02
- [x] Phase 04 substantially done: JSON serialization decisions (null handling,
      `_type` dispatch equivalents) inform the XML mapping — completed
      2026-07-02; ADR-002 self-tagging is the XML discriminator precedent

## Scope

In: `quick-xml`-based ser/de for the RM classes EHRbase serializes, against
the vendored v1 ITS-XML 1.0.2 bundle
(`crates/openehr-serde/schemas/xml/its-xml-1.0.2-nsv1/`): `BaseTypes.xsd`
(data types + identification), `Structure.xsd` (data structures + LOCATABLE),
`Content.xsd` (EHR entries), `Composition.xsd` (COMPOSITION/EVENT_CONTEXT),
`Version.xsd` (change control), `Resource.xsd`. C14N fallback.
Out: OPT/template XML (`Archetype.xsd`, `Template.xsd`, `CompositionTemplate.xsd`,
`OpenehrProfile.xsd` — Phase 09 owns that; this phase only ensures the
RM-level XML types exist), the `Extract.xsd` EHR-extract (matches
`rm.ehr_extract`, out of scope per Phase 03), and demographic XML (the v1
`ALL/` bundle ships no demographic schema — EHRbase does not emit demographic
XML; defer to a synthetic pass only if a later phase needs it).

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
- [x] Vendor the ITS-XML XSD bundles for reference — completed 2026-07-03.
      Both lineages are under `crates/openehr-serde/schemas/xml/` with
      `PROVENANCE.md`: `its-xml-1.0.2-nsv1/` (the v1-namespace STABLE bundle,
      tag `Release-1.0.2v2` @ `f7a93777`, = what EHRbase emits, the P5 TARGET)
      and `its-xml-2.0.0-nsv2/` (RM 1.1.0 + BASE 1.2.0 + AM 1.4 + OET + QUERY,
      `master` @ `de8b37ba`, retained as latest-spec / Stage-3 reference).
- [ ] Implement `quick-xml` ser/de for rm.data_types + identification matching `BaseTypes.xsd`, namespace `http://schemas.openehr.org/v1`, attribute-based `xsi:type` discriminators
- [ ] Implement `quick-xml` ser/de for rm.data_structures + rm.common LOCATABLE matching `Structure.xsd`
- [ ] Implement `quick-xml` ser/de for rm.ehr content/entries matching `Content.xsd`
- [ ] Implement `quick-xml` ser/de for COMPOSITION/EVENT_CONTEXT (`Composition.xsd`) and change-control VERSION/CONTRIBUTION/AUDIT_DETAILS (`Version.xsd`) + AUTHORED_RESOURCE (`Resource.xsd`)
- [ ] Implement the `xmllint --c14n` shell-out fallback for canonical XML (C14N) comparison in tests
- [ ] Write XML round-trip tests (serialize -> deserialize -> equal) using the real EHRbase v1 composition fixtures already in-repo (`crates/openehr-server/tests/resources/service/samples/*.xml`) as the primary oracle, mirroring the JSON real-world-corpus approach
- [ ] Add PORT STATUS trailers; update `docs/ROSETTA.md` with XML-specific quirks (v1 namespace, `xsi:type` discriminator vs JSON `_type`, attribute vs element choices, `archetype_node_id` as an attribute)

## DECISION — ITS-XML namespace/version (settled 2026-07-03)

**Target ITS-XML 1.0.2 STABLE, namespace `http://schemas.openehr.org/v1`.**

The "RM version" and the "ITS-XML namespace" are independent axes. RM stays
pinned at 1.1.0 everywhere (JSON is unaffected). For XML specifically, v1/1.0.2
wins on both criteria that matter:

1. **Latest STABLE.** ITS-XML 2.0.0 (namespace `v2`) is TRIAL/in-development;
   1.0.2 is the current STABLE release. Latest-stable ≠ latest-dev.
2. **1:1 parity.** Stock EHRbase (parity baseline v2.33.0) emits v1-namespace
   canonical XML — confirmed in-repo: `TemplateServiceImp.java` sets the OPT
   root QName to `http://schemas.openehr.org/v1`, and the real composition
   fixtures at `crates/openehr-server/tests/resources/service/samples/*.xml`
   declare `xmlns:v1="http://schemas.openehr.org/v1"` with attribute-based
   `xsi:type` discriminators. EHRbase's RM XML comes from `archie`, which
   bundles the v1 schemas.

Adopting v2/RM-1.1.0 XML is a Stage-3 improvement, not part of the faithful
port. The v2 bundle is vendored (`its-xml-2.0.0-nsv2/`) only as reference for
that later work. (This reverses an earlier same-day scoping note that had
tentatively targeted v2; the evidence above settled it toward v1.)

## Exit criteria

- [ ] The RM classes EHRbase serializes round-trip through v1 canonical XML with output pinned/validated
- [ ] Canonical XML output validates against the vendored v1 ITS-XML 1.0.2 XSDs (`its-xml-1.0.2-nsv1/`) and matches the in-repo EHRbase composition fixtures
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
