---
name: unlogged-node-rejected
description: "UNLOGGED node table rejected by owner ruling 2026-08-25 — do not re-propose; measured record on #2698"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 32d068af-12e7-4654-9ece-124240b2367f
  modified: 2026-08-25T20:54:02.609Z
---

The UNLOGGED `node` table lever is REJECTED (owner ruling 2026-08-25, on the
#2698 A/B at a 10k-composition seed): no single-client latency change,
−16%/−11% DB statement work, −84% WAL volume — not worth crash-truncation, a
blocking boot rebuild from `vo_version.body`, and empty `node` on physical
standbys/basebackups. `node` stays LOGGED.

**Why:** the folded commit statement already captured the lever's original
motivation (the once-separate 1.7–3 ms node insert); the commit is
fsync-latency-bound, so less WAL volume does not move p50.

**How to apply:** do not re-propose UNLOGGED (or reduced-durability) storage
for `node` or any primary-tier table; concurrency/WAL-bandwidth arguments were
already weighed in the ruling. Same standing class as [[owner-work-style]]'s
no-verify-cache rule (verify_on_read strict default, no read-path caches).
