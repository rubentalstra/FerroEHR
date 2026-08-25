---
name: caching-replica-safe-only
description: Owner rule — only replica-safe in-process caches; nothing that breaks horizontal scaling; no shared cache service
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 61772e07-7f45-4d64-a77c-e5be26729532
  modified: 2026-08-25T11:46:48.351Z
---

Owner directive (2026-08-25, during the #2698 perf program): never introduce caching that prevents running multiple server replicas ("do never use system caching because then you can't parallelize run and vertical scale it"), and no shared cache service (Valkey/Redis) — a shared-cache GET is its own network round trip, defeating the latency win.

**Why:** FerroEHR must scale horizontally; any per-process state whose correctness depends on cross-replica invalidation makes replicas serve diverging answers.

**How to apply:** An in-process cache is allowed only when it holds a pure derivation of IMMUTABLE data (e.g. the verified-signature cache: a committed version's body+signature never change, so every replica independently converges — no invalidation event exists). A cache over mutable facts (latest version, template store after re-upload) is only acceptable if invalidated through PostgreSQL itself (LISTEN/NOTIFY to evict local entries), never via a second stateful service and never TTL-only for correctness-bearing reads.
