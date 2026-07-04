# Phase 00 — Scaffolding

- Status: **done**
- Build order: complete (foundation, precedes everything)
- Decisions: `PORT_MASTER_PLAN.md §12–16`

## Outcome

The single-root Cargo workspace exists and is green (build/fmt/clippy/deny).
EHRbase's Java was relocated per §9.1 (`git mv`); the `.claude/` harness + hooks
are live; `reference/v1` is pinned at v0.32.0; the archaeology delta is recorded.
Crate naming settled to `openehr-*` (spec, generated) / `ehrbase-*` (application).

## Verification

`cargo build --workspace` succeeds for the scaffolded crates; hooks fire; the
`docs/` tree + phase files exist.
