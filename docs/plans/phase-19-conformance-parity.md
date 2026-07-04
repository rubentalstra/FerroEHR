# Phase 19 — Conformance & parity

- Status: not-started (Stage-1 app build, step 11 of 13)
- Consumes: the fully integrated server (P18)
- Compile required: parity (the acceptance bar)
- Decisions: `PORT_MASTER_PLAN §15`, ADR-006 (parity harness is the acceptance instrument)

## Objectives

Prove behavioural equivalence with EHRbase at the REST/AQL surface — the
acceptance bar for the whole "faithful, spec-conformant" goal (ADR-006, since we
do **not** class-mirror Java). Run the openEHR conformance corpora + the parity
harness, which drives our server and a stock EHRbase with identical requests and
diffs responses, behind the negative-test gate.

## Preconditions

- [ ] P18 (server runs end to end)

## Scope

**In:** the parity harness (`scripts/parity.sh`) driving both servers + diffing
(JSON/XML canonical, status codes, headers); the `USE_REFERENCE_EHRBASE=1`
negative gate (a parity test must fail against stock EHRbase without our fix);
openEHR conformance corpora + EHRbase `test-data` + Better `web-template-tests`;
`insta` snapshots for canonical output; target ≥99% parity on Linux x86_64 first.
**Out:** performance tuning (P20).

## Tasks

- [ ] Build/complete the parity harness + negative-test gate
- [ ] Run conformance corpora (EHR/composition/directory/contribution/query/definition)
- [ ] Triage + fix divergences (error bodies, headers, AQL edge semantics)
- [ ] Snapshot canonical outputs; CI wiring

## Exit criteria

- [ ] ≥99% parity at the REST surface (Linux x86_64); documented residual diffs
- [ ] Negative gate holds (tests fail vs stock EHRbase without our fix)
- [ ] Conformance corpora pass

## Decisions made this phase

- The parity harness — not class-level diffing — is how "behaviour-compatible"
  is proven (ADR-006).
