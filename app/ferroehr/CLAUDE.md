# `ferroehr` — the platform library

The application core (four app crates, zero re-exports). Top-level modules
(`src/lib.rs`): `service` (the SM service layer), `storage`, `aql` (the query
engine), `versioning` (change-control + VERSION `signature` signing),
`validation`, `templates`, `db` (sqlx pool + migrations), `config` (the full
`ferroehr.toml` tree), `telemetry`, `system_log` (IHE ATNA), `ids`, `codec_serde`,
`extensions`, `banner`. Hand-written idiomatic Rust of our own design on the
generated `openehr-*` crates. The binary lives in `app/ferroehr-server`; the REST
adapter (`ferroehr-rest`) depends on this crate and calls the concrete
`FerroEhrService` directly. **Zero re-exports: every import names its defining
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
- **Benches** (`benches/aql.rs`, criterion, `harness = false`): every bench
  emits a CPU flamegraph under `--profile-time`
  (`cargo bench -p ferroehr --bench aql -- --profile-time 10` →
  `target/criterion/<bench>/profile/flamegraph.svg`). New benches copy that
  file's `criterion::profiler::Profiler`-over-`pprof` impl — never enable
  pprof's own `criterion` feature (pinned to criterion ^0.5, incompatible
  with our 0.8). Profiling how-to: the `/flamegraph` skill.
- **One integration-test binary:** `tests/it/main.rs` + one `mod` per topic
  file; `tests/resources/` holds the shared fixtures (paths are anchored at
  `CARGO_MANIFEST_DIR`). A new suite is a module registered in `main.rs`, never
  a new top-level `tests/*.rs`. The three container suites (`events_amqp`,
  `fhir_outbound_amqp`, `multimedia_s3`) are serialized by the nextest
  `containers` group, which matches them by module prefix — renaming one of
  those modules means updating `.config/nextest.toml`.
- Gates: `cargo clippy -p ferroehr --all-targets` +
  `cargo nextest run -p ferroehr` green before commit; the CNF pipeline
  (`bash scripts/conformance.sh`) must show zero drift vs the committed baseline
  at phase close. A red row is attributed spec-first
  (`.claude/rules/cnf-triage.md`): this server is a suspect, never assumed
  correct — never bend the catalogue/runner to match it.
