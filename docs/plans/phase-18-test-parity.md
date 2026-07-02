# Phase 18 — Test parity

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): the entire workspace (Phases 00-17)
- Compile required: parity

## Objectives

Reach behavioral equivalence with stock EHRbase at the REST surface, using
openEHR conformance corpora, EHRbase's `test-data`/`serialisation_conformance_test`
sets, and archie golden vectors, targeting >=99% parity on Linux x86_64 first.
Every claimed-equivalent behavior is gated behind the `USE_REFERENCE_EHRBASE=1`
negative-test gate: a parity test is only valid if it still fails against
stock EHRbase without our fix.

## Preconditions

- [ ] Phase 17 done: workspace compiles with zero errors

## Scope

In: parity harness runs against openEHR conformance corpora, EHRbase
`test-data`, `serialisation_conformance_test`, archie golden vectors, insta
snapshot review, the `USE_REFERENCE_EHRBASE=1` negative-test gate.
Out: performance tuning (Phase 19), any new feature not present in stock
EHRbase, non-Linux-x86_64 platforms (broaden only after the first target is
green).

## Tasks

- [ ] Stand up the parity harness driving both the Rust server and a stock Java EHRbase with identical requests, diffing responses
- [ ] Implement the `USE_REFERENCE_EHRBASE=1` mode: run the same test suite against stock EHRbase alone and confirm each parity test fails without our fix
- [ ] Run the openEHR conformance corpora through the parity harness and triage failures
- [ ] Run EHRbase's `test-data` set through the parity harness and triage failures
- [ ] Run EHRbase's `serialisation_conformance_test` set through the parity harness and triage failures
- [ ] Run archie golden vectors against canonical JSON/XML serialization (Phases 04-05) and triage failures
- [ ] Review and accept/reject `insta` snapshot diffs via `cargo insta review`, with redactions for volatile fields (timestamps, generated UIDs)
- [ ] Fix triaged failures, looping back into the relevant earlier phase file to note the fix
- [ ] Compute and record the final parity percentage on Linux x86_64
- [ ] Add regression tests for every parity gap found and fixed, each with a passing negative-test gate result

## Exit criteria

- [ ] Parity harness reports >=99% behavioral parity at the REST surface on Linux x86_64
- [ ] Every parity-motivated fix has a corresponding test that fails under `USE_REFERENCE_EHRBASE=1` (i.e. against stock EHRbase without the fix) and passes against the Rust server
- [ ] No existing test was weakened, skipped, or deleted to reach this parity number

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. The negative-test gate is non-negotiable (Section 4.5 / Section
15): if a "parity" test passes against stock EHRbase even without our fix, it
is not a valid parity test and must be redesigned, not counted toward the
99% figure.
