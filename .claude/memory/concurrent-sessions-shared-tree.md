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
- **Explicit `git add <paths>` is NOT enough** (bitten 3× on 2026-07-14):
  another session/agent may have pre-staged its own files, and a plain
  `git commit` sweeps the whole index. Commit with an explicit pathspec —
  `git commit -m "…" -- <paths>` — which commits ONLY those paths regardless
  of what else sits staged; or run `git diff --cached --name-only` first and
  unstage strangers.
- Scope cargo gates to the crates you touched (`-p`), not `--workspace` — the
  other session may have a broken crate in flight (e.g. `ehrbase-conformance`).
- For multi-file subagent work, use worktree isolation and merge branches back.
- Don't "fix" broken files you didn't touch (e.g. a half-edited test file) —
  they're the other session's work in progress.

**Target-dir history (final state 2026-07-16 — ONE `./target`, no overrides
anywhere):** every isolation scheme tried has been retired after ballooning
the disk: `target-cli` (2026-07-12, 35 GB copy), the fixed agent lanes
`target/agent-t1..t4` + the RustRover `target/ide` override (2026-07-16,
part of a 394 GB fill: 211 GB debug + 140 GB lanes + 40 GB ide). Owner
ruling: the CLI, all subagents, AND the IDE share the single default
`./target`; lock waiting is expected and never answered with a second
target dir; subagents never run cargo in parallel (the orchestrator builds
once at convergence); check `du -sh target` at session start and after any
rewrite-scale change, `cargo clean` above ~30 GB. Never pkill -9 rustc to
"fix" slowness — it corrupts incremental caches. Full discipline in
CLAUDE.md §"Target-dir & warm-build discipline".
