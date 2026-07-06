---
name: run-conformance
description: >
  Runs the openEHR conformance suite (scripts/conformance.sh) — the CNF
  platform test schedule plus the corpus suites — against the Rust server
  (ADR-008 acceptance instrument). Use when the user asks to check
  conformance, run the conformance suite, or verify spec compliance.
allowed-tools: [Read, Bash]
argument-hint: "[api-group-or-test-filter]"
---

# /run-conformance

Runs `scripts/conformance.sh` (built out from P12 smoke coverage to the full
CNF Platform Conformance Test Schedule at P19).

## Steps

1. Confirm `scripts/conformance.sh` exists; if not, report that conformance
   wiring starts at P12 (see `docs/plans/phase-19-conformance-parity.md`).
2. Run it (optionally with the given filter), with the server under test
   built from the current tree.
3. Report failures grouped by API group / test case, with the spec clause
   each failing case asserts. Never weaken a test to pass (testing.md).

## The CNF source of truth (vendored)

The schedule the runner implements is vendored in-repo:
`docs/specs/openehr/CNF/docs/platform_test_schedule/` (the test-case
definitions, per API group + per data type) and
`docs/specs/openehr/CNF/tests/platform/robot/` (the executable upstream Robot
suites + fixtures: `.opt` templates, canonical JSON/XML payloads). When a
conformance failure needs diagnosis — or the runner doesn't cover an area
yet — read the matching test case there and the spec section it cites; fix
the server, never the expectation.
