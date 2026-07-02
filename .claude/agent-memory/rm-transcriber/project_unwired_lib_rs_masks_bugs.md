---
name: project-unwired-lib-rs-masks-bugs
description: openehr-base and openehr-rm's lib.rs files are unwired (no `pub mod` declarations) per ADR-001 §9's Phase-A convention, as of the phase-03-complete baseline (commit cdd68592d / branch develop). This means `cargo check` on these crates only compiles the ~6-line lib.rs doc comment — none of the actual transcribed .rs files are type-checked, and real bugs in them go undetected until wiring happens.
metadata:
  type: project
---

`crates/openehr-base/src/lib.rs` and `crates/openehr-rm/src/lib.rs`, as of
the `develop`/phase-03-complete baseline (commit `cdd68592d`), are just a
handful of doc-comment lines with **no `pub mod` declarations at all**. Per
ADR-001 §9 ("Files stay unwired ... until P17 so the workspace keeps
compiling and CI stays green through Phase A"), this is deliberate — but it
means `cargo check -p openehr-base` / `-p openehr-rm` only type-checks the
literal contents of `lib.rs` itself, not any of the transcribed
`identification/`, `resource/`, `data_types/`, `data_structures/`,
`common/`, etc. files underneath. **"cargo check passes" for these two
crates currently proves nothing about the code inside them.**

**Why this matters:** while adding P4 serde derives (2026-07-02), `cargo
check -p openehr-rm -p openehr-base` reported zero errors throughout, even
though I'd made a real mistake (see [[feedback-serde-type-tag-pitfall]]).
Suspicious of the silence, I copied the `identification` cluster into an
isolated scratch-crate probe and found: (1) my serde mistake immediately,
and (2) a genuine, pre-existing compile bug unrelated to my work —
`ArchetypeId::rm_name()`/`rm_entity()` in
`crates/openehr-base/src/identification/archetype_id.rs` hold `let mut
parts = self.qualified_rm_entity().splitn(...)`, where the temporary
`String` returned by `qualified_rm_entity()` is dropped at the end of the
statement while `parts` (an iterator borrowing it) is used on the next line
— `rustc E0716`. This has been sitting in the tree since it was
transcribed and would fail the moment `identification` gets wired at P17.

**Also found:** a git-topology surprise — this specific worktree's branch
(`worktree-agent-a424e0be9a275d6cc`) forked directly from `develop`/
phase-03-complete, but a **separate branch, `claude/phase-04-serialization-
json`, already contains a full P4 wiring pass** (commit `7138131f6`,
"wire openehr-rm and make it compile ... compile burn-down from ~57 errors
to zero across the 107 transcribed classes"). That branch's `lib.rs` has
`pub mod common; pub mod data_structures; pub mod data_types;` etc. and,
per its own commit message, already had canonical-JSON serde wired for the
whole RM. Multiple sibling `worktree-agent-*` branches exist too —
confirms a multi-agent parallel-worktree setup where several sessions may
be independently re-doing (or extending) the same P4 pass on different,
unmerged branches.

**How to apply:**
- Do not trust `cargo check`/`cargo clippy` passing on `openehr-base` or
  `openehr-rm` as evidence that transcribed-but-unwired code is
  correct — check whether `lib.rs` actually has `pub mod` lines for the
  module you're touching first (`grep "pub mod" crates/openehr-*/src/lib.rs`).
- If it's unwired and you need confidence your edits are real, isolated
  probe crates (copy the relevant `.rs` files into a scratch `Cargo.toml`
  + `main.rs` with the same deps, `cargo run`) work well and cost only a
  few minutes — this is how both findings above surfaced.
- Before starting new work in `openehr-base`/`openehr-rm`, check `git
  branch -a --contains <current HEAD>` / look for `claude/phase-04-*`
  branches to see if a more-integrated branch already exists with
  overlapping work — flag this to the orchestrating process rather than
  silently duplicating or racing it.
