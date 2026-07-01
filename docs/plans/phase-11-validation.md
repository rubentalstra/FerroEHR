# Phase 11 — Composition validation

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): RM, WebTemplate (Phases 03, 10)
- Compile required: no (Phase A)

## Objectives

Port EHRbase's ValidationWalker equivalent: validate an incoming COMPOSITION
(or other versioned object) against its WebTemplate, enforcing archetype
constraints (cardinality, existence, value-set/terminology bindings) and
producing the same class of validation errors EHRbase does.

## Preconditions

- [ ] Phase 03 done: RM classes exist to validate
- [ ] Phase 10 done: WebTemplate available as the constraint source
- [ ] Phase 02 done: terminology service available for value-set/binding checks

## Scope

In: the `Validate` trait (validation context + path + error accumulator,
mirroring Archie's validators), cardinality/existence checks, terminology
binding checks, RM invariant checks layered under `garde` for outer DTO
validation.
Out: AQL-time validation (Phase 12/13 own query validation), FLAT/STRUCTURED
input validation (Phase 16, though it reuses this phase's `Validate` trait).

## Tasks

- [ ] Define the `Validate` trait: takes a validation context, a path, and an error accumulator, mirroring Archie's validator design
- [ ] Implement cardinality validation (min/max occurrences) against WebTemplate node constraints
- [ ] Implement existence validation (required vs optional nodes) against WebTemplate node constraints
- [ ] Implement terminology binding validation: `CODE_PHRASE` values checked against the Phase 02 terminology service and archetype-declared value sets
- [ ] Implement RM invariant checks (e.g. `DV_INTERVAL` lower <= upper, `DV_QUANTITY` unit presence) as part of the same `Validate` trait
- [ ] Layer `garde` validation on the outer request DTOs (Phase 06) ahead of RM-level `Validate` invocation
- [ ] Write a test validating a conformant composition against its archetype succeeds
- [ ] Write a test validating a non-conformant composition (bad cardinality, unbound code) fails with the expected error path
- [ ] Add PORT STATUS trailers referencing EHRbase's ValidationWalker Java class as source

## Exit criteria

- [ ] A conformant composition validates successfully against its WebTemplate
- [ ] A non-conformant composition fails validation with an error naming the offending path
- [ ] Terminology binding checks correctly accept/reject codes against the bundled terminology

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. Keep the `Validate` trait error type generic enough that Phase
16's FLAT/STRUCTURED validation and Phase 06's DTO-level `garde` validation
can both report through the same accumulator shape without a second design
pass.
