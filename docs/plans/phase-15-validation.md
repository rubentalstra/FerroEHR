# Phase 15 — Composition validation

- Status: not-started (Stage-1 app build, step 7 of 13)
- Consumes: `openehr-rm`, `openehr-term`, P14 (WebTemplate)
- Compile required: yes (compiling, tested increment)
- Decisions: ADR-003 (spec-gap policies), ADR-006

## Objectives

Validate a submitted COMPOSITION against its operational template — EHRbase's
`ValidationWalker` equivalent: structure/cardinality/occurrences against the
WebTemplate, RM invariants, and **terminology binding** against `openehr-term`.
Wired into the P12 create/update path so invalid compositions are rejected with
the correct openEHR error response.

## Preconditions

- [ ] P14 (WebTemplate), P12 (create/update path to hook into)

## Scope

**In:** a validation walker over (composition × WebTemplate) collecting errors
with RM paths; cardinality/occurrences/existence checks; RM invariant checks
(reuse `openehr-*` `*_impl.rs` invariant methods, ADR-003); terminology-bound
code validation (`openehr-term`); map failures to the ITS-REST 422 error body.
**Out:** the AQL engine (P16); FLAT input validation specifics (P17, which reuses
this).

## Tasks

- [ ] Validation walker (composition vs WebTemplate) with path-tagged errors
- [ ] RM invariant + terminology-binding checks
- [ ] Hook into P12 composition create/update; 422 mapping
- [ ] Tests: valid + deliberately-invalid compositions (conformance corpus)

## Exit criteria

- [ ] Valid compositions accepted; invalid ones rejected with correct 422 + paths
- [ ] Terminology-bound codes validated against the bundle
- [ ] Compiles + clippy-clean

## Decisions made this phase

- Genuinely spec-underdetermined checks follow ADR-003 (`todo!` with a cited
  reason, never invented behaviour).
