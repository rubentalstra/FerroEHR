---
paths: ["app/ferroehr/**"]
---

# sqlx + sea-query conventions (persistence + the AQL engine — both shipped)

`ferroehr` is the only crate that talks to PostgreSQL, using `sqlx` 0.9 (driver,
pool, migrations) + `sea-query` 1.0 + `sea-query-sqlx` (the dynamic SQL builder
+ binder; `sea-query-binder` is the obsolete sea-query-0.32 pairing — do not
use it). **Not sea-orm.** Target PostgreSQL 18.6+.

## Migrations

- The schema is **our own PG18-native design** (re-authored
  enterprise-grade): the unified `node` table, the temporal
  `vo_version` table, supporting tables, and our `ext` helper functions. It
  is live and CNF-pipeline-verified.
- **Greenfield migration policy (owner ruling 2026-08-20, issue #2452):**
  while the product is greenfield — no installation upgrades a database in
  place — the squashed baselines and existing migration files ARE edited in
  place; deployments recreate. Do NOT add checksum-reconciliation machinery,
  immutability guards, or upgrade-path repair for edited migrations, and do
  not re-file the in-place edits as a defect. sqlx's applied-migration
  immutability discipline arms only when the owner declares stabilization
  (a future explicit ruling), at which point migrations become append-only.
- Create migrations with the official CLI only:
  `sqlx migrate add --source app/ferroehr/migrations/<schema> --sequential <desc>`,
  written as modern PG 18 SQL (`uuidv7()`, temporal `WITHOUT OVERLAPS`,
  `RETURNING OLD/NEW` where the design calls for them).
- `ferroehr::db::run_migrations` bootstraps schemas + extensions and runs the
  `ext` migrator before `ehr`; each set keeps its own `_sqlx_migrations` table.
- `sea-query` `Iden` table/column definitions (`db/iden.rs`) + hand-written
  row-mapping structs (over the generated `openehr-rm` types) — no ORM/codegen.

## Queries

- Prefer `sqlx::query!`/`query_as!` (compile-time checked) wherever the SQL
  is static; drop to `sea-query` when the AQL engine needs to build SQL
  dynamically (ASL → SQL is inherently dynamic — see `aql-engine.md`).
- Use native PG 18 features where the plan calls for them: `uuidv7()` for
  generated IDs, `RETURNING OLD/NEW` for audit/history writes, temporal
  constraints where the schema models versioned rows, skip scan/JSON_TABLE
  where they simplify AQL-generated SQL.
- `sqlx` has **no `jiff` feature** — use the official `jiff-sqlx` wrapper
  types (`jiff_sqlx::Timestamp`, `.to_jiff()`) on plain sqlx queries. On
  sea-query-built queries the binder's `with-jiff` is unimplemented upstream —
  bind via SQL (`now()`) or a chrono value at the boundary; do not silently
  switch the crate to `chrono`.
- `rust_decimal` is the `BigDecimal` replacement for `DV_QUANTITY` and other
  fixed-point RM fields; use the `sqlx` `rust_decimal` feature, not `f64`.

## Transactions and service boundaries

- One `sqlx::Transaction` per service-level write (composition create/update,
  contribution commit, EHR-status change), matching the openEHR
  contribution/commit semantics (one CONTRIBUTION per change set — the spec
  is the authority).
- Every write emits an `audit_details` + `contribution` row in the same
  transaction — an openEHR requirement: the versioning / CONTRIBUTION /
  audit semantics are defined in `docs/specs/openehr/RM/docs/common/`
  (Change Control: VERSION, VERSIONED_OBJECT, CONTRIBUTION, AUDIT_DETAILS)
  and `docs/specs/openehr/RM/docs/ehr/`; implement against that text
  (spec-adherence.md), with EHRbase as prior art only.

## Testing

- Integration tests get a real PostgreSQL 18 database from the shared
  harness (`testkit::db()`, `tools/testkit` — one server, template-clone per
  test); the template build verifies the vendored migrations apply cleanly.
  See `testing.md` for the full test discipline.

This file adds persistence-specific rules on top of `rust-style.md` (idiomatic
app code).
