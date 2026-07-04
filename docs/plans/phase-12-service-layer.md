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

## Tasks

- [ ] Repositories (EHR, composition+versions, status, folder, contribution, audit, stored_query, item_tag)
- [ ] Versioning: current + `_history`, revision history, `OBJECT_VERSION_ID`
- [ ] Contribution + audit on every write; transactional integrity (`sqlx`)
- [ ] Wire the service into the P11 server-trait impls (EHR/composition/directory/contribution endpoints live)
- [ ] Integration tests against testcontainers PG 18 (create→retrieve→update→version)

## Exit criteria

- [ ] EHR + composition + directory + contribution CRUD + versioning work end to end over HTTP
- [ ] Each write produces a contribution + audit row; `_history` populated on update/delete
- [ ] Compiles + clippy-clean; integration tests green

## Decisions made this phase

- EHRbase's service/versioning semantics are the behavioural reference; parity is
  verified at the REST surface (P19), not by class-mirroring.
