# Current phase

**The roadmap is `docs/blueprint/00-THE-BLUEPRINT.md`** — read it first. It is
the single source of truth for the trajectory toward "first fully
spec-compliant openEHR CDR". This file is the live pointer under it; the
consolidated gap surface is the blueprint §2 (proven foundations + ECC
breakdown + spec-area map).

**Open items live in [`WORKLIST.md`](WORKLIST.md)** — one row per item,
owner-mandated single tracker (2026-07-12).

## Active work — W-11 benchmark rewrite (PRIO 1, owner 2026-07-13)

**W-10 (conformance framework redesign) closed 2026-07-13** — re-derived
baseline **368/333/0/35, CORE + STANDARD PASS** under the rewritten
multi-SUT instrument; upstream EHRbase 2.34.0 recorded as comparison DATA
(`docs/plans/w10-conformance-redesign.md`, PR #82).

Active now:

1. **W-11 — `tools/benchmark` complete rewrite: the hospital-day stress
   instrument** (`docs/plans/w11-benchmark-rewrite.md`): realistic clinical
   workload (templates + generated data; admissions, observations,
   medication rounds, lab contributions, chart-review reads, corrections,
   discharges), latency percentiles + throughput + CPU/RAM + storage
   footprint, multi-SUT like the ECC runner. Absorbs X1's benchmark half.
   Owner rule: **no false claims — measured numbers only.**

Then, in order:

2. **X1 publication** — the measured comparison page (ECC matrix from W-10 +
   the W-11 benchmark ladder; per-case upstream failure triage).
3. **P20 — optimization** (re-plan first, WORKLIST W-5).

## Priority order (from the blueprint build order, §3)

Remaining engineering work under the blueprint:

1. **P20 — optimization**: PG18 AIO tuning, hot-read pipelining, `JSON_TABLE`
   codegen (`docs/plans/phase-20-optimization.md`).


Every phase still ends with an ECC run showing zero drift; the baseline only
ratchets upward (blueprint §4 rule 4). The SM Platform Service Model surface is
complete (SM-1..SM-6); its component map lives in `docs/architecture.md` and the
vendored SM spec is at `docs/specs/openehr/SM/`.

**Read first:** `docs/blueprint/00-THE-BLUEPRINT.md`, then
`docs/ADRs/ADR-011-app-crate-redesign.md` (current app-crate reality) +
`docs/ADRs/ADR-008-greenfield-pg18-storage.md` (own PG18 internals).
