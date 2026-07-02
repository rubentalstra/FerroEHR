---
name: worktree-isolation
description: Each rm-transcriber invocation runs in its own git worktree under .claude/worktrees/<id>/ — reads of the shared checkout succeed but writes must target the worktree copy, and sibling crates may be genuinely empty even when the shared checkout shows them populated.
metadata:
  type: project
---

The `Read`/`Grep` tools can read files from the shared checkout path
(`/Users/rubentalstra/RustroverProjects/ehrbase-rs/crates/...`, no
`.claude/worktrees/...` prefix) even while running inside an isolated
worktree, but `Write`/`Edit` are hard-blocked from that path with the error
"This agent is isolated in the worktree ... Edit the worktree copy of this
file instead of the shared-checkout path." All writes must go through the
worktree-prefixed path
(`.claude/worktrees/<agent-id>/crates/openehr-base/src/...`, etc.) — the
worktree ID appears in the erroring tool's message and in `cwd`.

**Why:** each subagent invocation gets a separate git worktree so parallel
transcription/porting sessions do not clobber each other's uncommitted
state. The shared checkout is a legitimate read-through view of the parent
repo tree, but only the caller's own worktree is a legitimate write target.

**How to apply:**
- Confirm the worktree-prefixed absolute path exists before writing (`ls`
  the worktree directory, not the shared-checkout directory) — do not assume
  a directory or file seen via `Read`/`Grep` on the shared path exists in
  your own worktree too. In one observed session, the shared checkout's
  `crates/openehr-base/src/` already had `identification/` and `resource/`
  subdirectories (presumably from a concurrent or later sibling agent's
  work), but the calling agent's own worktree copy of `openehr-base/src/`
  had only `lib.rs` — a real, load-bearing discrepancy, not a stale cache
  artifact.
- Read precedent files (style reference, e.g. an already-transcribed
  sibling class) from wherever they are visible — the shared checkout is
  fine for this, since you are not writing there. Just do not assume the
  types those precedent files declare (e.g. `Any`, `Numeric`, `Ordered`)
  are actually importable from your own worktree's copy of the same crate;
  check with `find`/`ls` inside the worktree path specifically before
  writing `use` statements that assume they exist.
- When a task instruction gives you an absolute repo-root path (e.g.
  "Repo root: /Users/.../ehrbase-rs"), that is almost certainly the
  *shared* checkout path, not your worktree path — the worktree path is
  given separately as your `cwd`/memory-directory prefix. Translate every
  target file path through the worktree prefix before writing.

See also [[phase-a-forward-references]] for how this interacts with
Phase A's "need not compile" rule when a dependency type genuinely does not
exist yet anywhere.
