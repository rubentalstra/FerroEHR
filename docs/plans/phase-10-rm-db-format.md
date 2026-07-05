# Phase 10 — Storage foundation (greenfield node model, ADR-008)

> Re-scoped 2026-07-05 by ADR-008 (greenfield pivot). The original content of
> this file (porting EHRbase's rm-db-format) is superseded; that port is
> archived unmerged on `claude/phase-10-rm-db-format`.

- Status: not-started (Stage-1 app build, step 2 of 13)
- Consumes: `openehr-rm` + `openehr-its` (canonical JSON), P09 infrastructure
  (pool/migrators/testcontainers)
- Compile required: yes (compiling, tested increment)
- Decisions: ADR-008 (own PG18-native storage; spec conformance target)

## Objectives

Design-validate and build our own storage layer: the unified `node` table
(nested-set decomposition, canonical JSON fragments, promoted predicate
columns), the temporal `vo_version` table (`WITHOUT OVERLAPS`), supporting
tables (`ehr`, `contribution`, `audit`, `template_store`, `stored_query`,
`item_tag`), our `ext` helper functions (`openehr_magnitude` et al.), and the
Rust node codec: canonical composition ⇄ node rows, losslessly.

## Preconditions

- [ ] ADR-008 merged (this phase implements it)
- [ ] P09 infrastructure available (pool, migrator runner, testcontainers)

## Scope

**In:** the storage spike (corpus → candidate schema in a testcontainer;
representative CONTAINS/extract/order queries; temporal-PK validation;
fragment-size measurement — results recorded in this file); the final schema
as fresh `sqlx migrate add` migrations (replacing the ADR-007 baseline
content); sea-query `Iden` defs; the node codec (decompose: canonical JSON →
rows with `num`/`num_cap`/`parent_num`/`citem_num`/`path`; reassemble: rows →
canonical JSON — no aliasing, no synthetic fields); corpus round-trip tests.
**Out:** repository CRUD/versioning orchestration (P12), the AQL engine (P16),
REST wiring (P11).

## Tasks

- [ ] Storage spike: corpus-loaded candidate schema + query/bench validation
      (decides temporal PK vs fallback, index set, fragment format)
- [ ] Final schema migrations (`ehr` schema re-authored; `ext` = our helper
      functions) + updated `Iden` defs + testcontainers gate updated
- [ ] Node codec: decompose (canonical JSON → node rows) with nested-set
      numbering and path materialization
- [ ] Node codec: reassemble (rows → canonical JSON), lossless
- [ ] Corpus round-trip property/golden tests (48-composition corpus +
      EHR_STATUS + FOLDER cases)

## Exit criteria

- [ ] Spike results recorded; schema decisions closed with data
- [ ] Corpus decomposes + reassembles losslessly (nextest, testcontainers)
- [ ] `cargo nextest run -p ehrbase` green; crate clippy-clean

## Decisions made this phase

- (record spike outcomes + any deviations from ADR-008 here)
