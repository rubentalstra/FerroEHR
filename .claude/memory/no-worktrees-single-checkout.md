---
name: no-worktrees-single-checkout
description: "All work happens in the one main checkout — never a git worktree, so the owner can follow it in a single place"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: dc90e7e8-afc4-4863-a4a2-d90e4bb606e0
  modified: 2026-08-06T07:09:45.080Z
---

Do every change in the main checkout on the current branch. Never create a
git worktree (`git worktree add`), and never pass `isolation: "worktree"` to a
subagent.

**Why:** the owner follows the work by watching one working directory. A
worktree hides edits in a second tree they are not looking at, which makes an
autonomous run impossible to review as it happens (owner directive
2026-08-06, after a `docs-dist` prune was done in a temporary worktree).

**How to apply:** with no worktree isolation available, concurrent workers
would collide in the shared tree — so run **one worker at a time** with an
explicit file fence, and keep everything else in the orchestrator's own hands.
This tightens [[one-worker-per-phase-hard-fences]] from "fences" to "fences
plus serialization". A branch that must be built from another ref (a frozen
docs version, an orphan branch) is checked out in place and restored, never
split into a second tree.

See also [[concurrent-sessions-shared-tree]] for the ONE `./target` rule,
which has the same motivation: one tree, one build.

**The `docs-dist` trap (2026-09-04):** a stale `git worktree` at
`./docs-dist` (the frozen-docs branch, gitignored so `git status` stays
clean) sat inside the checkout for weeks and made the IDE show two active
branches; the owner found it very annoying. `docs-dist` is a remote-only
branch the docs pipeline writes; never check it out locally, and run
`git worktree list` when the IDE shows a second branch.
