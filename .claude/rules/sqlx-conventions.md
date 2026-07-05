---
paths: ["crates/ehrbase/**"]
---

# sqlx + sea-query conventions (P09 persistence, P16 AQL)

`ehrbase` is the only crate that talks to PostgreSQL, using `sqlx` 0.9 (driver,
pool, migrations) + `sea-query` 1.0 + `sea-query-sqlx` (the dynamic SQL builder
+ binder; `sea-query-binder` is the obsolete sea-query-0.32 pairing — do not
use it). **Not sea-orm** (ADR-006). Target PostgreSQL 18.4+.

## Migrations (ADR-007)

- `crates/ehrbase/migrations/{ext,ehr}/0001_baseline.sql` are **squashed
  baselines** of the EHRbase v2 Flyway chain — provably identical to its end
  state via the schema-equality test in `crates/ehrbase/tests/persistence.rs`
  (the original chain is the executable fixture under
  `crates/ehrbase/tests/resources/legacy_schema/`). Never edit an applied
  baseline; never weaken the equality gate.
- Create new migrations with the official CLI only:
  `sqlx migrate add --source crates/ehrbase/migrations/<schema> --sequential <desc>`,
  written as modern PG 18 SQL. When upstream EHRbase ships a new Flyway
  migration, translate it into our next migration and extend the fixture + gate.
- `ehrbase::db::run_migrations` bootstraps schemas + extensions and runs the
  `ext` migrator before `ehr` (the `ehr` DDL uses `ext`'s collation); each set
  keeps its own `_sqlx_migrations` table.
- No jOOQ codegen — `sea-query` `Iden` table/column definitions (`db/iden.rs`) +
  hand-written row-mapping structs (over the generated `openehr-rm` types)
  replace it.

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
  contribution commit, EHR-status change), matching **EHRbase's transaction
  semantics** (its `service` module is the behavioural reference — match what it
  makes atomic, idiomatically).
- Every write emits an `audit_details` + `contribution` row in the same
  transaction, as EHRbase does.

## Testing

- `testcontainers` + `testcontainers-modules` run a real PostgreSQL 18 for
  integration tests; verify the vendored migrations apply cleanly as part of
  that setup. See `testing.md` for the full test discipline.

This file adds persistence-specific rules on top of `rust-style.md` (idiomatic
app code, ADR-006) — no PORT STATUS trailer.
