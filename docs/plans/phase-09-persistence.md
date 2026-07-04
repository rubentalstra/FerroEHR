# Phase 09 — Persistence foundation

- Status: not-started — **NEXT** (Stage-1 app build, step 1 of 13)
- Consumes: `openehr-rm` (types stored); the vendored EHRbase v2 schema
- Compile required: **yes** — builds compiling, tested (ADR-006 retires the "need not compile" slack for the app layer)
- Decisions: ADR-006 (sqlx + sea-query, not sea-orm; reuse the real schema)

## Objectives

Stand up the persistence layer the whole server sits on: a `sqlx` Postgres pool,
the **real EHRbase v2 schema** applied via `sqlx migrate`, `sea-query` table
definitions for programmatic query building, and a `testcontainers` PG 18 harness
that proves migrations apply cleanly.

## Preconditions

- [ ] P00 done (workspace, `ehrbase` crate skeleton)
- [ ] The 41 Flyway migrations are present in `crates/ehrbase/migrations/{ehr,ext}/`

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

- [ ] `sqlx` pool + settings loader in `crates/ehrbase/src/db/`
- [ ] Wire the vendored migrations with `sqlx::migrate!`; confirm they apply
- [ ] `sea-query` `Iden` definitions for the v2 tables/columns
- [ ] `testcontainers` PG 18 fixture (`sqlx::test`-style) that runs migrations
- [ ] Extensions enabled in the fixture setup

## Exit criteria

- [ ] `cargo nextest run` brings up PG 18, applies all 41 migrations cleanly
- [ ] A smoke test round-trips a row through one v2 table via `sea-query` + `sqlx`
- [ ] Crate compiles + clippy-clean

## Decisions made this phase

- Reuse the vendored EHRbase v2 Flyway SQL verbatim (ADR-006); `sqlx` for
  exec/migrate/pool + `sea-query` as the builder.
