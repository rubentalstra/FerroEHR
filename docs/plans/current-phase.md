# Current phase

**The roadmap is `docs/blueprint/00-THE-BLUEPRINT.md`** — read it first. It is
the single source of truth for the trajectory toward "first fully
spec-compliant openEHR CDR". This file is the live pointer under it; the
consolidated gap surface is the blueprint §2 (proven foundations + ECC
breakdown + spec-area map).

**Open items live in [`WORKLIST.md`](WORKLIST.md)** — one row per item,
owner-mandated single tracker (2026-07-12).

## Active work — W-1 H1 sweep → W-2 ECC skip elimination → X1 → ADL2 → P20 (re-planned)

**W1 (public documentation website) closed 2026-07-11** — the site is live at
<https://rubentalstra.github.io/ehrbase-rs/> (landing + versioned book
dev · latest · v3.0.0 + offline OpenAPI reference), drift-gated in CI
(`docs/plans/w1-docs-website.md` for the full record).

Next up, in order:

1. **X1 — the honest EHRbase vs EHRbase-rs comparison**
   (`docs/plans/x1-comparison.md`, plan awaiting owner review): run upstream
   EHRbase through the ECC suite (with a fairness adjudication register),
   overhaul `tools/benchmark` (multi-SUT, percentiles, resource footprint),
   publish a measured comparison page on the docs site. Owner rule: **no
   false claims — measured numbers only.**
2. **P20 — optimization** (to be re-planned first; P99 removed 2026-07-12 —
   the release machinery already shipped with v3.0.0).

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
