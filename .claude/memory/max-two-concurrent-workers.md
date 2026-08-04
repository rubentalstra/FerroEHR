---
name: max-two-concurrent-workers
description: Owner cap — never run more than 2 implementation subagents concurrently (token budget)
metadata: 
  node_type: memory
  type: feedback
  originSessionId: cab93ad6-e47a-4b67-8d30-aa7e9da35436
---

Owner ruling (2026-07-12, during W-3f): **max 2 coding/implementation
subagents running at the same time.** Fan-outs proceed in pairs, next pair
launches only when the previous pair finishes.

**Why:** parallel Opus workers burn tokens fast; 12-wide fan-outs are too
expensive.

**How to apply:** queue worker waves of 2. Read-only *audit* fan-outs may run
wider with owner sign-off, but default to asking/pairing for anything heavy. Related: [[owner-work-style]],
[[concurrent-sessions-shared-tree]].
