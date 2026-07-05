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

- [x] Storage spike: corpus-loaded candidate schema + query/bench validation
      — `tests/storage_spike.rs`, results below (2026-07-05)
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

### Spike results (2026-07-05 — PG 18 testcontainer, 52-composition corpus ×100)

| Measurement | Result |
|---|---|
| Load, fine granularity | 149,700 rows / 5,200 comps in ~10 s; table 71 MB + indexes 17 MB; avg fragment **359 B** (never TOASTed) |
| Load, coarse (no ELEMENT/FEEDER_AUDIT rows) | 65,600 rows; 49 MB + 12 MB; avg fragment 681 B |
| CONTAINS scoped to one versioned object (hot path) | **0.44 ms** (nested loop over the `(vo_id, num)` PK) |
| CONTAINS corpus-wide (archetype-filtered) | ~42 ms for 6,300 matches (bitmap on `(rm_type, archetype)`) |
| Leaf extraction (`jsonb_path_query_first`, 8,000 rows) | ~21 ms |
| Magnitude ORDER BY, no index | ~14 ms (parallel sort) |
| Magnitude ORDER BY, expression index | **1.1 ms** — Index Scan on `openehr_magnitude(...)` partial btree (`DESC NULLS LAST` spelled in the index) |
| GIN `jsonb_ops` `$.**` equality anchor, full count | **7 ms**, Bitmap Heap Scan on the GIN index — recursive-descent anchors are index-served as researched |
| Temporal PK `WITHOUT OVERLAPS` (needs `btree_gist`) | Overlap rejected ✓; `upper_inf` partial index serves LATEST ✓ |
| Codec round-trip | Lossless on all 52 corpus compositions, both granularities |

### Decisions closed by the spike

- **Fine granularity** (every structure type incl. `ELEMENT` gets a row): the
  ~30 % storage premium buys direct rows for every AQL-containable type,
  ELEMENT-level promoted columns, and 359 B fragments (no TOAST). Coarse
  rejected.
- **Temporal `vo_version` model confirmed** (PG 18 `WITHOUT OVERLAPS` +
  `btree_gist`); no fallback needed.
- **`openehr_magnitude` as `IMMUTABLE` ext function + on-demand expression
  indexes** confirmed (13.6 ms → 1.1 ms); no synthetic stored fields. Index
  recipe: partial predicate matching the query + `DESC NULLS LAST` in the
  index definition.
- **GIN `jsonb_ops` on `node.data`** kept (serves `$.**` equality anchors);
  never `jsonb_path_ops`.
- **Node rows are stored per version** (`PK (vo_id, sys_version, num)`) so
  `ALL_VERSIONS` queries the same table uniformly — an improvement over the
  current/history split (updates are rare in clinical data; storage cost
  acceptable).
