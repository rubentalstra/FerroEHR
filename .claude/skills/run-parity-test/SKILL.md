---
name: run-parity-test
description: >
  Runs the parity harness (scripts/parity.sh) that drives the Rust server
  and a stock Java EHRbase with identical requests and diffs the responses,
  including the USE_REFERENCE_EHRBASE=1 negative-test gate. Use when the
  user asks to check parity, run the parity harness, or verify a fix against
  stock EHRbase.
allowed-tools: [Read, Bash]
argument-hint: "[--negative-gate] [test-name-filter]"
---

# /run-parity-test

Runs the cross-check that is Stage 1's acceptance bar
(`PORT_MASTER_PLAN.md` Section 15): identical requests against the Rust
server and a stock Java EHRbase, diffed. Target is ≥99% behavioural parity
at the REST surface on Linux x86_64 first.

## Steps

Run **in-session** (no subagents/worktrees).

1. Execute `scripts/parity.sh`, optionally filtered by
   `$1` if the harness supports a test-name filter. Add
   `USE_REFERENCE_EHRBASE=1` when `--negative-gate` is passed, or whenever
   validating a **new** parity test — a parity test is only valid if it
   still fails against stock EHRbase without our fix.
3. **Summarize failures only.** Do not paste full passing output. For each
   failure: the test name, the endpoint/request, and a short diff excerpt
   (request/response), not the full harness log.
4. **Flag invalid parity tests.** If a test passes under
   `USE_REFERENCE_EHRBASE=1` (i.e. it also passes against stock EHRbase),
   flag it — per the negative-test gate (Section 4.5), that test is not
   actually proving anything about our port and needs to be rewritten to
   target real EHRbase-specific behaviour.
5. **Never edit a test to make it pass.** If a failure looks like a bug in
   our port, report it; do not weaken the assertion, skip the test, or edit
   it to route around the bug. That decision belongs to a human or a
   dedicated fix task, never to this skill.
