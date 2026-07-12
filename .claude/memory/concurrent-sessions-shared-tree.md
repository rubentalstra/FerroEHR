---
name: concurrent-sessions-shared-tree
description: "The user runs multiple Claude sessions in the same working tree — commit explicit paths only, scope build/test gates, expect branch switches under you"
metadata: 
  node_type: memory
  type: project
  originSessionId: 3d6bd35c-d7ca-4629-bfa9-12bad4751500
---

Observed 2026-07-08: while one session worked, another session in the same
clone committed (`conformance:` commits), switched the branch
(claude/s2-access-control → claude/cnf-hardening), and its `git add` swept the
first session's mid-edit files into its own commit.

**Why:** the user deliberately runs parallel Claude sessions on one checkout;
`git add -A`/branch state is shared and races.

**How to apply:**
- Never `git add -A`/`git commit -a` — stage explicit paths only, and re-check
  `git branch --show-current` immediately before committing.
- Scope cargo gates to the crates you touched (`-p`), not `--workspace` — the
  other session may have a broken crate in flight (e.g. `ehrbase-conformance`).
- For multi-file subagent work, use worktree isolation and merge branches back.
- Don't "fix" broken files you didn't touch (e.g. a half-edited test file) —
  they're the other session's work in progress.

**RustRover lock contention (diagnosed 2026-07-11, fix inverted 2026-07-12):**
the IDE's cargo check shared `target/` and invalidated CLI artifacts on every
save (recompile ping-pong). The first fix — CLI on `target-cli` — doubled the
disk (35 GB copy) and was retired 2026-07-12: `target-cli` is deleted, do NOT
recreate it. Current scheme: the CLI keeps `./target`; the IDE is the one
isolated (RustRover Cargo settings → env `CARGO_TARGET_DIR=<ABSOLUTE
repo path>/target/ide` — absolute, never relative: a relative value
resolves against cargo's per-crate cwd and sprouts nested `target/` dirs
inside crates, observed 2026-07-12).
Never pkill -9 rustc to "fix" slowness — it corrupts incremental caches and
makes it worse. Full discipline (fixed agent lanes target/agent-t1..t4,
clean-at->30GB hygiene) lives in CLAUDE.md §"Target-dir & warm-build
discipline".
