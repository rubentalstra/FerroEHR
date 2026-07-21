# `ehrbase` — the platform library

The application core (four app crates, zero re-exports): storage, the service layer
(one module per SM chapter — concrete `EhrbaseService` methods, no trait
catalog), AQL engine, versioning, the full configuration tree
(`ehrbase::config`), telemetry, plus the `signing` (VERSION.signature) and
`system_log` (IHE ATNA) modules. Hand-written idiomatic Rust of our own
design on the generated `openehr-*` crates. The binary lives in
`app/ehrbase-server`; the REST adapter (`ehrbase-rest`) depends on this
crate and calls the service directly. **Zero re-exports: every import names
its defining module.**

- **Spec first:** every spec-facing behaviour (versioning/change-control,
  validation, AQL semantics) is implemented from the vendored text under
  `docs/specs/openehr/` (`/spec-lookup`) — never from memory or EHRbase
  behaviour. Cite spec file + section in comments; the only citable
  references are the vendored specs and official external docs — NEVER an
  internal doc.
- **Storage is greenfield PG18:** one `node` table (nested-set
  interval index, canonical JSON fragments — no aliasing, no synthetic
  fields) + one temporal `vo_version` table (`WITHOUT OVERLAPS`;
  `ALL_VERSIONS` supported). Every write emits contribution + audit in the
  same transaction. Change-control semantics are formally audited 1:1
  against RM common master06 — do not regress them casually.
- **AQL engine:** typed IR over the BMM-generated RM model, lowered via
  `sea-query`; every unsupported construct is a typed reject, never a
  silent wrong answer. Rules: `.claude/rules/aql-engine.md`.
- **SQL:** `sqlx` + `sea-query` (never sea-orm); migrations only via
  `sqlx migrate add --sequential`. Rules: `.claude/rules/sqlx-conventions.md`.
- **Consume `openehr-*` types directly** — never re-model the RM or
  re-serialize; canonical JSON/XML goes through `openehr-its`.
- DB tests take their database from the shared harness — `testkit::db()`
  (`tools/testkit`; template-clone per test). Never start a per-test PG
  container or run migrations in a test. Cluster-global objects a test must
  create (login roles) are named `<clone-db-name>_<suffix>` so the testkit
  sweep reaps them.
- Gates: `cargo clippy -p ehrbase --all-targets` +
  `cargo nextest run -p ehrbase` green before commit; full ECC
  (`scripts/conformance.sh`) must show zero drift at phase close.
