# Phase 10 — Storage foundation (greenfield node model, ADR-008)

> Re-scoped 2026-07-05 by ADR-008 (greenfield pivot). The original content of
> this file (porting EHRbase's rm-db-format) is superseded; that port is
> archived unmerged on `claude/phase-10-rm-db-format`.

- Status: done (2026-07-05)
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

- [x] ADR-008 merged (this phase implements it)
- [x] P09 infrastructure available (pool, migrator runner, testcontainers)

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
- [x] Final schema migrations (`ehr/0001_schema.sql`: node per-version PK,
      temporal vo_version, supporting tables; `ext/0001_openehr_functions.sql`:
      full spec-formula `openehr_magnitude` + ISO-8601 helpers) + `Iden` defs
      rewritten + persistence tests rebuilt (legacy fixture/gate removed)
- [x] Node codec: decompose — `src/storage/codec.rs` (nested-set numbering,
      citem tracking, readable COLLATE-C paths, canonical fragments)
- [x] Node codec: reassemble — lossless inverse, order-independent input
- [x] Corpus round-trip tests — all 52 corpus compositions in memory
      (`tests/codec_corpus.rs`) + full DB round-trip of the IPS composition
      incl. a CONTAINS interval-join check (`tests/persistence.rs`).
      EHR_STATUS/FOLDER-specific cases follow with their service flows (P12)

## Exit criteria

- [x] Spike results recorded; schema decisions closed with data
- [x] Corpus decomposes + reassembles losslessly (nextest, testcontainers)
- [x] `cargo nextest run -p ehrbase` green (15/15); crate clippy-clean (0 warnings)

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

## Handoff for next session

P10 done: the greenfield schema is live (`migrations/{ext,ehr}/0001_*`,
bootstrap = schemas + btree_gist), `ehrbase::storage` provides the lossless
node codec (`decompose`/`reassemble`/`NodeRow`), `db::iden` matches the new
tables, and the spike harness (`tests/storage_spike.rs`, ignored) remains as
the measurement tool. Next: **P11 REST server + auth** — axum app
implementing the generated ITS-REST traits (`openehr-its/src/rest/generated/`,
5 traits + ROUTES tables), tower-http stack, content negotiation, Basic +
OAuth2/OIDC. Verify `oauth2`/`openidconnect`/`tower-sessions`/`axum-login`
pins docs-first before first use.
