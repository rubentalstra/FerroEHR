# P20 — the full overhead checklist (owner-mandated, 2026-07-14)

Every per-request cost found by the PRIO-1 investigation (in-repo audit of the
`POST /ehr/{ehr_id}/composition` + `POST /query/aql` hot paths, file:line
receipts verified; web/upstream research appended when it lands). **Nothing
ships as "done" without its gate**: crate tests green, and the honest re-ladder
(T5) is the only source of published numbers. Ranked by expected impact on the
saturation knee.

**Standing owner mandate (2026-07-14): this is a GREENFIELD setup.** Complete
redesigns and rewrites are *preferred* over incremental patches wherever they
remove overhead — up to and including a **total redesign of the table schema**
(the `node`/`vo_version` decomposition itself, fragment granularity, the index
set, partitioning) if the evidence points there. Nothing is deployed; there is
no migration-compatibility debt. The only inviolables are the conformance
gates (ECC zero-drift, RM common master06 versioning semantics, canonical
data fidelity) and honesty (measured numbers only). When the research
findings land, the synthesis evaluates schema-level redesign options
explicitly — not just knob-turning.

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
- [x] **2. Pool hold time per commit** — was 3 pool acquisitions per create
      (pre-read / write tx / post-commit read-back) each held through heavy
      in-PG work; the measured 2→15 s acquire waits are hold-time-bound.
      T3b already folded the pre-checks (3→2). Remaining: item 3 removes the
      read-back acquisition (2→1); items 4/6 shrink the tx hold itself.
- [x] **3. Redundant post-commit read-back** — the SM create/update path
      re-reads the whole document (fresh pool acquire + vo_version SELECT +
      all-nodes SELECT + O(N) reassemble) only to extract the version uid that
      `Committed` already carries; fully wasted under `Prefer: return=minimal`.
      `service/ehr/composition.rs:63` → `service/ehr/mod.rs:77-81`. Fix: build
      the uid from `Committed`; read back only when representation is
      requested. **DONE ec1528bb7** (create = one pool acquisition end to
      end; `Committed` carries `time_committed`).
- [x] **4. GIN index write amplification** — `idx_node_data_gin` (jsonb_ops,
      `0001_baseline.sql:399`) tokenizes EVERY node fragment on EVERY commit:
      ~34–43 rows (vital-signs) to ~160–400 (IPS) GIN insertions per commit,
      plus the `ext.openehr_magnitude` expression index (`:415`) per row, plus
      4 btrees — inside the held write tx (measured node INSERT 2.2 s under
      load). The AQL engine queries via nested-set + promoted columns; verify
      the SQL generator actually emits any GIN-served predicate, then drop the
      GIN (and narrow/drop the magnitude expression index) via migration;
      ECC zero-drift gates it. **DONE 8d9d48027** (usage proof: zero
      GIN-servable operators emitted; both indexes removed from the baseline
      directly per the pre-production rule; 0007/0008/ext-0003 folded in).
- [x] **5. Dead reassemble when signing is off** — `apply_change` computes
      `served = reassemble(&rows)` (`versioning/change.rs:374,463`) to feed
      the signer, which early-returns when `EHRBASE_SIGNING_ENABLED=false` —
      a full O(N) rebuild + clone built and discarded per commit under the
      benchmark config. Gate the reassemble on `signer.enabled()`.
      **DONE ec1528bb7.**
- [x] **6. Temporal GiST EXCLUDE probes on `vo_version`** per insert
      (`0001_baseline.sql:252,258`; measured 1.6 s INSERT under load).
      Quantify (T4b) before any move; ADR-008 names the unique-index fallback.
      **DONE via item 21 (7f39c0fe2)** — removal chosen over quantification
      per the research finding (upstream pays plain btree; GiST exclusion
      serializes concurrent inserts).
- [x] **7. sqlx pool config** — `test_before_acquire` left at default true
      (+1 liveness round trip × every acquisition) and `min_connections = 0`
      (cold reopen + `SET search_path` churn). `db/pool.rs:22-34`,
      `db/settings.rs:44`. Fix: `test_before_acquire(false)`,
      `min_connections` = steady floor. **DONE ec1528bb7** (defaults 20/2,
      no per-checkout ping).
