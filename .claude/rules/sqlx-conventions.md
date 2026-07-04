---
paths: ["crates/ehrbase/**"]
---

# sqlx + sea-query conventions (P09 persistence, P16 AQL)

`ehrbase` is the only crate that talks to PostgreSQL, using `sqlx` 0.9 (driver,
pool, migrations) + `sea-query` 1.0 + `sea-query-binder` (the dynamic SQL builder
for the AQL→SQL engine). **Not sea-orm** (ADR-006). Target PostgreSQL 18.4+.

## Migrations

- `crates/ehrbase/migrations/` holds the **real EHRbase v2 Flyway SQL** (41
  files, vendored verbatim). Run them via `sqlx migrate` — this **is** the schema;
  do not re-author DDL. Append a new migration only for a genuinely new need,
  following `sqlx migrate` numbering.
- No jOOQ codegen — `sea-query` `Iden` table/column definitions + hand-written
  row-mapping structs (over the generated `openehr-rm` types) replace it.

## Queries

- Prefer `sqlx::query!`/`query_as!` (compile-time checked) wherever the SQL
  is static; drop to `sea-query` when the AQL engine needs to build SQL
  dynamically (ASL → SQL is inherently dynamic — see `aql-engine.md`).
- Use native PG 18 features where the plan calls for them: `uuidv7()` for
  generated IDs, `RETURNING OLD/NEW` for audit/history writes, temporal
  constraints where the schema models versioned rows, skip scan/JSON_TABLE
  where they simplify AQL-generated SQL.
- `sqlx` has **no `jiff` feature** — bridge `jiff` timestamps to
  `sqlx`'s `chrono`/native types manually at the query boundary; do not
  silently switch the whole crate to `chrono`.
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
