---
paths: ["crates/**"]
---

# Testing discipline

Test discipline is non-negotiable (a standing hard rule; see `CLAUDE.md`). It
applies to every crate, generated (ADR-004), hand-written, or ported.

## The hard rule

- **Never** silently weaken, skip, or delete an existing test to make the
  port pass.
- **Never** edit a test to route around a runtime bug it exposes. If a test
  fails and the fix is unclear, leave it failing and record a
  `// TODO(port):` — do not touch the test to make it green.
- Conformance/corpus tests assert the **openEHR specifications** (ADR-008):
  cite the spec clause a test encodes; never adjust an expectation to match
  an implementation bug.

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

## Oracles

The **vendored CNF suite** is the primary acceptance authority
(ADR-008): `docs/specs/openehr/CNF/docs/platform_test_schedule/` (per-endpoint
+ per-data-type test cases) and the executable Robot suites + fixtures
(`.opt` templates, canonical JSON/XML payloads) under
`docs/specs/openehr/CNF/tests/platform/robot/`. Also: openEHR conformance
corpora, EHRbase's `test-data` and `serialisation_conformance_test` sets,
Better's `web-template-tests`, and openEHR reference archetypes. Prefer these
over hand-written fixtures when a golden vector already exists. A test that
encodes a spec rule cites the spec/CNF section it asserts
(spec-adherence.md).

## Where tests live

Unit tests live beside the code they test (`#[cfg(test)] mod tests` in the
same file, mirroring the Java convention of a test class per source class).
Integration and cross-crate tests live in `crates/*/tests/`. Do not invent a
third location.

## Target

openEHR CNF conformance at the REST surface on Linux x86_64 first, then
broaden. Every phase ships compiling, clippy-clean, tested increments
(ADR-006 retired the old "phases need not compile" gate) — this whole rule
is fully active at all times.
