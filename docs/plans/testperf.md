# TESTPERF — testing-framework full redesign + rewrite

Owner (2026-07-18): suites take longer and longer (e.g.
`extensions_openapi::every_documented_path_routes` exceeding 300 s); "we can
have a way better and more optimized testing setup". Owner clarified in
session: **a full redesign and a full rewrite**, not incremental patches.
Exit: measurably faster wall-clock with unchanged coverage (never weaker
tests). This file is deleted in the PR that lands the rewrite (worklist
lifecycle).

## Measured baseline (the honest before)

- CI `build & test (pg18)` job: **~19 min** per run (e.g. run 29653033002:
  17:00:59 → 17:19:55).
- Local full-workspace `cargo nextest run --workspace --all-features`:
  baseline run in flight this session (started 2026-07-18, epoch 1784396917);
  number recorded below when it lands.
- Local baseline wall-clock: **TBD (run in flight)**.

## Root cause (profiled, not guessed)

- **357 DB-backed `#[tokio::test]`s each start their own `postgres:18`
  testcontainer and run the full 3-migrator sequence** (`ext` → `ehr` →
  `audit`). nextest runs one process per test, so nothing shares. Under a
  full-workspace run dozens of containers start concurrently; Docker host
  contention balloons container start + migration latency — that is the
  300 s outlier, not the test's own loop (in-process `oneshot`, no real
  network).
- The per-test helper is **duplicated inline in ~28 files** in
  `app/ehrbase` + `app/ehrbase-server`; only `ehrbase-rest` has a shared
  `tests/common/mod.rs`.
- CI declares a `postgres:18.4` service that the tests never use (they spin
  containers docker-in-docker next to it).
- nextest config is minimal (one serialized `containers` group for the 3
  AMQP/S3 binaries); no profiles, no timeouts.

## The redesign

One new harness crate, **`tools/testkit`**, replaces every per-test
container + per-test migration with **one shared PG18 server + one
migrated template database + `CREATE DATABASE … TEMPLATE` per test**
(~100 ms instead of ~5–10 s + contention):

1. **Server resolution** (per test process, cheap):
   - `EHRBASE_TEST_PG_URL` set → use that server (CI, local dev PG).
   - else → start/adopt a **reusable named testcontainer**
     (`testcontainers` 0.27.3 `with_reuse(ReuseDirective::Always)`,
     feature `reusable-containers`), guarded by an OS file lock so
     concurrent first-boot races collapse to one container. Container runs
     PG18 tuned for tests (`fsync=off`, `synchronous_commit=off`,
     `full_page_writes=off` — safe only for throwaway test data, per the
     official PostgreSQL docs on non-durable settings).
2. **Template database per migration fingerprint**: hash of all migration
   SQL + the migrator sequence → `ehrbase_tk_tpl_<hash>`. First process to
   need it takes a PG advisory lock, creates + migrates the template,
   stamps the fingerprint as the database comment (readable via
   `shobj_description` without connecting — connections to a template
   block cloning). Everyone else fast-paths on the comment check.
   Migration change → new fingerprint → new template; stale templates
   swept.
3. **Per-test clone**: `CREATE DATABASE ehrbase_tk_<ts>_<rand> TEMPLATE …`
   (default `WAL_LOG` strategy — no forced checkpoints, per the PostgreSQL
   `CREATE DATABASE` docs). Returns `TestDb` (pool + name + guard).
   Cleanup: best-effort drop on guard drop + an advisory-try-locked sweep
   of old `ehrbase_tk_*` clones at harness init.
4. **API**: `testkit::db().await -> TestDb` (unique DB, migrated,
   pooled); multiple calls per test give independent databases (dump/load
   tests need two). `ehrbase-rest/tests/common/mod.rs` keeps its
   `test_service`/`test_router` signatures as thin shims over testkit so
   the 25 REST files change minimally; the ~28 inline `Pg` copies in
   `app/ehrbase` + `app/ehrbase-server` are deleted outright.
5. **AMQP/S3 suites** (`events_amqp`, `fhir_outbound_amqp`,
   `multimedia_s3`): keep their broker/S3 containers (that's what they
   test) but take PG from testkit; stay in the serialized `containers`
   nextest group with 1 retry.
6. **CI redesign**: the test + coverage jobs run PG18 as an explicit
   `docker run` step (service containers can't carry `-c` command flags)
   with the same non-durable tuning, and export `EHRBASE_TEST_PG_URL` — no
   docker-in-docker container pulls in tests at all.
7. **nextest profiles**: keep the `containers` group; add a `ci` profile
   (fail-fast off, junit) and a default `slow-timeout` warning period so
   the next slow-test regression surfaces immediately instead of silently
   growing.

Dependency note: `testkit` depends on `ehrbase` (for `db::connect` +
`db::run_migrations` + `DbConfig`); `app/ehrbase`'s own tests dev-depend on
`testkit` — a dev-dependency cycle, which cargo explicitly permits
(dev-dependencies do not participate in the release dependency graph).

## Tasks

- [x] Profile: CI job timings + infra map (357 DB tests, per-test container,
      duplication inventory) — recorded above.
- [x] Local baseline number recorded: **791 s (13.2 min)** full-workspace
      wall-clock on this machine (exit status of that prior-session run not
      captured; CI's ~19 min job is the primary baseline).
- [x] `tools/testkit` crate: env → reusable named container (create-race
      retry instead of a file lock), template-per-fingerprint under a PG
      advisory lock with the completion stamp as the database comment,
      clone + sweep (databases AND `ehrbase_tk_*` roles), `TestDb` guard,
      `empty_db()` for the storage spike. Unit + live harness tests green;
      warm path 0.386 s for two migrated databases, cold 1.7 s.
- [x] `app/ehrbase-rest/tests/common/mod.rs` rewritten onto testkit; 18
      REST test files adapted (name params dropped, `common::Pg` →
      `testkit::TestDb`).
- [x] The inline-helper files rewritten: 26 in `app/ehrbase/tests` + 3 in
      `app/ehrbase-server/tests`; every inline `Pg`/`migrated_pool` copy
      deleted; AMQP/S3 files keep brokers, PG via testkit;
      `storage_spike.rs` on `empty_db()`; `tenant_isolation.rs` roles are
      per-clone (`<clone>_tester`) — cluster-global-safe.
- [x] nextest.toml: `containers` group kept (brokers only), slow-timeout
      60 s flagging, `ci` profile (fail-fast off) wired into ci.yml.
- [x] ci.yml: test + coverage jobs export `EHRBASE_TEST_PG_URL` at the
      existing postgres:18.4 service + non-durable ALTER SYSTEM tuning —
      zero docker-in-docker containers in CI tests.
- [x] Docs: `.claude/rules/testing.md` + `sqlx-conventions.md` harness
      sections, root CLAUDE.md + `docs/architecture.md` tools lists,
      `app/ehrbase/CLAUDE.md`, testkit crate CLAUDE.md.
- [ ] Full gates green (workspace clippy/nextest/fmt) + honest
      before/after wall-clock recorded in `docs/PROGRESS.md`.
- [ ] `/phase-done`: worklist row closed, this file deleted.
