# Phase 99 — Cutover

> **App-crate reality (ADR-011, 2026-07-09):** three app crates —
> `app/{ehrbase, ehrbase-rest, ehrbase-sm}` + `tools/{conformance, benchmark}`
> + `crates/openehr-*`. EHRbase Java is already out of the tree (ADR-008); this
> phase is the release cutover, not a Java-deletion pass.

- Status: not-started (Stage-1 app build, step 13 of 13)
- Consumes: everything
- Decisions: `docs/blueprint/00-THE-BLUEPRINT.md`, ADR-006

## Objectives

Complete the transition to a pure-Rust EHRbase: delete the last remaining
ported-out Java and any residual Maven config, finalize docs, and tag the first
pure-Rust release. After this, Stage 2 (RBAC/authz, plugin system, multi-tenancy)
begins.

## Preconditions

- [ ] P19 parity ≥99%; P20 optimization done

## Scope

**In:** delete residual `.java` + Maven files (`pom.xml`, `mvnw`, `.mvn/`) whose
subsystems are fully ported + at parity; final `README`/docs; version + tag the
first pure-Rust release. **Out:** Stage-2 feature work (separate `s2-phase-*`
files).

## Tasks

- [ ] Delete remaining ported-out Java + Maven config
- [ ] Final docs pass (README, architecture, VERSIONS)
- [ ] Tag the first pure-Rust EHRbase-RS release

## Exit criteria

- [ ] No `.java`/Maven remaining for ported subsystems
- [ ] Release tagged; `cargo build/clippy/test` + parity all green

## Handoff for next session

Stage 2 begins: RBAC/attribute authz (restore the pre-v2 enterprise capability),
the plugin system (ADR needed), and multi-tenancy — much of this already landed
early via the E-arc (see `docs/PROGRESS.md`); the blueprint tracks the rest.
