# `ehrbase` — the platform library

The application core (four app crates, zero re-exports). Top-level modules
(`src/lib.rs`): `service` (the SM service layer), `storage`, `aql` (the query
engine), `versioning` (change-control + VERSION `signature` signing),
`validation`, `templates`, `db` (sqlx pool + migrations), `config` (the full
`ehrbase.toml` tree), `telemetry`, `system_log` (IHE ATNA), `ids`, `codec_serde`,
`extensions`, `banner`. Hand-written idiomatic Rust of our own design on the
generated `openehr-*` crates. The binary lives in `app/ehrbase-server`; the REST
adapter (`ehrbase-rest`) depends on this crate and calls the concrete
`EhrbaseService` directly. **Zero re-exports: every import names its defining
module.**

- **Service layer = one module per SM chapter, concrete methods, no trait
  catalog** (`service::{ehr, definition, demographic, query, validity, admin,
  ehr_index, terminology, message, subject_proxy}` + support modules
  `committer`, `version_update`, `status`, `response`, `error`,
  `platform_service`). SM design authority: `docs/specs/openehr/SM/`.
- **Spec first:** every spec-facing behaviour (versioning/change-control,
  validation, AQL semantics) is implemented from the vendored text under
  `docs/specs/openehr/` (`/spec-lookup`) — never from memory or EHRbase
  behaviour. Cite spec file + section in comments; the only citable references
  are the vendored specs and official external docs — NEVER an internal doc.
- **Storage is greenfield PG18:** one `node` table (nested-set interval index,
  canonical JSON fragments — no aliasing, no synthetic fields) + one temporal
  `vo_version` table (`WITHOUT OVERLAPS`; `ALL_VERSIONS` supported). Every write
  emits contribution + audit in the same transaction. Change-control semantics
  are implemented against RM common master06 (`RM/docs/common/master06-change_control_package.adoc`)
  — do not regress them casually.
- **AQL engine** (`src/aql/`): typed IR over the BMM-generated RM model, lowered
  via `sea-query`; every unsupported construct is a typed reject, never a silent
  wrong answer. Rules: `.claude/rules/aql-engine.md`.
- **SQL:** `sqlx` + `sea-query` (never sea-orm); migrations only via
  `sqlx migrate add --sequential`. Rules: `.claude/rules/sqlx-conventions.md`.
- **System log** (`src/system_log/`): the ARR drain batches (`recv_many` → one
  multi-row UNNEST INSERT when syslog is off) with concurrent memoized subject
  resolution and rate-limited drop warnings; default `audit.queue_capacity` 8192.
- **Consume `openehr-*` types directly** — never re-model the RM or re-serialize;
  canonical JSON/XML goes through `openehr-its`.
- DB tests take their database from the shared harness — `testkit::db()`
  (`tools/testkit`; template-clone per test). Never start a per-test PG container
  or run migrations in a test. Cluster-global objects a test must create (login
  roles) are named off the clone db name so the testkit sweep reaps them.
- Gates: `cargo clippy -p ehrbase --all-targets` +
  `cargo nextest run -p ehrbase` green before commit; the CNF pipeline
  (`bash scripts/conformance.sh`) must show zero drift vs the committed baseline
  at phase close.
