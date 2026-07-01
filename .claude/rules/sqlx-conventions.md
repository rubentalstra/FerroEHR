---
paths: ["crates/openehr-server/**"]
---

# sqlx + sea-query conventions

`openehr-server` is the only crate that talks to PostgreSQL. It replaces
EHRbase's jOOQ + Flyway persistence layer with `sqlx` 0.9 (driver, pool,
migrations) and `sea-query` 0.32 + `sea-query-binder` 0.7 (the jOOQ-DSL
analogue for the AQL→SQL engine). Target PostgreSQL 18.3+.

## Migrations

- `crates/openehr-server/migrations/` holds the Flyway SQL copied **verbatim**
  from EHRbase's `jooq-pg` module in the Phase 0 `git mv`. Do not edit a
  migration that has already shipped in a prior phase — append a new
  migration file instead, exactly as Flyway/EHRbase would have.
  `sqlx migrate` numbering conventions apply to any brand-new migration this
  port adds beyond the copied set.
- Generated jOOQ code itself is discarded, not ported — `sea-query` +
  hand-written row-mapping structs replace it.

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

- Transaction boundaries mirror the Java service layer's boundaries
  (`service` module in EHRbase): one `sqlx::Transaction` per service-level
  write operation (composition create/update, contribution commit, EHR
  status change), matching what the Java transaction demarcation covered —
  do not merge or split transactions relative to the source without a
  `// PORT NOTE:`.
- Every write that the Java layer paired with an `audit_details` +
  `contribution` insert must do the same here, in the same transaction.

## Testing

- `testcontainers` + `testcontainers-modules` run a real PostgreSQL 18 for
  integration tests; verify migrations apply cleanly as part of that setup.
  See `testing.md` for the full test discipline.

Every file in this crate still needs the PORT STATUS trailer and annotation
vocabulary from `rust-style.md`; this file only adds persistence-specific
rules on top.
