# Phase 14 — WebTemplate builder

- Status: in-progress (PR-A: WebTemplate builder + wt+json endpoint + full-corpus gate)
- Consumes: `openehr-am` (OPT model, P13), `openehr-term`
- Compile required: yes (compiling, tested increment)
- Decisions: ADR-006

## Objectives

Build the **WebTemplate** (the flattened, UI/validation-oriented view of an OPT)
from an ingested operational template — EHRbase's `OptVisitor` equivalent. The
WebTemplate drives composition validation (P15), the FLAT/STRUCTURED formats
(P17), and AQL semantic path analysis (P16). Cache built WebTemplates with
`moka`.

## Preconditions

- [x] P13 (OPT ingested — consumed via `openehr_its::opt14`)

## Scope

**In:** OPT → WebTemplate node tree (paths, rm types, cardinalities, terminology
bindings, inputs); `moka` cache keyed by template id; WebTemplate JSON export
(`application/openehr.wt+json`); Better `web-template` semantics as the oracle.
**Out:** validation logic (P15); FLAT/STRUCTURED conversion (P17).

## Tasks

- [x] WebTemplate model + OPT→WebTemplate builder — `openehr-flat::webtemplate` (model/id/inputs/builder); Better `web-template` shape (ids, aqlPath, inputs, compaction, post-processing) as the oracle.
- [x] `moka` cache — `WebTemplateCache` (`cache.rs`), wired into `ehrbase-rest` `AppState`.
- [x] WebTemplate JSON export endpoint — `GET /definition/template/adl1.4/{id}` serves `application/openehr.wt+json` on `Accept` (else the OPT XML).
- [x] Tests vs Better `web-template-tests` vectors — full 63-file Better set vendored + 91-file service corpus as a smoke gate (145 build, 0 builder failures); insta goldens + targeted assertions.

## Exit criteria

- [x] Reference OPTs produce WebTemplates matching Better's format (insta goldens for Demo Vitals / Diagnosis / medication_list; Better ships no stored WT vectors, so goldens are self-generated and format-matched, and the full-corpus gate proves the builder).
- [x] Cache hit path verified — `cache::tests::builds_once_then_serves_from_cache`.
- [x] Compiles + clippy-clean — `openehr-flat` + `ehrbase-rest` build, clippy-clean, `cargo fmt` clean.

## PR-A scope boundaries (recorded as `TODO(port)`)

- Required-RM-attribute injection (needs the BMM RM attribute model, P16),
  `ISM_TRANSITION`/careflow synthesis, "any"-element expansion, archetype
  internal-ref resolution, and node-level term bindings are deferred.
- 9 Better templates cannot be exercised yet: `openehr_its::opt14` (P13's
  generated parser) rejects them (`missing element node_id/occurrences/purpose`)
  — a P13 parser-strictness follow-up, not a WebTemplate-builder gap.

## Decisions made this phase

- Better `web-template` semantics are the oracle; EHRbase quirks behind the
  `ehrbase-quirks` flag on `openehr-flat` (P17).
