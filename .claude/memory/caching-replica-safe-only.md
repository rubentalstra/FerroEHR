---
name: caching-replica-safe-only
description: Owner rule — no caching that complicates horizontal scaling; the verify-once signature cache was removed by owner decision, strict per-read verification is the default
metadata:
  type: feedback
---

Owner directives (2026-08-25, during the #2698 perf program): never introduce caching that prevents running multiple server replicas, no shared cache service (Valkey/Redis — a shared-cache GET is its own network round trip), and — the final ruling — the verified-signature cache (`verify_on_read = once`, #2707) was REMOVED the same day at the owner's request: even a replica-safe in-process cache was unwanted ("I do not like the idea of the memory being used"), and `strict` per-read verification is the accepted default, its ~0.5 ms per VERSION read consciously paid.

**Why:** FerroEHR must scale horizontally, and the owner prefers zero cache state on the read-verification path over the latency win; simple beats clever for compliance features.

**How to apply:** Do not re-propose read-path caches for the signing/verification machinery. An in-process cache elsewhere is only acceptable when it holds a pure derivation of IMMUTABLE data or invalidates through PostgreSQL itself (LISTEN/NOTIFY), never via a second stateful service — and even then, ask the owner first when it touches a compliance surface.
