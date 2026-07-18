# `testkit` — the shared test-database harness (tooling, not part of the app)

One PostgreSQL 18 server + one migrated **template database** per migration
fingerprint + one `CREATE DATABASE … TEMPLATE` clone per test. Every
DB-backed test in the workspace gets its database from `testkit::db()` —
never by starting its own container or running its own migrations.

- **Server resolution:** `EHRBASE_TEST_PG_URL` (CI, local dev server) →
  else the reusable named container `ehrbase-testkit-pg18` (testcontainers
  `reusable-containers`; deliberately left running across runs — reclaim
  with `docker rm -f ehrbase-testkit-pg18`). The container runs the
  non-durable settings the PostgreSQL docs describe for throwaway data
  (`fsync=off` etc.) — never copy those flags anywhere near production
  config.
- **Template discipline:** the template is migrated by the platform
  library's own `db::run_migrations` and keyed on
  `db::migration_fingerprint()`; completion is stamped as the database
  comment. Never connect to the template outside the build path — an open
  connection blocks every clone (PostgreSQL docs § CREATE DATABASE).
- **Isolation is per-database, not per-server.** A test that needs
  server-global state (ALTER SYSTEM, cross-database assertions) does not
  belong on the shared server — flag it instead of hacking around the
  harness.
- **Dependency shape:** `testkit → ehrbase` (normal dep); the app crates'
  tests consume testkit as a dev-dependency. The `ehrbase` ↔ `testkit`
  dev-dependency cycle is deliberate and cargo-legal — do not "fix" it.
- Failures are typed (`TestkitError`) and always mean broken test
  infrastructure, never application behaviour — call sites `.expect()`.
- Gates: `cargo clippy -p testkit --all-targets` +
  `cargo nextest run -p testkit`.
