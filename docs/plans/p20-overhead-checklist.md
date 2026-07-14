# P20 — the full overhead checklist (owner-mandated, 2026-07-14)

Every per-request cost found by the PRIO-1 investigation (in-repo audit of the
`POST /ehr/{ehr_id}/composition` + `POST /query/aql` hot paths, file:line
receipts verified; web/upstream research appended when it lands). **Nothing
ships as "done" without its gate**: crate tests green, and the honest re-ladder
(T5) is the only source of published numbers. Ranked by expected impact on the
saturation knee.

Bench context the estimates assume: pool=50, signing OFF, shed=256, Basic
`ehrbase:ehrbase` (argon2id m=4096,t=2,p=1), tokio workers = num_cpus.

## The checklist

- [x] **1. Basic-auth KDF per request** — argon2id verify ran inline on the
      async worker for EVERY request, no cache (~1–3 ms memory-hard CPU each;
      ~a core at bench rates AND starves the workers polling DB futures).
      `extensions/access/authn/basic.rs:41`. **DONE a620f7374**: verified-
      credential cache (SHA-256 of the presented header, TTL 60 s
      `verified_cache_ttl_seconds`, 0 disables) + KDF misses on the blocking
      pool. Book config page entry still owed (docs rule) → item 18.
- [ ] **2. Pool hold time per commit** — was 3 pool acquisitions per create
      (pre-read / write tx / post-commit read-back) each held through heavy
      in-PG work; the measured 2→15 s acquire waits are hold-time-bound.
      T3b already folded the pre-checks (3→2). Remaining: item 3 removes the
      read-back acquisition (2→1); items 4/6 shrink the tx hold itself.
- [ ] **3. Redundant post-commit read-back** — the SM create/update path
      re-reads the whole document (fresh pool acquire + vo_version SELECT +
      all-nodes SELECT + O(N) reassemble) only to extract the version uid that
      `Committed` already carries; fully wasted under `Prefer: return=minimal`.
      `service/ehr/composition.rs:63` → `service/ehr/mod.rs:77-81`. Fix: build
      the uid from `Committed`; read back only when representation is
      requested. *(orchestrator, in progress)*
- [ ] **4. GIN index write amplification** — `idx_node_data_gin` (jsonb_ops,
      `0001_baseline.sql:399`) tokenizes EVERY node fragment on EVERY commit:
      ~34–43 rows (vital-signs) to ~160–400 (IPS) GIN insertions per commit,
      plus the `ext.openehr_magnitude` expression index (`:415`) per row, plus
      4 btrees — inside the held write tx (measured node INSERT 2.2 s under
      load). The AQL engine queries via nested-set + promoted columns; verify
      the SQL generator actually emits any GIN-served predicate, then drop the
      GIN (and narrow/drop the magnitude expression index) via migration;
      ECC zero-drift gates it. *(dispatched to a worker)*
- [ ] **5. Dead reassemble when signing is off** — `apply_change` computes
      `served = reassemble(&rows)` (`versioning/change.rs:374,463`) to feed
      the signer, which early-returns when `EHRBASE_SIGNING_ENABLED=false` —
      a full O(N) rebuild + clone built and discarded per commit under the
      benchmark config. Gate the reassemble on `signer.enabled()`.
      *(orchestrator, in progress)*
- [ ] **6. Temporal GiST EXCLUDE probes on `vo_version`** per insert
      (`0001_baseline.sql:252,258`; measured 1.6 s INSERT under load).
      Quantify (T4b) before any move; ADR-008 names the unique-index fallback.
- [ ] **7. sqlx pool config** — `test_before_acquire` left at default true
      (+1 liveness round trip × every acquisition) and `min_connections = 0`
      (cold reopen + `SET search_path` churn). `db/pool.rs:22-34`,
      `db/settings.rs:44`. Fix: `test_before_acquire(false)`,
      `min_connections` = steady floor. *(orchestrator, in progress)*
- [ ] **8. TCP_NODELAY unset** on accepted sockets (`lib.rs:204-217`) — Nagle
      can add up to ~40 ms on small (204/minimal) responses. Set nodelay.
      *(orchestrator, in progress)*
- [ ] **9. AQL parse/plan cache absent** — every ad-hoc query re-parses
      (logos+chumsky), re-types, re-lowers to SQL
      (`service/query/execute.rs:81`). Fix: bounded moka `aql text → lowered
      SQL/IR` cache.
- [ ] **10. ATNA audit double event** — 2 events (op + login "Application
      Activity") built per successful request when auditing is on
      (`system_log/middleware.rs:113-149`); one atomic check when off (bench
      = off, so not a bench factor). Consider suppressing the per-request
      login event by default.
- [ ] **11. `ehr_access::enforce` cold misses** — per-EHR settings cache
      misses hit the DB once per new EHR (`extensions/access/ehr_access.rs:
      201-218`); hospital-day creates many EHRs. Consider negative-caching
      default-open EHRs at creation.
- [ ] **12. event_outbox INSERT per commit** regardless of subscribers
      (`versioning/change.rs:622,669`). Gate on eventing-enabled.
- [ ] **13. Default `max_connections = 10`** (prod foot-gun; bench overrides
      to 50) — raise the default + document sizing (`db/settings.rs:29-31`).
- [ ] **14. AQL per-row subtree reload** — `read_subtree_canonical` is one
      SELECT per candidate row when a CONTAINS-anchored cell reloads
      (`storage/node_repo.rs:155-197`). Batch it / project via promoted
      columns.
- [ ] **15. Validation walk cost is load-bearing but heavy** — RM-invariant +
      terminology passes visit every `_type` node pre-tx (~1.5k visits for
      IPS). Keep (conformance), but re-measure after 1–8; candidates: fuse
      walks, skip terminology pass for nodes without coded values.
- [ ] **16. Workload validity (F8)** — the populated skeletons + the old
      raw-JSON jitter produced 422s (constraint-blind variation): the varier
      is being rewritten constraint-aware in FLAT space (jitter clamped into
      each input's declared range, temporals truncated to the pattern);
      LOCK_SCHEME bumps to v2. *(worker in flight)* T5 re-ladder is blocked
      on this.
- [ ] **17. Upstream/web research findings** — the second research agent
      (sqlx pipelining vs deadpool, PG group commit/commit_delay with
      synchronous_commit ON, WAL tuning, upstream EHRbase write path +
      their PG image's baked-in tuning = possible parity gap, Spring
      Security's auth caching). **Append its numbered findings here when it
      returns; each becomes a checklist item.**
- [ ] **18. Docs owed by the fixes** — book config page: `verified_cache_ttl_
      seconds` (+ any new knobs from 7/13); changelog entries per
      user-visible change (auth cache done; pool defaults, nodelay when they
      land).
- [ ] **19. The honest gate** — full ECC zero-drift run after the batch +
      the T5 fine re-ladder (both SUTs, populated valid workload) → README/
      COMPARISON refreshed with whatever the numbers say. No number is
      claimed from this checklist without it.

## Verified NOT a problem (don't re-chase)

- JSON body parsed exactly once for JSON commits (`negotiate.rs:222`).
- WebTemplate cache is a genuine fast path post-T3a (no per-commit reads).
- `write_contribution` already one CTE (T3b); create takes no advisory lock.
- `reject_duplicate_persistent` early-returns for event compositions.
- Overload shed layer is a stock tower semaphore (cheap).
- PEP pre/post checks early-return for non-Query ops.
