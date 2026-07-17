---
name: run-conformance
description: >
  Runs the ECC conformance suite (scripts/conformance.sh) — our own
  conformance framework and the acceptance instrument — against the
  Docker-composed server (or any BYO SUT). Use when the user asks to check
  conformance, run the conformance/ECC suite, verify spec compliance, or at
  phase close for the zero-drift gate.
allowed-tools: [Read, Bash]
argument-hint: "[area-or-case-filter | --sut byo --base-url <url>]"
---

# /run-conformance

Runs the **ECC suite** (`tools/conformance`) via `scripts/conformance.sh` —
compose up --build → full catalogue → reports under
`docs/conformance/<sut>/` (our server: `docs/conformance/ehrbase-rs/`).
Rewritten 2026-07-13 for the multi-SUT instrument (w10).

## Ground rules (before touching anything)

- ECC is **our own framework**: own case ids (`ECC-*`), own generated data
  sets, spec-derived expectations (`tools/conformance/CLAUDE.md`). The
  upstream CNF Robot suites are reference text, never the instrument.
- **Never weaken a case to pass.** A failing case against our server is a
  correct instrument outcome; corpus/golden defects go through the
  adjudication registers (`tools/conformance/adjudications/`,
  skip-with-reason) — never through editing the case.
- Profile verdicts (CORE/STANDARD/OPTIONS) are **machine-computed** by the
  runner; never hand-assert them anywhere.

## Steps

1. **Preflight:** Docker available (`docker info`); no conflicting compose
   stack already running. The runner always drives a deployed SUT over HTTP
   — there is no in-process mode (owner ruling).
2. **Run.** Default (our server, from the current tree):
   ```bash
   bash scripts/conformance.sh          # optionally: <area-or-case-filter>
   ```
   Foreign/BYO SUT (the X1 comparison path): `CONF_SUT=ehrbase-java` or
   `--sut byo --base-url <url>` per the runner's CLI — the fairness
   adjudication register applies to foreign SUTs only.
3. **Compare against the committed baseline**
   (`docs/conformance/ehrbase-rs/results.json`): the only permitted delta is
   newly-green cases — **zero drift**. Report:
   executed / passed / failed / adjudicated-skip counts, the machine
   verdicts, and any drift case-by-case.
4. **Diagnose failures** from the case's own citation + schedule trace
   (every ECC case carries them): read the cited
   `docs/specs/openehr/...` section, then fix the **server** — or, for a
   genuine corpus/instrument defect, file an adjudication with its reason.
5. **At phase close:** the ratcheted results + report + badges under
   `docs/conformance/` are committed with the phase.
