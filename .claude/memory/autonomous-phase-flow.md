---
name: autonomous-phase-flow
description: "Standing owner instruction — auto PR+merge each phase and start the next without asking (E1→E5 enterprise stage, 2026-07-10)"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: df0992bc-dba7-497a-b3b2-8614868c0600
---

Owner standing instruction (2026-07-10, during the enterprise E-stage): when
a phase finishes — push, create the PR, **merge it right away**, checkout a
fresh branch from develop, and **start the next phase immediately** without
waiting for a "yes continue".

**Why:** the owner wants uninterrupted autonomous progression through the
roadmap (now the root `ROADMAP.md` — the E1→E5 enterprise arc that
originally motivated this has shipped; docs/enterprise/ was deleted
2026-07-14).

**Hard ordering rule (owner correction 2026-07-11, angry):** the sequence is
strictly commit → push → **create PR → merge** → `git fetch` → checkout the
next branch **from the updated develop**. NEVER cut a new working branch from
develop while finished work sits unmerged on a feature branch — the new
branch silently misses that work ("otherwise we are missing data"). If work
was just committed on any working branch, merge it to develop first, then
branch. (Branch naming: conventional types `feat/…`/`fix/…`/`chore/…` per
the CLAUDE.md hard rule, 2026-07-19 — the `claude/*` scheme is retired.)

**How to apply:** each phase still closes behind the standing gates
(workspace suites green + full ECC zero drift, run centrally via
scripts/conformance.sh; phase file ticked; blueprint/roadmap updated). Only
genuinely new design decisions (spec-silent seams needing a design choice the
owner hasn't made) still warrant an AskUserQuestion — mechanical
continuation never does. Related: [[verify-crate-versions-live]],
[[concurrent-sessions-shared-tree]].
