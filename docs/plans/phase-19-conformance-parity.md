# Phase 19 — openEHR conformance (ADR-008)

> Re-scoped 2026-07-05 by ADR-008: the acceptance instrument is the openEHR
> CNF conformance framework; the EHRbase-diff parity harness is retired.

- Status: not-started (Stage-1 app build, step 11 of 13)
- Consumes: the fully integrated server (P18)
- Compile required: conformance (the acceptance bar)
- Decisions: ADR-008

## Objectives

Prove **openEHR specification conformance** at the REST/AQL surface: run the
official CNF Platform Conformance Test Schedule (REST + JSON realization)
against our server, plus the corpus suites (openEHR conformance corpora,
canonical JSON/XML fidelity gates, AQL corpus), and close every divergence.

## Preconditions

- [ ] P18 (server runs end to end)
- [ ] Smoke-conformance wiring in place since P12 (grow-as-you-go)

## Scope

**In:** vendoring/adapting the CNF test schedule + runners
(`openEHR/specifications-CNF`); a conformance runner (`scripts/conformance.sh`)
+ CI wiring; triaging and fixing divergences; documenting deliberate spec-gap
decisions (ADR-003 style). **Out:** performance (P20); EHRbase parity
(retired by ADR-008).

## Tasks

- [ ] Vendor/wire the CNF platform test schedule (REST+JSON profile)
- [ ] Conformance runner script + CI job
- [ ] Run the full schedule; triage + fix divergences
- [ ] Corpus suites green (canonical JSON/XML, AQL corpus, composition CRUD)
- [ ] Deviation register: residual/deliberate deviations with spec citations

## Exit criteria

- [ ] CNF schedule passes (documented exceptions only) on Linux x86_64
- [ ] All corpus suites green in CI
- [ ] Deviation register complete

## Decisions made this phase

- (record conformance exceptions here)
