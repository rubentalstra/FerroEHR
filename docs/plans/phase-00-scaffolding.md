# Phase 00 — Scaffolding

- Status: in-progress
- Started: 2026-07-02   Owner: Ruben
- Consumes (spec/layer): none (bootstrap phase)
- Compile required: yes — must build (empty + moved crates)

## Objectives

Fork EHRbase, convert the Maven reactor into a single root Cargo workspace,
`git mv` the existing Java into the three server crates, stand up the ten
empty openEHR spec crates, generate the `.claude/` agent harness and the
`docs/` tree, pin `reference/v1` as a read-only git ref, and record (not
build) the v1-vs-v2 enterprise-feature archaeology.

## Preconditions

- [x] Fork of `ehrbase/ehrbase` exists and is the working repo
- [x] `PORT_MASTER_PLAN.md` and `CLAUDE.md` are in place at the repo root

## Scope

In: workspace layout, crate skeletons, `git mv` reorganization, `.claude/`
harness, `docs/` tree, git hooks, CI workflows, reference/v1 tag, archaeology
diff (record only).
Out: any actual Java-to-Rust porting or spec transcription — that starts at
Phase 01. No enterprise feature (RBAC etc.) is built here, only catalogued.

## Tasks

- [ ] Place delivered root configs, git hooks, CI workflows, and the porter agent at their target paths; install .githooks via core.hooksPath
- [ ] Point Cargo.toml [workspace.package] repository/authors at the fork
- [ ] Register the agent-harness hooks in .claude/settings.json (attribution guard, java/Maven protection, dangerous-command block, fmt/clippy, phase context, phase gate)
- [ ] Pin reference/v1 read-only ref at v0.32.0 (last pre-v2 tag)
- [ ] Create the 13 workspace crates (10 spec + 3 server) with workspace lints and Section 9 dependency arrows; empty workspace builds
- [ ] Generate the .claude/ harness (rules, skills, agents)
- [ ] Generate the docs/ tree (VERSIONS, PORTING, ROSETTA, LIFETIMES.tsv, PROGRESS, architecture, ADR template, research README, plans/)
- [ ] Run the v1-vs-v2 archaeology diff into docs/enterprise/v1-vs-v2-delta.md (record only; build nothing)
- [ ] git mv the EHRbase Java into the three server crates per Section 9.1; Flyway migrations verbatim to crates/openehr-server/migrations/
- [ ] Remove the Maven-era GitHub workflows and fork-sync pull.yml (replaced by the Rust CI pipeline)
- [ ] Commit the two research dossiers to docs/research/
- [ ] Verify: cargo build --workspace, cargo fmt --all --check, cargo deny check, attribution-guard test commit

## Exit criteria

- [ ] Workspace builds with empty spec crates and relocated Java in place
- [ ] `.claude/` harness and hooks are registered and functioning
- [ ] `docs/` tree is complete per Section 13
- [ ] Archaeology recorded in `docs/enterprise/v1-vs-v2-delta.md`
- [ ] `reference/v1` pinned as a read-only ref
- [ ] CI workflows are Rust-based (no Maven-era workflow remains)

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Root-level configs (CLAUDE.md, PORT_MASTER_PLAN.md, Cargo.toml,
rust-toolchain.toml, deny.toml, rustfmt.toml, CI workflow, hook scripts) have
landed at the repo root but the workspace crates, `.claude/` harness content,
and most of `docs/` (this `docs/plans/` tree aside) do not exist yet, and the
Java tree is still in its original Maven layout awaiting the Section 9.1
`git mv`. Next session: work the phase-00 task list top to bottom, starting
with placing the delivered configs and creating the 13 workspace crates so
`cargo build --workspace` succeeds against an empty-plus-moved tree.
