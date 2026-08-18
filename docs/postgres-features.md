# PostgreSQL 17 + 18 features FerroEHR leverages

**Pin: PostgreSQL 18, target 18.6+** (`docs/VERSIONS.md`; CI runs `postgres:18.6`).
Upstream EHRbase targets **PG 15/16**; we target **18** to exploit two major
releases of new capability for a JSONB-heavy openEHR CDR. This file is the
reference the persistence, AQL-engine, and auth subsystems build against —
"the best possible system" means using these, not the PG-16 subset.

## Versioning note (why the feature list is only 17.0 + 18.0)

PostgreSQL adds features **only in major releases**. Every minor release —
**18.1 through 18.6** (18.5 was never released) and all of **17.x** — is a
cumulative **bugfix + security** rollup with **no new SQL features** (18.6,
2026-08-13, fixes 28 CVEs; same-major upgrades need no dump/restore). So the
feature delta over EHRbase's PG 16 is exactly the **PG 17.0** and **PG 18.0**
feature sets below; we run the latest patch (18.6) for the fixes.

## PG 17.0 — SQL/JSON + query performance

| Feature | What it enables for FerroEHR |
|---|---|
| **`JSON_TABLE()`** | Project JSONB clinical documents into relational rows in SQL — a core tool for the AQL→SQL generator, removing app-side JSON walking. |
| **SQL/JSON query fns** — `JSON_EXISTS`, `JSON_QUERY`, `JSON_VALUE` | Standards-based JSONB path extraction/validation in generated AQL SQL. |
| **SQL/JSON constructors** — `JSON()`, `JSON_SCALAR()`, `JSON_SERIALIZE()` | Build/normalize JSON in-query where needed. |
| **`jsonpath` type methods** — `.integer()/.boolean()/.date()/.timestamp()…` | Type-safe coercion inside path queries — precise AQL value extraction. |
| **`MERGE … RETURNING` + `merge_action()`** | Upsert composition/version/status rows and report INSERT/UPDATE/DELETE — useful for versioned writes. |
| **`MERGE … WHEN NOT MATCHED BY SOURCE`** | Reconcile/soft-delete rows absent from an incoming set. |
| **Optimizer: `IN`/`NOT IN`, correlated subqueries, B-tree `IN` batches** | Faster AQL predicate + code/identifier lookups. |
| **Incremental backup** (`pg_basebackup --incremental`) | Ops concern for large CDRs (not app code). |

## PG 18.0 — async I/O, temporal, identifiers, generated columns, auth

| Feature | What it enables for FerroEHR |
|---|---|
| **`uuidv7()` (native)** | Timestamp-ordered UUIDs for `OBJECT_VERSION_ID`/row keys — index-friendly, no `uuid` crate round-trip for DB-generated ids. |
| **Temporal `PRIMARY KEY`/`UNIQUE`/`FOREIGN KEY` `WITHOUT OVERLAPS`** | Enforce non-overlapping validity on the one temporal `vo_version` table (the greenfield storage design — `sys_period tstzrange`, no current/`_history` pairs; see `docs/architecture.md` §Storage) at the DB — a natural fit for openEHR versioning. |
| **`RETURNING OLD/NEW`** in INSERT/UPDATE/DELETE/MERGE | One-statement audit capture (write + return prior value) for the `audit`/`contribution` rows on every version write. |
| **Virtual generated columns** | Cheap read-time derived columns (e.g. a JSONB leaf) without storage — candidate indexes/filters for AQL hot paths. |
| **B-tree skip scan** | Multicolumn indexes usable when a leading column is unconstrained — fewer indexes for the row-per-locatable + AQL access patterns. |
| **Asynchronous I/O (AIO)** (`io_method`, `io_combine_limit`) | Faster seq/bitmap scans + vacuum — throughput for large-corpus AQL tuning. |
| **OAuth authentication** (`oauth` in `pg_hba.conf`, `oauth_validator_libraries`) | DB-level OAuth option; complements our app-level OAuth2/OIDC for federated identity. |
| **Self-join elimination** (`enable_self_join_elimination`) | AQL SQL that self-joins the same locatable table can be simplified by the planner. |
| **`OR` → `= ANY(array)` transformation** | AQL `OR`/`MATCHES` predicate lists become index-friendly array lookups automatically. |
| **`jsonb` null → SQL scalar `NULL` cast** | Simpler optional-field extraction in generated SQL (no error on JSON null). |
| **Partition planner improvements** | If time-partitioning `vo_version`/`node` by time is adopted, cheaper planning across partitions. |

## Feature → subsystem mapping (where to use each)

- **Persistence / service layer:** `uuidv7()`, temporal `WITHOUT OVERLAPS`
  constraints, `RETURNING OLD/NEW`, `MERGE … RETURNING` (versioning + audit).
- **AQL engine:** `JSON_TABLE`, `JSON_QUERY`/`JSON_VALUE`/`JSON_EXISTS`,
  `jsonpath` type methods, skip scan, `OR`→`ANY`, self-join elimination,
  virtual generated columns for hot filters. (`sea-query` emits these; see
  `.claude/rules/aql-engine.md`.)
- **Auth:** app-level OAuth2/OIDC (crates) is primary; DB `oauth` is available.
- **Optimization:** AIO tuning, `JSON_TABLE` codegen, generated-column/skip-scan
  indexes — profile-first, and only while conformance stays green.

**Discipline:** use PG 18 features where they simplify or speed the SQL, but a
feature that is *only* a perf win (not needed for correctness/conformance) is a
`// TODO(#NNNN):` on its optimization issue — never trade away REST/AQL
conformance for it.
