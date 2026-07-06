# Phase 16 — AQL engine (our typed IR over the node model, ADR-008)

> Re-scoped 2026-07-05 by ADR-008: our own design (no ASL port); EHRbase's
> engine is prior art only.

- Status: not-started (Stage-1 app build, step 8 of 13)
- Consumes: `openehr-query` (AST, done), P10 (node model), P14 (WebTemplate),
  the BMM-generated RM attribute model
- Compile required: yes (compiling, tested increment)
- Decisions: ADR-008

## Objectives

Execute AQL 1.1: semantic/path analysis over a **BMM-generated RM attribute
model** (no reflection, no hand-maintained tables — `openehr-codegen` /
`openehr-lang` emit it), lowering into **our own typed query IR**, SQL
generation via `sea-query` against the P10 node model (nested-set interval
joins for CONTAINS, `jsonb_path_query_first` + jsonpath item methods +
`openehr_magnitude` for typed leaf comparison/ordering, `JSON_TABLE` for
array unnesting, GIN `jsonb_ops` `$.**` equality anchors as pre-filters),
execution via `sqlx`, and `RESULT_SET` (ITS-REST 1.0.3) assembly.

## Preconditions

- [ ] P10 (node model), P14 (WebTemplate), RM attribute model generated

## Scope

**In:** the RM attribute-model codegen target; path analysis + typing
(candidate types, multi-valued detection, abstract→concrete expansion); the
IR + AST→IR lowering; IR→SQL incl. versioning semantics (`LATEST_VERSION`
partial-index path, `ALL_VERSIONS` — in scope, ADR-008); parameter binding;
the feature envelope (accept/reject documented per construct); `/query/aql` +
stored queries wired through P11/P12. **Out:** the parser (P07, done); query
caching / perf tuning (P20).

## Tasks

- [x] `emit-rm-model` (or `openehr-lang` API): attribute→types, multiplicity,
      descendant sets, structure classification — generated from BMM
      — new `openehr-codegen` target emits `openehr-rm::model` (static
      `RmClass`/`RmAttribute`/`Container` + `class`/`attribute`/`attributes`/
      `descendants`/`ancestors`/`is_a`/`is_structure_root`) from BASE+RM BMM;
      `is_structure_root` mirrors `ehrbase::storage::codec::STRUCTURE_TYPES`;
      wired into `emit`, the drift script, and `/regen-codegen`.
- [ ] Path analysis + typing against the generated model
- [ ] Query IR + AST→IR lowering (incl. CONTAINS trees, version addressing)
- [ ] IR→SQL via sea-query (interval joins, typed extraction, magnitude,
      unnesting; LATEST/ALL_VERSIONS)
- [ ] Execute + `RESULT_SET` assembly; QUERY endpoints live
- [ ] AQL corpus end-to-end tests + the feature-envelope matrix

## Exit criteria

- [ ] The AQL conformance corpus executes correctly over real data
      (testcontainers), including version addressing
- [ ] Feature envelope documented; every rejection is an explicit typed error
- [ ] Compiles + clippy-clean + nextest green

## Decisions made this phase

- (record IR shape + envelope decisions here)
