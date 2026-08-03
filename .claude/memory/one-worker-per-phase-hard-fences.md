---
name: one-worker-per-phase-hard-fences
description: 2026-08-02 incident — concurrent workers collided in tools/openehr-codegen; within a refactor phase run ONE worker at a time with hard file fences; reverts are orchestrator-only
metadata: 
  node_type: memory
  type: feedback
  originSessionId: af8ec1a8-3953-4ae1-a5d1-355a712f597b
  modified: 2026-08-02T11:41:31.245Z
---

Incident 2026-08-02 (foundation phase): round 3 (app-scoped) and the
model-query worker (codegen-scoped) ran concurrently; round 3's RESULT_SET
task led it into the emitter — the crate the other worker was editing — and
an owner-ordered revert then unwound shared files under live work. Recovery:
freeze both, tar-backup all dirt to the scratchpad, reset uncommitted state
to HEAD, redo sequentially. Nothing committed was lost.

**Rules now standing:**
- Within a refactor phase, ONE implementation worker at a time — the
  max-2 cap is an upper bound for genuinely disjoint work (e.g. catalogue
  vs app), never for same-phase refactors where scope can bleed.
- Every worker brief carries a HARD FILE FENCE: "touching any path outside
  <list> = STOP and report" — a task that turns out to need an out-of-fence
  change reports the need (it may be another leg's work), never does it.
- REVERTS ARE ORCHESTRATOR-ONLY: a worker never runs git checkout/restore/
  clean on shared state; it reports what should be reverted.
- Before any recovery reset: tar-backup every dirty file to the scratchpad;
  verify committed history intact; keep .claude memory dirt out of resets.
Related: [[concurrent-sessions-shared-tree]], [[max-two-concurrent-workers]].
