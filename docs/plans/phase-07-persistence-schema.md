# Phase 07 — Persistence schema

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): none (infrastructure phase)
- Compile required: should compile

## Objectives

Copy EHRbase's Flyway migrations verbatim into `crates/openehr-server/migrations/`,
define the equivalent tables via `sea-query`, and prove the schema applies
cleanly against a real PostgreSQL 18 via `testcontainers`. This crate should
actually compile and its migrations should actually apply — no Phase-A slack.

## Preconditions

- [ ] Phase 00 done: `openehr-server` crate skeleton exists, migrations copied
      from `jooq-pg`/`db_scripts` verbatim during the `git mv`

## Scope

In: verbatim Flyway migration SQL, `sea-query` table definitions mirroring
`ehr.ehr`, `ehr.ehr_status_data`, `ehr.ehr_folder_data`, `ehr.comp_data`/`_history`,
`ehr.comp_version`, `ehr.contribution`, `ehr.audit_details`, `ehr.template_store`,
`ehr.stored_query`, `ehr.item_tag`, `sqlx` pool setup, testcontainers-driven
migration tests.
Out: jOOQ-generated code (discarded, replaced by sea-query per Section 9.1),
row-per-locatable read/write logic (Phase 14 owns `rm-db-format`), AQL SQL
generation (Phase 13).

## Tasks

- [ ] Confirm Flyway migration SQL was copied verbatim into `crates/openehr-server/migrations/` during Phase 00's `git mv`
- [ ] Set up `sqlx::PgPool` construction and configuration (connection string, TLS via `tls-rustls-aws-lc-rs`)
- [ ] Define `sea-query` table/column definitions for `ehr.ehr`, `ehr.ehr_status_data`, `ehr.ehr_folder_data`
- [ ] Define `sea-query` table/column definitions for `ehr.comp_data`/`_history` and `ehr.comp_version`
- [ ] Define `sea-query` table/column definitions for `ehr.contribution`, `ehr.audit_details`
- [ ] Define `sea-query` table/column definitions for `ehr.template_store`, `ehr.stored_query`, `ehr.item_tag`
- [ ] Verify required PostgreSQL extensions (`uuid-ossp`, `pgcrypto`, `pg_trgm`) are enabled by a migration
- [ ] Write a `testcontainers` + `testcontainers-modules` test that spins up PostgreSQL 18 and applies every migration cleanly
- [ ] Add PORT STATUS trailers referencing `jooq-pg`'s generated schema as the source

## Exit criteria

- [ ] `cargo check -p openehr-server` passes for the persistence module in isolation
- [ ] All Flyway migrations apply cleanly against a real PostgreSQL 18 container
- [ ] Every table in Section 6's persistence list has a `sea-query` definition

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. Because current + `_history` table pairs use triggers for
versioning in EHRbase v2, capture the trigger SQL verbatim in the migration
copy even though the trigger logic itself isn't ported to Rust until Phase 15
needs to reason about it.
