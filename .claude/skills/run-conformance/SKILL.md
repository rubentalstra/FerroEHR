---
name: run-conformance
description: >
  Runs the CNF 2.0 conformance pipeline (scripts/conformance.sh) — the
  reference runner over the committed machine-readable catalogue and the
  acceptance instrument — against the Docker-composed server (or any BYO
  SUT). Use when the user asks to check conformance, run the conformance/CNF
  suite, verify spec compliance, or at phase close for the zero-drift gate.
allowed-tools: [Read, Bash]
argument-hint: "[case-id-filter | --sut byo --base-url <url>]"
---

# /run-conformance

Runs the **CNF 2.0 pipeline** (`tools/cnf-runner`) via
`scripts/conformance.sh` — compose up --build on FRESH volumes (the
exclusive-server ground) → the committed catalogue → pure-function verdicts
→ artefacts under `docs/conformance/<sut>/` (our server:
`docs/conformance/ehrbase-rs/`). Rewritten 2026-07-22 for the CNF cutover
(#202; the ECC harness is retired).

## Ground rules (before touching anything)

- The catalogue is **authored from the CNF 2.0 framework** (official
  schedule case ids; spec-text-only expectations; ambiguities through the
  typed register — `tools/cnf-runner/CLAUDE.md`). The upstream Robot suites
  are reference text; their official DATA fixtures live in the corpus as
  provenance-stamped re-adjudications.
- **Never weaken a case to pass.** A failing case against our server is a
  correct instrument outcome; corpus/authoring defects are fixed with spec
  citations or registered as ambiguities — never bent to observed behaviour.
- Profile verdicts (CORE/STANDARD/OPTIONS/SEC-BASIC) are **machine-computed**
  by the verdict pipeline from (statement, results, catalogue, capability
  matrix); never hand-assert them anywhere.

## Steps

1. **Preflight:** Docker available (`docker info`); no conflicting compose
   stack already running. The runner always drives a deployed SUT over HTTP
   — there is no in-process mode (owner ruling).
2. **Run.** Default (our server, from the current tree):
   ```bash
   bash scripts/conformance.sh          # optionally: <case-id-filter>
   ```
   Foreign/BYO SUT: `CONF_SUT=byo CONF_BASE_URL=<url>` (supply
   `CONF_IXIT`/`CONF_STATEMENT` for a non-default party set; credentials via
   the `SUT_*` env variables the ixit references).
   Measured performance stage (hour-plus; exclusive SUT):
   `CONF_PERF_CLASS=POC|S|L|R` (+ `CONF_PERF_HOURS=1|2|4|6|8|12` for an
   extended sustained hold, `CONF_PERF_SKIP_SEED=1` to reuse a prior
   seeding's sidecar corpus index). The step-load STRESS ladder is a
   separate, non-conformance instrument:
   `cargo run -p cnf-runner -- stress --root tools/cnf-runner/artifacts
   --ixit <party>/ixit.json --out docs/conformance/<sut>/stress.json
   [--corpus-class POC] [--skip-seed]` — it writes stress.json only, never
   results.json. The full canonical CLI table lives in
   `tools/cnf-runner/CLAUDE.md`.
3. **Compare against the committed baseline**
   (`docs/conformance/ehrbase-rs/results.json` + `verdicts.json`): the only
   permitted delta is newly-green cases — **zero drift**. Report:
   passed / failed / errored / not-applicable counts, the machine verdicts,
   and any drift case-by-case.
4. **Diagnose failures** from the case's own `spec_refs` + the run record:
   read the cited `docs/specs/openehr/...` section, then fix the **server**
   — or, for a genuine catalogue/corpus defect, fix it with the citation (or
   register the ambiguity). `CNF_DEBUG_EXCHANGES=1` dumps every wire
   exchange during triage.
5. **At phase close:** the ratcheted `results.json`, `verdicts.json`,
   rendered report/statement/certificate and badges under
   `docs/conformance/<sut>/` are committed with the phase.
