# Phase 01 — Foundation + Identification (BASE)

- Status: **done** — GENERATED from BMM (ADR-004), not hand-transcribed
- Build order: complete (spec foundation)
- Decisions: ADR-004

## Outcome

BASE 1.3.0 (foundation types + base types + identification) is **generated** by
`openehr-codegen` into `openehr-base` (the old `openehr-foundation` folded in).
Compiles clean, clippy-clean. The MI/covariance/generic hazards this phase was
meant to resolve by hand are decided once, in the emitter.

To change a BASE type: edit the emitter (`crates/openehr-codegen/src/emit.rs`)
or a `*_impl.rs` sibling and regenerate (`cargo run -p openehr-codegen -- emit`)
— never hand-edit a `// @generated` file.

## Verification

`openehr-base` builds + lib-clippy clean; downstream crates resolve BASE types
via `openehr_base::prelude`.
