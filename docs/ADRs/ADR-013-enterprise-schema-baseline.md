# ADR-013: Enterprise-grade schema baseline (B7 redesign)

- **Status:** accepted (owner-confirmed 2026-07-10; the four structural
  choices below were explicitly decided by the owner)
- **Date:** 2026-07-10
- **Builds on:** ADR-008 (the node/vo_version architecture — unchanged);
  ADR-007 (baseline-squash method — reused). Evidence set:
  `docs/design/schema-review/01..03` (current-schema inventory, PG18
  enterprise dossier, spec persistence requirements).

## Context

The schema grew accretively (10 `ehr` migrations + 1 `ext`) through
P10→B6: columns bolted on with sentinel defaults, CHECKs dropped/re-created
by auto-generated name, 13 of 19 tables outside any audit discipline, a
stale/partial `iden.rs`, write-only index structures, and **zero**
operational surface (no roles, no grants, no comments). The spec extraction
(review doc 03) also surfaced concrete gaps. Nothing is deployed, so the
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
   roles get EXECUTE only. pgaudit/TLS stay deployment-layer (documented in
   `docs/design/schema-review/02`).
4. **Perf mechanisms wired now (owner call, against the measure-first
   lean):** keep `node_data_gin` AND create the ADR-008 btree expression
   indexes over `ext.openehr_magnitude(data)` (scoped: a partial index on
   leaf-bearing nodes). P20 validates/repricing; recorded as speculative.

### Spec-compliance fixes (review doc 03 anchors)

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

### Enterprise hygiene (review doc 02 anchors)

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
- The spec-silence register (review doc 03) is the standing PORT NOTE list:
  signature canonicalisation, ATNA store shape, retention/purge policy,
  EHR_ACCESS model, template versioning — all deliberately open.

## Alternatives considered

Append-corrective migrations (keeps jank in the permanent record);
plain-unique-only versioning (loses engine-enforced non-overlap); roles as
deployment docs (dev/prod divergence); dropping the GIN + deferring
expression indexes (the measure-first default — owner chose to wire now).
