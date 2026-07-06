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

- [x] Validation walker (composition vs WebTemplate) with path-tagged errors —
      `openehr-flat::validation` (RM-invariant pass + terminology pass +
      WebTemplate archetype-conformance walk; occurrences/existence/cardinality/
      type/domain-leaf; collect-all `Vec<ValidationMessage{path,message,kind}>`).
- [x] RM invariant + terminology checks — RM class invariants as ADR-003
      `*_impl.rs` (43 impls + `_type`→`Validate` dispatcher; archie-faithful),
      enabled by new codegen `*_impl.rs`-preservation; openEHR-terminology-group
      validation via the new `openehr-term::bundle` loader.
- [x] Hook into P12 composition create/update; 422 mapping — validated before
      persist (gated on a declared `archetype_details/template_id`; RM-optional
      absent template → skip); `ApiError::ValidationFailed` → ITS-REST **422**
      body `{message, validationErrors[]}`; 400 kept for parse/convert.
- [x] Tests: valid + invalid compositions (SDK corpus pairs + mutations) — unit
      + PG18 e2e (valid→201, invalid→422+paths, unknown-template→422).

## Exit criteria

- [x] Valid compositions accepted; invalid ones rejected with correct 422 + paths
- [x] Terminology-bound codes validated against the bundle (openEHR groups:
      category/setting/current_state/null_flavour/participation/math_function)
- [x] Compiles + clippy-clean (workspace; drift green)

## Deferred follow-ups (spec-audit findings, tracked — NOT stubs)

A `spec-conformance-reviewer` audit (against `docs/specs/openehr/`) confirmed the
subsystem is largely spec-faithful; these ranked follow-ups remain (each spec-cited):
- **F2** — wire the remaining ~10 terminology-bound invariants the RM defers to the
  validator: `COMPOSITION.territory`/`.language`, `ENTRY`/`DV_TEXT.language`/`.encoding`,
  `ISM_TRANSITION.transition`, `AUDIT_DETAILS`/`ATTESTATION.change_type`,
  `PARTY_RELATED.relationship`, `TERM_MAPPING.purpose`, `DV_ORDERED.normal_status`,
  `DV_MULTIMEDIA.media_type`. The `openehr-term::bundle` validators already exist; the
  terminology pass just needs to call them.
- **F3** — add `DV_TEXT.Valid_value` (non-empty), `ITEM_LIST.Valid_structure`,
  `DV_PARAGRAPH`/`DV_PARSABLE` invariants (spec > archie).
- **F5** — enforce the spec "not-empty-if-present" invariants archie ignores
  (`COMPOSITION.Content_valid`, `SECTION.Items_valid`, `INSTRUCTION.Activities_valid`,
  `CLUSTER` items).
- **F6** — invariant-name corrections (`Is_archetypeRoot`→`Is_archetype_root`,
  `Accuracy_valid`→`Accuracy_validity`, VERSION_TREE_ID split + `branch≥1`).
- Also: CONTRIBUTION-path compositions bypass `create_composition` (not yet
  validated); DV_INTERVAL/DV_ORDERED magnitude-consistency awaits P16
  `openehr_magnitude`. All recorded here so they're addressed, not forgotten.

## Decisions made this phase

- Genuinely spec-underdetermined checks follow ADR-003 (`todo!` with a cited
  reason, never invented behaviour).
