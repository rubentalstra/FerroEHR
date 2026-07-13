---
paths: ["crates/**", "app/**", "tools/**"]
---

# Testing discipline

Test discipline is non-negotiable (a standing hard rule; see `CLAUDE.md`). It
applies to every crate — generated (ADR-004) and hand-written alike.

## The hard rule

- **Never** silently weaken, skip, or delete an existing test to make a
  build pass.
- **Never** edit a test to route around a runtime bug it exposes. If a test
  fails and the fix is unclear, leave it failing and record a
  `// TODO(port):` — do not touch the test to make it green.
- Conformance/corpus tests assert the **openEHR specifications** (ADR-008):
  cite the spec clause a test encodes; never adjust an expectation to match
  an implementation bug. ECC corpus/golden defects go through the
  adjudication registers (skip-with-reason), never through editing the case
  (`tools/conformance/CLAUDE.md`).

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
- **Database:** `testcontainers` + `testcontainers-modules` run a real
  PostgreSQL 18 in Docker; `sqlx::test` fixtures; verify migrations apply
  cleanly as part of the fixture setup.
- **HTTP mocking:** `wiremock` for the terminology/FHIR client and any
  external integration test.
- **Benches:** `criterion` + `divan`, kept separate from correctness tests.

## Oracles and the acceptance instrument

- **The acceptance instrument is the ECC suite** (`tools/conformance`,
  `scripts/conformance.sh`) — our own conformance framework with its own
  numbering and generated data sets. Phase-close ECC runs must show **zero
  drift** vs the committed baseline (`docs/conformance/results.json`); the
  baseline only ratchets upward (blueprint §4 rule 4).
- **The vendored CNF text is the oracle the instrument derives from:**
  `docs/specs/openehr/CNF/docs/platform_test_schedule/` defines what a
  conformant server must do; the upstream Robot suites + fixtures under
  `CNF/tests/platform/robot/` are *reference material only* — ECC cases are
  never mapped to or imported from them (owner ruling).
- Golden vectors: openEHR conformance corpora, the vendored canonical-JSON
  corpus, Better's `web-template-tests`, openEHR reference archetypes.
  Prefer an existing golden vector over a hand-written fixture. A test that
  encodes a spec rule cites the spec/CNF section it asserts
  (spec-adherence.md).

## Where tests live

Unit tests live beside the code they test (`#[cfg(test)] mod tests` in the
same file). Integration and cross-crate tests live in the owning crate's
`tests/` directory (`crates/*/tests/`, `app/*/tests/`, `tools/*/tests/`).
Do not invent a third location.

## Target

Full-ECC green is the standing bar (claimed at B6: CORE + STANDARD PASS) —
every change preserves it; the baseline only ratchets upward. Every phase
ships compiling, clippy-clean, tested increments (ADR-006 retired the old
"phases need not compile" gate) — this whole rule is fully active at all
times.
