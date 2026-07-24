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
  reviewed cutover comparison lives in git history.)
- **The vendored CNF text is the STALLED structural GUIDE the instrument's
  COVERAGE derives from — NOT the correctness oracle** (owner ruling
  2026-07-24; openEHR CNF never released a stable version):
  `docs/specs/openehr/CNF/docs/platform_test_schedule/` names WHICH behaviours
  to exercise, but the CORRECT behaviour is derived from the RELEASED spec
  components (RM / BASE / AM / QUERY / TERM / ITS-XML / SM / ITS-REST docs text) —
  where the schedule and a released spec conflict, the released spec wins. The
  upstream Robot suites under `CNF/tests/platform/robot/` are stalled reference
  material; their official DATA fixtures are adopted into the runner corpus only
  as provenance-stamped re-adjudications (never blind imports, never as an
  oracle).
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

## CNF coverage (breadth is a mandate, not just pass rate)

A green pipeline over a thin catalogue proves almost nothing — the real
acceptance bar is COVERAGE. The CNF catalogue (`tools/cnf-runner/artifacts`)
must exercise EVERYTHING the spec defines on the wire: every SM operation,
every status-code branch (200/201/204/400/404/409/412/422/…), every
required/conditional header (`ETag`, `Location`, `Last-Modified`, `Prefer`,
`If-Match`), content-negotiation variants (JSON + XML, `Accept` q-values),
precondition and error families, and every RM/AQL behaviour — each as its own
small, ISOLATED case so a red row localizes to one behaviour. Every small
use-case counts; the goal is total behavioural coverage.

- **A spec-defined wire behaviour with no case is a COVERAGE GAP, never an
  acceptable omission.** Close it (a new spec-cited case) — or, only where the
  spec genuinely puts a behaviour off-wire, record the honest boundary
  (statement-declared capability / an `artifacts/registers/` entry). Silence is
  not coverage.
- **Coverage only ratchets up.** Cases are added, never removed to go green;
  narrowing coverage needs an adjudicated, spec-cited reason.
- **One behaviour per case** — many small isolated cases beat one broad case,
  because a failure then names exactly one defect (which is also what makes the
  attribution law tractable, `.claude/rules/cnf-triage.md`).
- Same completeness discipline the vendored corpora carry (100% exercised,
  coverage-gated — never partial coverage that silently narrows the claim).

## Target

A green CNF pipeline is the standing bar (CORE + STANDARD PASS) —
every change preserves it; the baseline only ratchets upward, and green comes
ONLY from fixing the guilty component after spec-adjudicated attribution
(`.claude/rules/cnf-triage.md`), NEVER from bending the catalogue or runner to
match this server. Every phase ships compiling, clippy-clean, tested
increments — this whole rule is fully active at all times.
