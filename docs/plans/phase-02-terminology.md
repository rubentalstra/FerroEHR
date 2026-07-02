# Phase 02 — Terminology bundle (TERM 3.x) + service API

- Status: done
- Started: 2026-07-02   Completed: 2026-07-02   Owner: Ruben
- Consumes (spec/layer): TERM 3.0.0 / Layer 3
- Compile required: should compile (leaf crate)

## Objectives

Bundle the openEHR terminology XML as static assets inside `openehr-terminology`
and implement the terminology service API that `rm.support` and later the AQL
engine and validation layers depend on. This crate is a dependency leaf on top
of `openehr-base` only, so it should actually compile, not just Phase-A capture.

## Preconditions

- [x] Phase 01 done: `openehr-base` identification types available (terminology
      codes reference `CODE_PHRASE` / `TERMINOLOGY_ID`) — P1 files exist but are
      unwired (Phase A), hence the local `TerminologyCode` stand-in below

## Scope

In: `openehr_terminology.xml` (en) bundled as a static asset, `PropertyUnitData.xml`,
the terminology service trait/API surface, the `id=532` dual-rubric quirk.
Out: non-English language bundles (behind `lang-{de,es,fr,pt,ja}` feature flags,
add only when needed), FHIR terminology client (that is `openehr-server`'s
`reqwest`-based integration, later).

## Tasks

- [x] Vendor `specifications-TERM/computable/XML/en/openehr_terminology.xml` into `openehr-terminology/assets/` — plus es/ja/pt bundles, the external ISO/IANA code sets, and the three XSDs (Release-3.0.0 @ d45ef3e)
- [x] Vendor `computable/XML/PropertyUnitData.xml` into the same assets directory — parsed model + units_for_openehr_property, pinned test (521 units; Mass includes kg)
- [x] Parse the terminology XML bundle at build time or first-use into an in-memory lookup structure — include_str! + quick-xml pull parser, LazyLock first-use singleton (`TerminologyService::bundled()`)
- [x] Implement the terminology service trait (rubric lookup, code-set membership, hierarchy queries) matching `rm.support`'s terminology-service interfaces — TERMINOLOGY_SERVICE/TERMINOLOGY_ACCESS/CODE_SET_ACCESS + both identifier constants classes, transcribed from the cached RM 1.1.0 support tables
- [x] Preserve the `id=532` dual-rubric quirk (`complete` vs `completed`) with a regression test pinning both rubrics — two tests: verbatim bundle duplicates + document-order lookup semantics
- [x] Gate non-English bundles behind `lang-{de,es,fr,pt,ja}` feature flags (stub is acceptable if no bundle is vendored yet) — es/ja/pt vendored and gated; de/fr remain declared-but-empty (upstream ships no such bundles at 3.0.0)
- [x] Add PORT STATUS trailers; confirm `cargo check -p openehr-terminology` passes — trailers on all 10 files; cargo TEST passes (12/12), clippy zero warnings

## Exit criteria

- [x] `openehr-terminology` compiles standalone (`cargo check -p openehr-terminology`)
- [x] Terminology service API resolves at least one real rubric from the bundled XML in a test — 249 → "creation" (audit change type)
- [x] The `id=532` quirk has a pinned regression test

## Decisions made this phase

- Service signatures use a crate-local `TerminologyCode` stand-in for the
  spec's `CODE_PHRASE` (openehr-rm depends on this crate, not vice versa);
  reconcile at P3/P17 — ROSETTA row records it.
- Three editorial defects in the published TERMINOLOGY_ACCESS table (bare
  CODE_PHRASE return, parameterless has_code_for_group_id, rubric language)
  and the Boolean-typed `an_id` in the group-identifiers table transcribed
  per evident intent, each flagged with a PORT NOTE.
- Spec preconditions surface as `Option` returns, not panics.
- `openehr_external_terminologies.xml` vendored too — all 7 spec code-set
  identifiers resolve against real data (countries, character sets,
  languages, media types come from the external file).
- Workspace `clippy.toml`: `allow-expect-in-tests`/`allow-unwrap-in-tests`,
  encoding the existing "no unwrap/expect outside tests" rule.
- Dependency pins: the verify-marked workspace pins were corrected to real
  current releases (utoipa-axum 0.2, sea-query 1.0.1, quick-xml 0.41,
  ordered-float 5, serde_jcs 0.2, hmac 0.13, utoipa-redoc/scalar/rapidoc);
  quick-xml 0.41 renamed unescape_value → normalized_value(XmlVersion),
  migrated accordingly.

## Handoff for next session

P2 complete: openehr-terminology is the first compiled, tested crate (12
tests, zero clippy warnings) — bundle parser, property/unit data, and the
full rm.support terminology service surface over vendored Release-3.0.0
assets. Next is P3 (docs/plans/phase-03-rm.md), the big RM transcription:
extend docs/research/spec-cache/RM-1.1.0/ (support/ chapters already cached)
with the remaining RM packages and run rm-transcriber waves per package
(data_types → data_structures → common → ehr → demographic → integration),
Phase A rules. Remember the CODE_PHRASE ↔ TerminologyCode reconciliation
note when transcribing data_types.text.
