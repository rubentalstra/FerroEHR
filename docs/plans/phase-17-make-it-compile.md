# Phase 17 — Make it compile

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): the entire workspace (Phases 00-16)
- Compile required: yes — make-it-compile

## Objectives

Drive `cargo check --workspace` to zero errors. Process leaf crates first
(crates that depend on nothing else internal), then their dependents, per
Section 4.1 Phase B. Compiler-error count is the burn-down metric. This is
the first phase where compilation is actually required across the board —
Phases 01-16 were allowed to capture intent without compiling.

## Preconditions

- [ ] Phases 01-16 have produced Phase-A translations for their scope, even if
      incomplete in places (a `todo!()` is acceptable input to this phase; a
      missing file is not)

## Scope

In: `Cargo.toml` wiring for every crate, import resolution, `todo!()`
resolution where the real value is now available, type errors, trait bound
errors, lifetime errors.
Out: behavioral correctness beyond what the compiler can check (that is
Phase 18's job), new feature work, optimization.

## Tasks

- [ ] Run `cargo check --workspace` and record the baseline error count per crate
- [ ] Fix `openehr-foundation` and `openehr-base` (dependency leaves) to zero errors
- [ ] Fix `openehr-terminology` to zero errors
- [ ] Fix `openehr-rm` to zero errors
- [ ] Fix `openehr-serde`, `openehr-odin`, `openehr-bmm` to zero errors
- [ ] Fix `openehr-adl`, `openehr-flat`, `openehr-aql` to zero errors
- [ ] Fix `openehr-rest`, `openehr-ehrbase-compat` to zero errors
- [ ] Fix `openehr-server` (the binary, depends on everything) to zero errors
- [ ] Resolve remaining `todo!()`s where Phase 01-16 work now makes the real implementation available; leave `// TODO(port):`-annotated stubs where it doesn't
- [ ] Run `cargo clippy --workspace --all-targets` and triage warnings (fix or explicitly allow with justification)
- [ ] Run `cargo fmt --all` across the workspace

## Exit criteria

- [ ] `cargo build --workspace` succeeds with zero errors
- [ ] `cargo clippy --workspace --all-targets` produces no un-triaged warnings
- [ ] `cargo fmt --all --check` passes

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. This phase's whole discipline is leaf-first ordering — resist
the temptation to jump to `openehr-server` first just because that's where
most of the interesting logic lives; its errors will cascade from upstream
crates that aren't fixed yet.
