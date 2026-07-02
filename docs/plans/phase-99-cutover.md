# Phase 99 — Cutover

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): the entire workspace (Phases 00-19)
- Compile required: yes (final release build)

## Objectives

Close out Stage 1: delete the last remaining ported-out Java files and any
residual Maven configuration, finalize documentation, and tag the first
pure-Rust release. This is the last phase of Stage 1; Stage 2 (enterprise
feature restoration) begins only after this phase is done.

## Preconditions

- [ ] Phase 18 done: >=99% parity holds
- [ ] Phase 19 done: optimization pass complete and re-verified against parity

## Scope

In: deleting remaining `.java` files whose Rust counterpart has reached
parity, deleting residual Maven configuration (`pom.xml`, `mvnw`, `mvnw.cmd`,
`.mvn/`) once nothing references it, final `docs/` pass, tagging the release.
Out: any Stage 2 work (enterprise feature restoration) — that begins in a new
phase sequence (`docs/plans/s2-phase-NN-*.md`) after this phase closes.

## Tasks

- [ ] Confirm every `.java` file remaining in the tree has a Rust counterpart that has reached parity per Phase 18
- [ ] Delete all remaining `.java` files across `openehr-rest`, `openehr-ehrbase-compat`, `openehr-server`
- [ ] Delete residual Maven build files (`pom.xml`, `mvnw`, `mvnw.cmd`, `.mvn/`) once no tooling references them
- [ ] Remove the `protect_java.sh` hook's Java-protection logic (or confirm it now no-ops) since there is no Java left to protect
- [ ] Update `PORT_MASTER_PLAN.md` status line and `docs/PROGRESS.md` to reflect Stage 1 completion
- [ ] Do a final pass over `docs/ROSETTA.md` and `docs/PORTING.md` for accuracy against the final code
- [ ] Verify `cargo build --workspace --release`, `cargo nextest run --workspace`, `cargo clippy --workspace --all-targets`, `cargo audit`, `cargo deny check` all pass clean
- [ ] Tag the first pure-Rust release
- [ ] Open the Stage 2 phase-file sequence (`docs/plans/s2-phase-00-*.md`) seeded from the Section 11.1 archaeology output

## Exit criteria

- [ ] No `.java` file remains in the repository
- [ ] No Maven configuration remains in the repository
- [ ] The workspace builds in release mode and all quality gates (`nextest`, `clippy`, `audit`, `deny`) pass
- [ ] A release tag exists marking the first pure-Rust EHRbase port
- [ ] Stage 2 phase-file sequence is opened and ready to begin

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. Do not begin this phase until Phase 18's parity number and
Phase 19's optimization pass are both confirmed stable — cutover is a
one-way door for the Java reference material still living in this repo.
