# Phase 12 — Service layer (orchestration, versioning, contributions, audit)

- Status: not-started (Stage-1 app build, step 4 of 13)
- Consumes: `openehr-rm`, P09 (tables), P10 (rm-db-format), P11 (server traits)
- Compile required: yes (compiling, tested increment)
- Decisions: ADR-006 (idiomatic; EHRbase service semantics as reference)

## Objectives

The application service layer that turns REST calls into persisted, versioned
openEHR data: EHR / composition / directory (folder) / contribution / stored-query
orchestration, with **versioning** (current + `_history`), **contributions**, and
**`audit_details`** on every write, inside `sqlx` transactions. Implements the
generated server traits from P11. Follows EHRbase's service semantics (its
`service/` + `api/repository/` Java is the behavioural reference), written
idiomatically.

## Preconditions

- [ ] P09 (schema/pool), P10 (rm-db-format), P11 (server foundation)

## Scope

**In:** repositories over the v2 tables (`sea-query` + `sqlx`); create/retrieve/
update/delete for EHR, `EHR_STATUS`, COMPOSITION, DIRECTORY/FOLDER; version
control (`VERSIONED_OBJECT`, `ORIGINAL_VERSION`, current + `_history` via the
versioned tables); every write emits a `CONTRIBUTION` + `AUDIT_DETAILS`;
optimistic concurrency (`If-Match`/version uid); stored-query CRUD; item tags.
**Out:** composition *validation* against templates (P15 — called from the
create/update path once available); AQL execution (P16); FLAT/EhrScape (P17).

Note (ADR-008 reconciliation): the greenfield schema has no `_history` pairs —
versioning is the temporal `vo_version` table (current = `upper_inf(sys_period)`;
history = the same table). "repositories" are the `ehrbase::service` modules over
`node`/`vo_version`, not per-table repos.

## Tasks

- [x] EHR / EHR_STATUS / COMPOSITION / DIRECTORY(FOLDER) / CONTRIBUTION persisted
      on the shared `vobject` machinery (`node` + `vo_version`), via the P10 codec.
      (stored-query + item-tag CRUD still open — see below.)
- [x] Versioning: temporal `vo_version` (no `_history`), `OBJECT_VERSION_ID`,
      version-by-id + time-travel (`version_at_time`); optimistic concurrency
      (`If-Match`). (Full REVISION_HISTORY list still open.)
- [x] Contribution + audit on every write; one `sqlx` transaction per change.
- [x] Wire the service into the server-trait impls via the `Backend` seam
      (dependency inversion); EHR/EHR_STATUS/composition/directory/contribution
      operations live; the binary boots with the DB-backed service.
- [x] Integration tests against testcontainers PG 18 (`tests/service_ehr.rs`):
      create→retrieve→update→version→time-travel→delete, conflict, not-found — 3/3.

Still open (machinery ready; scoped next, not shortcuts): `contribution_create`
(atomic multi-version apply — shared-contribution write path), REVISION_HISTORY,
stored-query + item-tag CRUD, `ehr_get_by_subject`, template linkage (needs P13
`template_store`), demographic (RM phase), query/AQL (P16), fine-grained RBAC (S2).

## Exit criteria

- [x] EHR + EHR_STATUS + composition + directory + contribution retrieval work end
      to end over the service API (REST → dispatcher → service → PG 18); versioning
      + time-travel verified. (`contribution_create` write path is the next step.)
- [x] Each write produces a contribution + audit row in the same transaction;
      updates/deletes add a new temporal version (no `_history` pairs — ADR-008).
- [x] Compiles + clippy-clean; integration tests green (PG 18 testcontainers).

## Decisions made this phase

- **Dependency inversion** for the service seam: `ehrbase-rest` owns a `Backend`
  trait (union of the five generated `*Api` traits); `ehrbase` implements it and
  injects it — the DB service stays in the app crate with no dependency cycle.
- **Generated traits carry default `NotImplemented` bodies** so implementors
  override only what they support — eliminates duplicated stub lists.
- **One versioned-object machinery** (`vobject`) serves COMPOSITION / EHR_STATUS /
  FOLDER; versioning is the temporal `vo_version` table (ADR-008), not
  current+`_history`; `version_at_time` is a `sys_period @>` lookup.
- Committer is taken from the authenticated principal via a request-scoped
  task-local set by the auth middleware (no change to the generated trait sigs).
- Static SQL uses runtime `sqlx::query*` (no build-time DB); bulk node inserts use
  `sqlx::QueryBuilder`; `sea-query` is reserved for the AQL engine (P16).
