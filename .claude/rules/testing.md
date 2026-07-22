---
paths: ["crates/**", "app/**", "tools/**"]
---

# Testing discipline

Test discipline is non-negotiable (a standing hard rule; see `CLAUDE.md`). It
applies to every crate — generated and hand-written alike.

## The hard rule

- **Never** silently weaken, skip, or delete an existing test to make a
  build pass.
- **Never** edit a test to route around a runtime bug it exposes. If a test
  fails and the fix is unclear, leave it failing and record a
  `// TODO:` — do not touch the test to make it green.
- Conformance/corpus tests assert the **openEHR specifications**:
  cite the spec clause a test encodes; never adjust an expectation to match
  an implementation bug. Corpus/fixture defects are ADJUDICATED with a
  first-hand spec/schema citation (an expected-rejection entry in the owning
  gate, or an `artifacts/registers/ambiguities.yaml` entry for spec silence
  — `.claude/rules/cnf-triage.md`), never routed around by editing the case.

## Tooling

- **Runner:** `cargo-nextest` (`cargo nextest run --workspace`), not
  `cargo test`.
- **Snapshots:** `insta` pins canonical JSON/XML output against golden
  vectors — the key tool for serialization parity. Redact volatile fields
  (timestamps, generated UUIDs) before snapshotting. Review intentional
  changes with `cargo insta review`; never accept a snapshot change you have
  not read.
- **Properties:** `proptest` for RM round-trips (serialize → parse → equal),
  parser stability, and AQL parse/print round-trips.
- **Database:** every DB-backed test takes its database from the shared
  harness — `testkit::db()` (`tools/testkit`): one PostgreSQL 18 server
  (`EHRBASE_TEST_PG_URL` in CI, a reusable `ehrbase-testkit-pg18`
  testcontainer locally), one migrated template database per migration
  fingerprint, one `CREATE DATABASE … TEMPLATE` clone per call. **Never
  start a per-test PostgreSQL container or run migrations in a test** —
  that pattern is retired (it cost ~5–10 s + Docker contention per test).
  Broker/blob tests (RabbitMQ, SeaweedFS) still run real testcontainers,
  serialized via the nextest `containers` group.
- **HTTP mocking:** `wiremock` for the terminology/FHIR client and any
  external integration test.
- **Benches:** `criterion` + `divan`, kept separate from correctness tests.

## Oracles and the acceptance instrument

- **The acceptance instrument is the CNF 2.0 runner** (`tools/cnf-runner`,
  `scripts/conformance.sh`) — the data-driven interpreter over the committed
  machine-readable catalogue, with pure-function verdicts. Phase-close runs
  must show **zero drift** vs the committed baseline
  (`docs/conformance/ehrbase-rs/results.json` + `verdicts.json`); the
  baseline only ratchets upward. (The ECC harness retired 2026-07-22; the
  reviewed comparison is `docs/conformance/cnf-comparison.md`.)
- **The vendored CNF text is the oracle the instrument derives from:**
  `docs/specs/openehr/CNF/docs/platform_test_schedule/` defines what a
  conformant server must do; the upstream Robot suites under
  `CNF/tests/platform/robot/` are reference material, and their official
  DATA fixtures are adopted into the runner corpus as provenance-stamped
  re-adjudications (never blind imports).
- Golden vectors: openEHR conformance corpora, the vendored canonical-JSON
  corpus, Better's `web-template-tests`, openEHR reference archetypes.
  Prefer an existing golden vector over a hand-written fixture. A test that
  encodes a spec rule cites the spec/CNF section it asserts
  (spec-adherence.md).

## Where tests live

Unit tests live beside the code they test (`#[cfg(test)] mod tests` in the
same file) — and ONLY there: **dedicated test FILES under `src/` are banned**
(owner ruling 2026-07-17; the four historical ones were relocated). A test
that drives the public API belongs in the owning crate's `tests/` directory
(`crates/*/tests/`, `app/*/tests/`, `tools/*/tests/`) with a descriptive
file name; a test of private internals stays a small inline module next to
the code it tests. If an internals test grows large, that is a design signal
to test through the public seam, not to split the tests into a src file.
Do not invent a third location.

## Target

A green CNF pipeline is the standing bar (CORE + STANDARD PASS) —
every change preserves it; the baseline only ratchets upward. Every phase
ships compiling, clippy-clean, tested increments — this whole rule is fully
active at all times.
