# B7 schema review — 02: PostgreSQL 18 enterprise best practices (2026-07-10)

Web-sourced research (Opus fan-out, B7 task 1c); recommendations with sources
+ applicability to our node/vo_version design. Feeds ADR-013. Sources are in
the B7 research transcript; official PG docs corroborate the feature claims,
blog benchmarks are directional.

## 1. Organization
- **1.1** Keep the `ehr`/`ext` schema split — justified as a GRANT/ownership
  boundary (app writes `ehr.*`; `ext` functions on the read path only).
- **1.2** snake_case identifiers (already our style); RM UPPER_SNAKE only as
  *values*.
- **1.3** Deterministic constraint/index names (`idx_/uq_/fk_/ck_` +
  table+cols) — never rely on PG auto-names (our `kind` CHECK was
  dropped/re-added twice by auto-name; fragile).
- **1.4** `COMMENT ON` every table/non-obvious column/function in the creating
  migration — high value for our deliberately non-obvious encodings
  (`num/num_cap`, `sys_period`, verbatim `data`).

## 2. Integrity
- **2.1** Keep FKs; index every FK child column; watch wide cascades from
  `ehr` (prefer explicit ordered deletes in admin paths).
- **2.2** CHECK > trigger > app for SQL-expressible invariants: add
  `ck_node_nested_set (num_cap >= num)`, `parent_num < num`; keep
  contribution+audit atomicity in the service transaction (not triggers).
- **2.3** `DEFERRABLE INITIALLY IMMEDIATE` on FKs in multi-row version commits
  (`node → vo_version`), deferred per-transaction only where needed.
- **2.4** ⚠ Temporal PK `WITHOUT OVERLAPS` = GiST EXCLUDE under the hood:
  production-usable but first-release; keep the plain
  `UNIQUE (vo_id, sys_version)` btree (also needed as FK target + replica
  identity, see 6.3); keep the `upper_inf` partial btree for LATEST_VERSION
  (GiST doesn't serve it).

## 3. Security (PHI)
- **3.1** Four-role architecture: owner / `migrator` (DDL) / `app_writer`
  (DML) / `app_reader` (SELECT) — never superuser at runtime.
- **3.2** `ALTER DEFAULT PRIVILEGES` so future migrations' tables are
  reachable without manual grants (deploy-outage classic).
- **3.3** RLS: skip in single-tenant Stage-1 (matches CLAUDE.md), but keep
  `ehr_id` placement RLS-ready (it already is on node/vo_version); adopt at
  Stage-2 multi-tenancy with `FORCE ROW LEVEL SECURITY`.
- **3.4** At-rest encryption at the volume/disk layer (PG has no native TDE);
  do NOT pgcrypto-encrypt `node.data` (kills AQL jsonpath + zero-translation).
- **3.5** pgaudit as DB-layer complement to our app-level openEHR audit +
  ATNA system_log: `ddl, role, connection` global + object-audit on the PHI
  tables only; ship to immutable store; ~6y retention.
- **3.6** TLS (`hostssl`), pin `search_path` on any SECURITY DEFINER, revoke
  `CREATE ON SCHEMA public FROM PUBLIC`; `ext` functions plain (not definer),
  owned by a role the app can't write.

## 4. Large-table operability
- **4.1** Do NOT partition yet (~50–100 GB threshold); design keys so
  RANGE-by-commit-time partitioning is possible later (P20).
- **4.2** ⚠ Partition keys must be in every PK/unique — interacts awkwardly
  with `WITHOUT OVERLAPS`; validate before combining (another argument for
  the btree-unique fallback).
- **4.3** fillfactor: `node` append-only → keep 100; `vo_version` gets one
  close-out UPDATE per supersession → fillfactor ~90 (note: the temporal PK
  indexes sys_period so the update can't be HOT anyway — measure).
- **4.4** `default_toast_compression = lz4` (validates ADR-008); fragments
  should mostly stay sub-TOAST — confirm at spike.
- **4.5** TOAST write amplification: keep the design append-only; never
  hot-update a row carrying large TOASTed `data`.
- **4.6** Aggressive autovacuum on `vo_version` (scale_factor ~0.02, raised
  cost limit); periodic `pgstattuple` + `REINDEX CONCURRENTLY` for the GiST
  temporal index (bloats faster than btree).

## 5. Migration hygiene (sqlx)
- **5.1** Squash the greenfield baseline to one `0001` per schema NOW (nothing
  deployed); append-only forever after; never edit applied migrations.
- **5.2** Lock-safe DDL for live systems: `CREATE INDEX CONCURRENTLY`;
  constraints as `NOT VALID` + later `VALIDATE`.
- **5.3** `lock_timeout` (≈5s) + bounded `statement_timeout` in the migration
  runner wrapper (not per-file).
- **5.4** sqlx: concurrent-index migrations must be isolated +
  non-transactional — ⚠ verify the sqlx 0.9 directive spelling before first
  use.
- **5.5** Idempotency for repeatable objects only (`CREATE OR REPLACE
  FUNCTION`, `CREATE EXTENSION IF NOT EXISTS`); never blind `IF NOT EXISTS`
  on structural DDL.

## 6. Backup / retention / PITR
- **6.1** WAL archiving + PITR from day one (pgBackRest / managed PITR).
- **6.2** UNLOGGED forbidden on all clinical/audit tables (crash-truncated,
  invisible to replicas/PITR).
- **6.3** ⚠ Replica identity: the temporal PK (GiST) may not satisfy logical
  replication — the explicit btree `UNIQUE (vo_id, sys_version)` should be
  the replica identity; every table needs a PK.

## 7. PG 18 specifics
- **7.1** `uuidv7()` confirmed best practice (~26% smaller PK indexes, ~3×
  ordered scans vs v4). ⚠ v7 leaks creation-time — a non-issue for openEHR
  (version timestamps are API-visible anyway); conscious decision recorded.
- **7.2** Temporal PK: adopt-with-spike discipline; known gaps (no temporal
  FK cascade, GiST cost, partition/replication interactions).
- **7.3** VIRTUAL generated columns (PG18 default) are NOT indexable and
  cannot call user-defined functions — for hot AQL leaf ordering use btree
  **expression indexes over `ext.openehr_magnitude(data)`** (ADR-008's path);
  STORED only when the value must also project.
- **7.4** Skip scan reduces index count but favors low-cardinality leading
  columns; validate with EXPLAIN, don't assume.

## Disagreement flags (both-ways arguments recorded)
temporal PK vs plain-unique fallback; RLS now-vs-Stage-2; uuidv7 timing leak;
FKs at extreme write volume (keep them for a CDR).
