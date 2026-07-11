# Current phase

**The roadmap is `docs/blueprint/00-THE-BLUEPRINT.md`** — read it first. It is
the single source of truth for the trajectory toward "first fully
spec-compliant openEHR CDR". This file is the live pointer under it; the
consolidated gap surface is the blueprint §2 (proven foundations + ECC
breakdown + spec-area map).

## Active work — W1: public documentation website

**Phase file: `docs/plans/w1-docs-website.md`** (branch `claude/w1-docs-website`).
mdBook (Rust-only toolchain, owner ruling 2026-07-11) on GitHub Pages: landing
page + release-versioned book + static OpenAPI endpoint reference generated
from the vendored ITS-REST contract, with drift gated in CI.

Context: the blueprint build arc (B1–B8) and the enterprise capabilities
(E1–E5) are **closed** (E5 2026-07-11, PR #48). Full conformance holds:
**ECC 341 executed · 315 passed · 0 failed — CORE PASS / STANDARD PASS**. The
docs cleanup landed 2026-07-11 (PR #51 — 45 historical files pruned; everything
flows through the blueprint + ADRs + `docs/architecture.md`). Per-phase record:
`docs/PROGRESS.md`.

## Priority order (from the blueprint build order, §3)

Remaining engineering work under the blueprint:

1. **P20 — optimization**: PG18 AIO tuning, hot-read pipelining, `JSON_TABLE`
   codegen (`docs/plans/phase-20-optimization.md`).
2. **P99 — cutover**: final docs pass, tag the first release
   (`docs/plans/phase-99-cutover.md`).

Every phase still ends with an ECC run showing zero drift; the baseline only
ratchets upward (blueprint §4 rule 4). The SM Platform Service Model surface is
complete (SM-1..SM-6); its component map lives in `docs/architecture.md` and the
vendored SM spec is at `docs/specs/openehr/SM/`.

**Read first:** `docs/blueprint/00-THE-BLUEPRINT.md`, then
`docs/ADRs/ADR-011-app-crate-redesign.md` (current app-crate reality) +
`docs/ADRs/ADR-008-greenfield-pg18-storage.md` (own PG18 internals).
