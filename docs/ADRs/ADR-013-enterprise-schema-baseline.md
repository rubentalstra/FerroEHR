# ADR-013: Enterprise-grade schema baseline (B7 redesign)

- **Status:** accepted (owner-confirmed 2026-07-10; the four structural
  choices below were explicitly decided by the owner)
- **Date:** 2026-07-10
- **Builds on:** ADR-008 (the node/vo_version architecture — unchanged);
  ADR-007 (baseline-squash method — reused). The PG18 enterprise-practices
  dossier and spec-persistence-requirements research that fed this ADR are
  distilled inline (the load-bearing operational guidance now lives in the
  Appendix below, merged 2026-07-11 when the standalone B7 research dossiers
  were retired).

## Context

The schema grew accretively (10 `ehr` migrations + 1 `ext`) through
P10→B6: columns bolted on with sentinel defaults, CHECKs dropped/re-created
by auto-generated name, 13 of 19 tables outside any audit discipline, a
stale/partial `iden.rs`, write-only index structures, and **zero**
operational surface (no roles, no grants, no comments). The spec-persistence
requirements extraction also surfaced concrete gaps. Nothing is deployed, so the
baseline can still be re-authored (ADR-007 method).

## Decision (owner-confirmed choices first)

1. **Fresh squashed baseline.** `ehr/0001_baseline.sql` + `ext/0001_*.sql`
   re-authored; migrations 0002–0010 deleted. Append-only forever after.
2. **Temporal PK stays** — `PRIMARY KEY (vo_id, sys_period WITHOUT
   OVERLAPS)` — **plus** the btree `UNIQUE (vo_id, sys_version)` as FK
   target and explicit **replica identity** (`REPLICA IDENTITY USING
   INDEX`). Revisit only on P20 benchmark evidence.
3. **Roles in migrations.** Idempotent creation of `ehrbase_migrator`,
   `ehrbase_app` (writer), `ehrbase_reader`; per-schema GRANTs + `ALTER
   DEFAULT PRIVILEGES` so future tables are reachable; `REVOKE CREATE ON
   SCHEMA public FROM PUBLIC`. `ext` functions owned by the migrator, app
   roles get EXECUTE only. pgaudit/TLS stay deployment-layer (Appendix §3).
4. **Perf mechanisms wired now (owner call, against the measure-first
   lean):** keep `node_data_gin` AND create the ADR-008 btree expression
   indexes over `ext.openehr_magnitude(data)` (scoped: a partial index on
   leaf-bearing nodes). P20 validates/repricing; recorded as speculative.

### Spec-compliance fixes (spec-persistence-requirements anchors)

5. `ehr.system_id text NOT NULL` recorded at creation (req 2.1 — immutable
   per-EHR value, not just service config).
6. `audit.change_type` CHECK against the audit-change-type group codes
   (`249,250,251,523,666`) (req 1.4.2); `audit.committer jsonb` stays.
7. `vo_version.other_input_version_uids jsonb` (nullable) — merge
   provenance for imported merged versions (req 1.3.4); trunk-only
   branching remains the PORT-NOTEd typed rejection, but the identity
   columns no longer lose data on import.
8. `creating_system_id text NOT NULL` **without** the `''` sentinel — the
   service writes the real creating system id on every version (local
   commits: our system_id; imports: preserved).
9. `preceding-version` invariant guard: `CHECK (sys_version = 1 OR NOT
   first)` is app-enforced today; the baseline adds
   `ck_vo_version_sys_version_positive` and the nested-set CHECKs on `node`
   (`num_cap >= num`, `parent_num < num` for num > 0).
10. Subject uniqueness (req 2.8, CNF-hard): the partial unique on
    `(subject_id, subject_namespace)` stays, now named + commented with the
    CNF citation and the RM-soft nuance.

### Enterprise hygiene (PG18 enterprise-practices anchors — see Appendix)

11. Deterministic names for every constraint/index (`pk_/uq_/fk_/ck_/idx_`).
12. `COMMENT ON` every table, every non-obvious column (nested-set
    encoding, `sys_period` semantics, verbatim-canonical `data`), every
    `ext` function — the ADR-008 rationale ships inside the database.
13. `fk_node_vo_version` becomes `DEFERRABLE INITIALLY IMMEDIATE` (atomic
    multi-row version commits); all other FKs immediate; every FK child
    column indexed.
14. `vo_version` `fillfactor = 90` (one close-out UPDATE per supersession);
    `node` stays 100 (append-only). `node.data` + `audit.committer` get
    `COMPRESSION lz4`.
