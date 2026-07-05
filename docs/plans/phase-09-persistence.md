# Phase 09 — Persistence foundation

- Status: done (2026-07-05)
- Consumes: `openehr-rm` (types stored); the vendored EHRbase v2 schema
- Compile required: **yes** — builds compiling, tested (ADR-006 retires the "need not compile" slack for the app layer)
- Decisions: ADR-006 (sqlx + sea-query, not sea-orm; reuse the real schema)

## Objectives

Stand up the persistence layer the whole server sits on: a `sqlx` Postgres pool,
the **real EHRbase v2 schema** applied via `sqlx migrate`, `sea-query` table
definitions for programmatic query building, and a `testcontainers` PG 18 harness
that proves migrations apply cleanly.

## Preconditions

- [x] P00 done (workspace, `ehrbase` crate skeleton)
- [x] The 41 Flyway migrations are present — moved to `crates/ehrbase/tests/resources/legacy_schema/` as the equality-gate fixture (ADR-007)

## Scope

**In:** `sqlx` pool + config (`figment`/`config`, `DATABASE_URL`); run the
vendored migrations via `sqlx::migrate!` (reuse verbatim — do **not** re-author
DDL); `sea-query` `Iden` table/column definitions for the v2 tables (`ehr`,
`comp_version`/`_history`, `comp_data`/`_history`, `ehr_status_data`,
`ehr_folder_data`, `contribution`, `audit_details`, `template_store`,
`stored_query`, `item_tag`); `testcontainers` + `testcontainers-modules` PG 18
fixture; required extensions (`uuid-ossp`, `pgcrypto`, `pg_trgm`).
**Out:** the RM↔JSONB mapping (P10), any repository CRUD logic (P12), the AQL
SQL builder (P16 — uses these `sea-query` tables).

## Tasks

- [x] `sqlx` pool + settings loader in `crates/ehrbase/src/db/` — `settings.rs` (figment, `EHRBASE_DB_*`/`DATABASE_URL`) + `pool.rs` (search-path-initialized `PgPool`)
- [x] Wire the migrations with `sqlx::migrate!`; confirm they apply — **superseded per user decision + ADR-007:** the Flyway chain was squashed to one clean `0001_baseline.sql` per schema (created via `sqlx migrate add --sequential`); a schema-equality test proves the baseline ≡ the legacy chain end-state
- [x] `sea-query` `Iden` definitions for the v2 tables/columns — `iden.rs`, 17 final-state tables (post-V15/V25 column sets)
- [x] `testcontainers` PG 18 fixture that runs migrations — test-owned containers (removed on `Drop`), per-test databases (`tests/persistence.rs`)
- [x] Extensions enabled in the fixture setup — bootstrap in `db::run_migrations` (`uuid-ossp`, `pgcrypto`, `pg_trgm` into `ext`)

## Exit criteria

- [x] `cargo nextest run` brings up PG 18 and applies the baselines cleanly (idempotently); the equality gate replays all 40 legacy migrations and asserts fingerprint identity
- [x] A smoke test round-trips a row through `ehr` via `sea-query` + `sqlx` (uuid + jiff-sqlx timestamp decode)
- [x] Crate compiles + clippy-clean (0 warnings, workspace fmt clean; 8/8 tests pass)

## Decisions made this phase

- **ADR-007:** squash the vendored Flyway chain into one clean sqlx baseline per
  schema, derived from the chain's end state on PG 18 and verified by a
  pg_catalog fingerprint equality test; the original chain is preserved as the
  executable test fixture. Deviation: the orphaned `tenant_id_seq` is dropped.
- `sea-query-binder` is stuck on sea-query 0.32 — replaced by **`sea-query-sqlx`**
  (the official sea-query 1.0 ↔ sqlx 0.9 binder). `jiff-sqlx` added for
  jiff↔Postgres at plain-sqlx boundaries (the binder's with-jiff is
  unimplemented upstream).
- `sqlx` 0.9's `SqlSafeStr` gate: dynamic (sea-query-built) SQL passes through
  `sqlx::AssertSqlSafe`, matching the official sea-query example.

## Handoff for next session

P09 done: `ehrbase::db` provides settings/pool/migrations/idens; migrations are
two clean `0001_baseline.sql` files (ext then ehr, two `_sqlx_migrations`
trackers) with the legacy Flyway chain as an equality-gate fixture. Next: P10
rm-db-format — decompose/reconstruct between `openehr-rm` graphs and the
`comp_data` row model (`num`/`parent_num`/`num_cap`/`citem_num`, alias
compaction), reusing `openehr-its::json` canonical encoding. Golden vectors:
`crates/ehrbase/tests/resources/rm_db_format/…/*.db_aliased.json`.