- [x] **8. TCP_NODELAY unset** on accepted sockets (`lib.rs:204-217`) — Nagle
      can add up to ~40 ms on small (204/minimal) responses. Set nodelay.
      **DONE ec1528bb7** (`ListenerExt::tap_io`).
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
- [x] **13. Default `max_connections = 10`** (prod foot-gun; bench overrides
      to 50) — raise the default + document sizing (`db/settings.rs:29-31`).
- [ ] **14. AQL per-row subtree reload** — `read_subtree_canonical` is one
      SELECT per candidate row when a CONTAINS-anchored cell reloads
      (`storage/node_repo.rs:155-197`). Batch it / project via promoted
      columns.
- [ ] **15. Validation walk cost is load-bearing but heavy** — RM-invariant +
      terminology passes visit every `_type` node pre-tx (~1.5k visits for
      IPS). Keep (conformance), but re-measure after 1–8; candidates: fuse
      walks, skip terminology pass for nodes without coded values.
- [x] **16. Workload validity (F8)** — the populated skeletons + the old
      raw-JSON jitter produced 422s (constraint-blind variation): the varier
      is being rewritten constraint-aware in FLAT space (jitter clamped into
      each input's declared range, temporals truncated to the pattern);
      LOCK_SCHEME bumps to v2. **DONE 3827e8f28** (FLAT-space constraint-
      aware jitter, 8 kinds x 100 combos regression net; three surfaced
      findings dispatched to their own worker). T5 unblocked.
- [x] **17. Upstream/web research findings — RETURNED 2026-07-14.** The
      decisive ones (full report in the phase record):
      - **F1 (negative): no PG parity gap.** Upstream ships stock
        `postgres:16.2-alpine` — zero baked-in tuning (Dockerfile_postgres +
        compose fetched verbatim). They beat us on an untuned DB; do not
        chase server config as the explanation.
      - **F2/F13 (negative): auth.** Upstream uses `NoOpPasswordEncoder`
        (plaintext compare). Our argon2 cost is already amortized by the
        verified-credential cache (item 1) — not the knee.
      - **F3/F7 (structural, RANK 1): upstream commits in ~4 SQL round trips**
        (contribution, audit, version, one jOOQ bulk node insert) vs our
        ~9–11, and **sqlx does not pipeline** — every statement is a
        serialized round trip while the tx holds its locks. Levers: keep
        collapsing statements (T3b started), AND move the hot write path to
        a pipelined `tokio-postgres`/`deadpool-postgres` connection
        (CLAUDE.md reserved exactly this). → item 20.
      - **F4/F6 (schema, RANK 2): our `vo_version` carries TWO GiST EXCLUDE
        (WITHOUT OVERLAPS) constraints; upstream's version table is a plain
        btree PK.** GiST exclusion inserts serialize under concurrency —
        matches the measured 1.6 s vo_version INSERT under load. ADR-008
        itself reserved the fallback: btree `UNIQUE (vo_id, sys_version)` +
        app-enforced non-overlap. → item 21 (greenfield mandate applies).
      - **F8 (RANK 3): our own per-commit extensions** — event_outbox INSERT
        (+2 indexes) and the advisory lock (verify per-vo granularity). →
        items 11/12 sharpened.
      - **F10 (RANK 4): group commit** — `commit_delay`/`commit_siblings`
        batch WAL fsyncs with `synchronous_commit` ON (durability intact);
        modest gains; apply to BOTH SUTs. `wal_compression=on` candidate. →
        item 22.
      - **F11 (negative): PG18 AIO (`io_method`) is READ-only** — no effect
        on the write knee; scope it to the AQL read path only.
      - **F12: dynamic sea-query SQL defeats sqlx's per-connection statement
        cache** (each unique AQL re-PREPAREs). Parameterize/canonicalize
        generated SQL + the item-9 plan cache; read-side pipelining lands
        here too.
      - **F14 (minor): per-request tracing spans** — ensure the bench
        profile filters `tower_http::trace`; confirm log level. → item 23.
      - **F15 (negative): our multi-row node INSERT is already right**
        (COPY only wins for very large docs).
- [ ] **20. Pipelined hot write path** (from F7): collapse the remaining
      independent statements per commit AND execute the commit sequence on a
      pipelined `tokio-postgres`/`deadpool-postgres` connection so the
      residual round trips flush together. Versioning semantics byte-equal
      (RM common master06); the signature-over-server-time ordering
      (audit→sign→vo_version) is the one hard sequential dependency.
- [x] **21. `vo_version` GiST → btree redesign** (from F6, greenfield
      mandate): replace both `WITHOUT OVERLAPS` GiST EXCLUDE constraints
      with plain btree uniqueness + application-enforced non-overlap (the
      tx already closes the prior version and inserts the next atomically);
      add an invariant test proving no overlap can be committed. Migration
      re-authors the constraint set; ECC zero-drift + the versioning oracle
      gate it. **DONE 7f39c0fe2** (constraints removed from the baseline;
      one-open-row-per-lineage partial btrees + close-then-insert at one
      now() + the archive-load overlap audit carry the master06 invariant;
      burst-update invariant test added; 490/490).
- [ ] **22. Group-commit tuning A/B** (from F10): `commit_delay` ≈ ½ ×
      pg_test_fsync flush time, `commit_siblings`, `wal_compression=on` —
      applied to BOTH SUTs, measured, `synchronous_commit` stays ON.
- [ ] **24. AQL ehr_id predicate is text-cast + duplicated is_queryable
      guards (LIVE evidence, owner-captured server logs during T5 L=16: the
      patient-dashboard query runs 1.0–1.3 s under load DESPITE the promoted
      ORDER BY)**: the generator lowers `e/ehr_id/value = '…'` as
      `CAST(e0.id AS text) = CAST($n AS text)` — index-blind on `ehr.id`, so
      the join stays unbounded instead of driving the `(ehr_id,
      context_start)` index backward scan. Emit a typed `= $n::uuid`
      comparison when the literal parses as a uuid (a non-uuid literal can
      match no row → constant false). Additionally the EHR-STATUS
      `is_queryable` guard subselect is emitted TWICE (once against
      `n1.ehr_id`, once against `e0.id`) and each scans every current
      EHR_STATUS row per query — dedupe to one guard and give it an
      index-served shape. Generator work (orchestrator) queued behind the
      in-flight item-9 agent to avoid a file collision.
- [ ] **23. Bench-profile tracing filter** (from F14): confirm the composed
      server's log level; filter `tower_http::trace` spans out of the
      benchmark profile if enabled.
- [x] **18. Docs owed by the fixes** — book config page: `verified_cache_ttl_
      seconds` (+ any new knobs from 7/13); changelog entries per
      user-visible change (auth cache done; pool defaults, nodelay when they
      land).
- [ ] **19. The honest gate** — full ECC zero-drift run after the batch +
      the T5 fine re-ladder (both SUTs, populated valid workload) → README/
      COMPARISON refreshed with whatever the numbers say. No number is
      claimed from this checklist without it.

- [ ] **25. Benchmark reporting: RPS + TPM (owner, 2026-07-14).** Not a
      rewrite — a reporting addition. (a) Dual-unit display: every published
      throughput figure shows requests/minute beside requests/second (same
      measurement, friendlier unit). (b) The true TPC-style metric:
      **clinical events completed per minute** — an event (admission, med
      round, lab batch, discharge…) is a multi-request business transaction;
      the driver counts an event completed only when ALL its steps
      succeeded, and REPORT/COMPARISON/knee tables carry events/min per
      class + total alongside req/s.

## Considered and deferred

- **Valkey/Redis cache tier (owner question 2026-07-14): NO for the
  single-node setup, YES-later for scale-out.** Everything cacheable is
  in-process moka today (WebTemplates, verified credentials, ehr_access,
  the AQL plan cache) — an in-process hit is sub-µs vs ~0.2–1 ms for a
  network cache GET, and the measured bottleneck is the PG write path,
  which a cache tier cannot help. It becomes the right design at
  multi-instance scale-out (Stage 2): shared verified-credential cache,
  distributed rate limiting, and — the genuinely safe win — caching
  immutable version reads (a committed OBJECT_VERSION_ID never changes, so
  composition-by-version-id caching is invalidation-free by construction).

## Verified NOT a problem (don't re-chase)

- JSON body parsed exactly once for JSON commits (`negotiate.rs:222`).
- WebTemplate cache is a genuine fast path post-T3a (no per-commit reads).
- `write_contribution` already one CTE (T3b); create takes no advisory lock.
- `reject_duplicate_persistent` early-returns for event compositions.
- Overload shed layer is a stock tower semaphore (cheap).
- PEP pre/post checks early-return for non-Query ops.