15. Missing FK added: `sp_variable.frame_id → sp_data_frame(frame_id)`.
    `item_tag.target_vo_id` stays FK-less deliberately (req 5.3.2: tags are
    loose, may target a VERSION or container, outside the version chain) —
    PORT NOTE in the DDL comment.
16. `template_store` keeps dual identity (uuid `id` = the SM's
    OPT-keyed-by-UUID, req 5.1.1; unique `template_id` = the wire address)
    — now documented in comments; the three definition stores stay separate
    (different identity schemes per formalism, req 5.1.1/5.1.2).
17. `db/iden.rs` regenerated complete + correct (all 19 tables; the phantom
    `Deleted` variant removed); it becomes the single typed name catalog.

## Consequences

- The service layer changes minimally: `ehr.system_id` on create,
  `creating_system_id` real value (no sentinel fallback), iden.rs renames.
- `tests/persistence.rs` migration-count/table assertions update to the new
  baseline (legitimate fixture change, cited).
- Gates: full workspace suites green + **full ECC zero drift vs 341
  executed · 315 passed · 0 failed** — the wire must not notice.
- The spec-silence register (spec-persistence requirements) is the standing PORT NOTE list:
  signature canonicalisation, ATNA store shape, retention/purge policy,
  EHR_ACCESS model, template versioning — all deliberately open.

## Alternatives considered

Append-corrective migrations (keeps jank in the permanent record);
plain-unique-only versioning (loses engine-enforced non-overlap); roles as
deployment docs (dev/prod divergence); dropping the GIN + deferring
expression indexes (the measure-first default — owner chose to wire now).

## Appendix: PG18 operational practices (merged from the B7 research dossier, 2026-07-11)

The operational guidance below is the load-bearing distillate of the B7
PostgreSQL-18 enterprise-practices research (web-sourced Opus fan-out, official
PG docs corroborated). It is **deployment-layer** — the schema references it but
cannot enforce it — and is what `docs/enterprise/deployment.md` cites. Section
numbers are preserved from the original dossier so existing cross-references
(§3/§5/§6) stay meaningful.

### §3 Security (PHI)

- **§3.1 Four-role model** — owner / `ehrbase_migrator` (DDL) / `ehrbase_app`
  (DML writer) / `ehrbase_reader` (SELECT); never a superuser at runtime. The
  migrations create these idempotently (Decision 3).
- **§3.2 `ALTER DEFAULT PRIVILEGES`** so future migrations' tables stay
  reachable without manual grants (a classic deploy-outage otherwise).
- **§3.3 RLS** — skipped in single-tenant Stage 1, but `ehr_id` is placed
  RLS-ready on `node`/`vo_version`; Stage-2 multi-tenancy (ADR-015) adopts
  `FORCE ROW LEVEL SECURITY`.
- **§3.4 At-rest encryption** at the volume/disk layer (PG has no native TDE).
  Do **not** pgcrypto-encrypt `node.data` — it would kill AQL jsonpath and the
  zero-translation storage design.
- **§3.5 pgaudit** as the DB-layer complement to the app-level openEHR audit +
  the ATNA `system_log`: `pgaudit.log = 'ddl, role, connection'` globally plus
  object-level audit on the PHI tables only; ship to an immutable store with
  ≈6-year retention.
- **§3.6 TLS in transit** — require `hostssl` on the server and `sslmode=verify-full`
  in the DSN; pin `search_path` on any SECURITY DEFINER function; `REVOKE CREATE
  ON SCHEMA public FROM PUBLIC`. `ext` functions are plain (not definer), owned
  by the migrator role the app cannot write.

### §5 Migration hygiene (sqlx)

- **§5.1** Squash the greenfield baseline to one `0001` per schema (done in
  Decision 1); append-only forever after — never edit an applied migration.
- **§5.2 Lock-safe DDL** for live systems: `CREATE INDEX CONCURRENTLY`;
  constraints as `NOT VALID` then a later `VALIDATE`.
- **§5.3** `lock_timeout` (≈5s) + a bounded `statement_timeout` set on the
  migrator connection (not per-file) so a migration cannot block live traffic
  indefinitely on a lock.

### §6 Backup / retention / PITR

- **§6.1** WAL archiving + PITR from day one (pgBackRest or a managed PITR).
- **§6.2** `UNLOGGED` is forbidden on all clinical/audit tables (crash-truncated,
  invisible to replicas/PITR).
- **§6.3 Replica identity** — the temporal PK (GiST) may not satisfy logical
  replication; the explicit btree `UNIQUE (vo_id, sys_version)` is the replica
  identity (`REPLICA IDENTITY USING INDEX`), and every table has a PK.
