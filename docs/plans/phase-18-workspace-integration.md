# Phase 18 — Workspace integration

- Status: not-started (Stage-1 app build, step 10 of 13)
- Consumes: all prior app phases
- Compile required: yes
- Decisions: ADR-006 (this is a final *integration* pass, not a compile rescue)

## Objectives

**Re-scoped by ADR-006.** The Bun-style "make everything compile at the end"
rescue no longer applies — the app layer is built as compiling, tested
increments on top of the already-compiling generated crates. This phase is the
final integration: assemble the `ehrbase` binary (server + CLI), wire config,
delete the last ported-out Java whose Rust counterpart has reached parity, and
confirm the **whole workspace** builds + clippy-clean + `cargo fmt`.

## Preconditions

- [ ] P11/P09/P13/P14/P15/P16/P10/P12/P17 delivered as compiling increments

## Scope

**In:** the `ehrbase` binary crate `main` (server + CLI subcommands, `clap`),
config assembly (`figment`), graceful shutdown (`axum-server`), plugin **stub**
(real plugin system is Stage 2); remove Java files whose Rust replacements are at
parity; `cargo build/clippy/fmt` clean across the workspace.
**Out:** parity testing (P19); optimization (P20); deleting *all* residual Java
(P99).

## Tasks

- [ ] `ehrbase` binary `main` (server + CLI); config; graceful shutdown
- [ ] Plugin subsystem stub (Stage-2 ADR later)
- [ ] Delete Java whose Rust counterpart reached parity
- [ ] `cargo build --workspace` + `clippy --workspace --all-targets` + `fmt --all --check` green

## Exit criteria

- [ ] `cargo run -p ehrbase` starts the full server
- [ ] Whole workspace compiles, clippy-clean, fmt-clean
- [ ] No orphaned Java for already-ported subsystems

## Decisions made this phase

- Plugin system is a Stage-2 concern (stub only here).
