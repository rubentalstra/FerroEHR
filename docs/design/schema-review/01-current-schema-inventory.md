# B7 schema review — 01: current-schema inventory (2026-07-10)

Research output (Opus fan-out, B7 task 1a); factual map of
`app/ehrbase/migrations/{ehr,ext}` + code usage. Feeds ADR-013.

## Headline findings

**Structure**: 19 tables (6 in the versioning/audit discipline: `ehr`,
`vo_version`, `node`, `contribution`, `audit`, `vo_attestation`; 13 outside
it: `template_store`, `stored_query`, `item_tag`, `archetype_store`,
`adl2_artefact`, `ehr_index`, `vo_archive`, `sp_subject`, `sp_binding`,
`sp_data_frame`, `sp_variable`, `sp_data_set`) + 6 `ext` functions (only
`openehr_magnitude` called directly; 5 helpers transitive-only).

## Accretion smells (verified)

1. `vo_version` column pile-up: `signature` (0002), `creating_system_id NOT
   NULL DEFAULT ''` (0008 — magic empty-string sentinel); `kind` CHECK
   dropped/re-created **twice** (0003, 0007) via PG's auto-generated
   constraint name (fragile). `item_tag.target_type` CHECK same pattern
   (0003).
2. `db/iden.rs` stale + partial: `VoVersion::Deleted` names a nonexistent
   column (dead variant); `signature`/`creating_system_id`/`lifecycle_state`
   missing; `Ehr` omits `subject_id`/`subject_namespace`; only 8/19 tables
   have Iden defs — names string-duplicated across ~15 service files.
3. Three parallel definition stores: `template_store` (dual-addressed: uuid
   `id` AND unique `template_id` — deleted by one, read by the other),
   `archetype_store` (`archetype_id` PK), `adl2_artefact` (`hrid` PK).
4. Missing FKs: `item_tag.target_vo_id` (dangling tags possible),
   `sp_variable.frame_id` (despite UNIQUE(frame_id) existing for it),
   `vo_archive.vo_id` (intentional, commented).
5. 0003 dropped NOT NULL on `ehr_id` across 4 tables (contribution,
   vo_version, node, item_tag) to shoehorn ehr-less demographics — weakens
   the EHR-scoped majority case; party rows distinguished by `ehr_id IS NULL`.
6. Text-everywhere: `kind`, `lifecycle_state` ('532' strings),
   `target_type`, `instance_type`, `query_type` all text+CHECK;
   `audit.change_type` has **no CHECK at all**.
7. No audit/temporal discipline on the 13 side tables (just `created_at`).

## Write-only / unused mechanisms

- `node_data_gin` (GIN jsonb_ops): **no query uses it** (extraction is
  `jsonb_path_query_first`; the only `@>` is on `sys_period`). Write cost,
  zero reads.
- `node.path COLLATE "C"` + `citem_num`: written, read only for reassembly —
  never a predicate/index in the AQL generator (CONTAINS uses num/num_cap).
- ADR-008's promised `openehr_magnitude` **expression indexes**: none exist.

## ADR-008 promised vs actual

Delivered: unified `node`, temporal `vo_version` (`WITHOUT OVERLAPS` +
`upper_inf` current partial), ALL_VERSIONS, uuidv7, canonical verbatim
fragments, contribution+audit per write, nested-set CONTAINS.
Deltas: 13 tables beyond the ADR's enumerated set (SM phases 0004–0010);
`ehr.subject_id/subject_namespace` denormalized copy of EHR_STATUS.subject
(synced at vobject.rs:819) — promoted column the ADR never mentions; both
temporal PK AND `UNIQUE (vo_id, sys_version)` (the latter as FK target);
designed-for perf mechanisms (expression indexes, GIN pre-filter) unwired.

## Operational surface

**Confirmed zero**: no GRANT/REVOKE/CREATE ROLE, no RLS/POLICY, no
partitioning, no retention automation. Single connection role,
`search_path = ehr, ext, public`; only `btree_gist` extension + two
`_sqlx_migrations` tables. `vo_archive` is a pure marker (tier movement
deferred P20).

## Detail reference

Full per-table columns/indexes/readers-writers map (file:line) is in the B7
research transcript; key write paths: `vobject.rs:884/1158/1191/1394`
(version lifecycle), `vobject.rs:316` (node insert), `admin.rs:81` (cascade
delete root), `dump_load.rs` (archive IO), AQL over `node` via `aql/sql.rs`
(iden.rs Idens used only there).
