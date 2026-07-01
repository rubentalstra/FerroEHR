# Phase 02 — Terminology bundle (TERM 3.x) + service API

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): TERM 3.0.0 / Layer 3
- Compile required: should compile (leaf crate)

## Objectives

Bundle the openEHR terminology XML as static assets inside `openehr-terminology`
and implement the terminology service API that `rm.support` and later the AQL
engine and validation layers depend on. This crate is a dependency leaf on top
of `openehr-base` only, so it should actually compile, not just Phase-A capture.

## Preconditions

- [ ] Phase 01 done: `openehr-base` identification types available (terminology
      codes reference `CODE_PHRASE` / `TERMINOLOGY_ID`)

## Scope

In: `openehr_terminology.xml` (en) bundled as a static asset, `PropertyUnitData.xml`,
the terminology service trait/API surface, the `id=532` dual-rubric quirk.
Out: non-English language bundles (behind `lang-{de,es,fr,pt,ja}` feature flags,
add only when needed), FHIR terminology client (that is `openehr-server`'s
`reqwest`-based integration, later).

## Tasks

- [ ] Vendor `specifications-TERM/computable/XML/en/openehr_terminology.xml` into `openehr-terminology/assets/`
- [ ] Vendor `computable/XML/PropertyUnitData.xml` into the same assets directory
- [ ] Parse the terminology XML bundle at build time or first-use into an in-memory lookup structure
- [ ] Implement the terminology service trait (rubric lookup, code-set membership, hierarchy queries) matching `rm.support`'s terminology-service interfaces
- [ ] Preserve the `id=532` dual-rubric quirk (`complete` vs `completed`) with a regression test pinning both rubrics
- [ ] Gate non-English bundles behind `lang-{de,es,fr,pt,ja}` feature flags (stub is acceptable if no bundle is vendored yet)
- [ ] Add PORT STATUS trailers; confirm `cargo check -p openehr-terminology` passes

## Exit criteria

- [ ] `openehr-terminology` compiles standalone (`cargo check -p openehr-terminology`)
- [ ] Terminology service API resolves at least one real rubric from the bundled XML in a test
- [ ] The `id=532` quirk has a pinned regression test

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. This crate has no Java counterpart (EHRbase pulled terminology
handling from `archie`), so it is pure spec transcription against
`specifications-TERM`. Because it sits at a dependency leaf, treat "should
compile" literally — don't leave this one in Phase-A limbo.
