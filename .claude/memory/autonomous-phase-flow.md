---
name: autonomous-phase-flow
description: "Standing owner instruction — auto PR+merge each phase and start the next without asking; never branch while finished work sits unmerged"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: df0992bc-dba7-497a-b3b2-8614868c0600
---

Owner standing instruction (2026-07-10): when
a phase finishes — push, create the PR, **merge it right away**, checkout a
fresh branch from main, and **start the next phase immediately** without
waiting for a "yes continue".

**Why:** the owner wants uninterrupted autonomous progression through the
work the tracker holds (the open issues + their milestones; direction themes
live on the public roadmap board).

**Hard ordering rule (owner correction 2026-07-11, angry):** the sequence is
strictly commit → push → **create PR → merge** → `git fetch` → checkout the
next branch **from the updated main**. NEVER cut a new working branch from
main while finished work sits unmerged on a feature branch — the new
branch silently misses that work ("otherwise we are missing data"). If work
was just committed on any working branch, merge it to main first, then
branch. (Branch naming: conventional types `feat/…`/`fix/…`/`chore/…` per
the CLAUDE.md hard rule, 2026-07-19.)

**How to apply:** each phase still closes behind the standing gates
(workspace suites green + zero CNF drift, run centrally via
`scripts/conformance.sh`; tracker-issue acceptance criteria ticked + the PR
declaring `Closes #N`, per root CLAUDE.md §Issue workflow). Only
genuinely new design decisions (spec-silent seams needing a design choice the
owner hasn't made) still warrant an AskUserQuestion — mechanical
continuation never does. Related: [[verify-crate-versions-live]],
[[concurrent-sessions-shared-tree]].
