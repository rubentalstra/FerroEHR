---
name: bmm3-on-hold
description: "Owner ruling 2026-08-04 — BMM v3 is ON HOLD (record issue #1920): no further v3 investment; v2.x is the stable generation; hold is tracker-state only, landed v3 code stays"
metadata: 
  node_type: memory
  type: project
  originSessionId: 05904df7-8f63-41af-8762-dd50ffe8db19
  modified: 2026-08-04T20:49:28.010Z
---

Owner ruling 2026-08-04: the BMM v3 generation is ON HOLD — v2.x is the
stable, tool-implemented generation and nothing consumes v3. The record is
issue **#1920** (closes only by owner decision); v3-only issues carry the
`on-hold` label + a native `blocked-by #1920` edge (currently #1917).

**Why:** v3 is an upstream development line; investing there before it
stabilises or a consumer exists is waste the owner explicitly cut.

**How to apply:** never pick up an `on-hold` issue; a NEW v3-only finding
gets `on-hold` + `blocked-by #1920`, no milestone, instead of entering the
worklist. The hold is TRACKER STATE ONLY — no code gating/removal; the
landed v3 emission/navigation/EL work stays tested and green. The v2.x
generation and the ODIN reader (#1910) are untouched by the hold.
Related: [[owner-work-style]].
