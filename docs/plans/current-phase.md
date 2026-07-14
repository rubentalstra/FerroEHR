# Current phase

**The roadmap is `docs/blueprint/00-THE-BLUEPRINT.md`** — read it first. It is
the single source of truth for the trajectory toward "first fully
spec-compliant openEHR CDR". This file is the live pointer under it; the
consolidated gap surface is the blueprint §2 (proven foundations + ECC
breakdown + spec-area map).

**Open items live in [`WORKLIST.md`](WORKLIST.md)** — one row per item,
owner-mandated single tracker (2026-07-12).

## Active work — P20 optimization (owner go 2026-07-14)

**P20 — profile-driven optimization** on branch `claude/p20-optimization`,
mission: flip the max-sustained row honestly (owner rule: **no false claims —
measured numbers only**). The working tracker is
[`p20-overhead-checklist.md`](p20-overhead-checklist.md) (32 items, receipts
per item); the phase plan is
[`phase-20-optimization.md`](phase-20-optimization.md).

State (2026-07-14): checklist items 1–18, 20–21, 23–26, 28–32 done; v3.0.1
released. **The definitive instrument-clean pair is measured (v5,
`claude/p20-final-pair`): ehrbase-rs 262.2 req/s (L=26, p99 195 ms) vs
upstream 160.5 req/s (L=16) on the fully-populated workload — the
max-sustained row is flipped, 1.63×.** ECC zero-drift receipt committed
(370·335·0, CORE+STANDARD PASS). Open: the item-19 publication rewrite
(in flight), 22 (group-commit A/B), 27 (knee profiler automation).

## Then, in order

1. **X1 publication** — the measured comparison page (ECC matrix from W-10 +
   the benchmark ladder; per-case upstream failure triage) —
   [`x1-comparison.md`](x1-comparison.md).
2. **P17 interop audit** — SIM-B/SDF transformation-rule audit
   ([`phase-17-flat-structured-ehrscape.md`](phase-17-flat-structured-ehrscape.md));
   interleaves, not conformance-gated.

Every phase still ends with an ECC run showing zero drift; the baseline only
ratchets upward (blueprint §4 rule 4). Closed-phase plan files are pruned
once their close is recorded in `docs/PROGRESS.md` (the W-1/W-3x/W-10/W-11/A1
files were pruned 2026-07-14).

**Read first:** `docs/blueprint/00-THE-BLUEPRINT.md`, then
`docs/ADRs/ADR-011-app-crate-redesign.md` (current app-crate reality) +
`docs/ADRs/ADR-008-greenfield-pg18-storage.md` (own PG18 internals).
