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
- [x] **9. AQL parse/plan cache absent** *(DONE 32347c781 — IR cached on the exact query text, params/paging/scope bind post-cache, terminology-expanding queries never cached so no TTL; F12: sea-query already binds everything, prepared statements reuse)* — every ad-hoc query re-parses
      (logos+chumsky), re-types, re-lowers to SQL
      (`service/query/execute.rs:81`). Fix: bounded moka `aql text → lowered
      SQL/IR` cache.
- [x] **10. ATNA audit double event** *(DONE 4ed4ea2a8 — login record only on a genuine authentication: a verified-credential cache miss)* — 2 events (op + login "Application
      Activity") built per successful request when auditing is on
      (`system_log/middleware.rs:113-149`); one atomic check when off (bench
      = off, so not a bench factor). Consider suppressing the per-request
      login event by default.
- [x] **11. `ehr_access::enforce` cold misses** *(DONE 4ed4ea2a8 — create pre-warms default-open; import paths now evict — a latent security gap closed)* — per-EHR settings cache
      misses hit the DB once per new EHR (`extensions/access/ehr_access.rs:
      201-218`); hospital-day creates many EHRs. Consider negative-caching
      default-open EHRs at creation.
- [x] **12. event_outbox INSERT per commit** *(DONE 4ed4ea2a8 — boot-time consumer gate; at-least-once preserved for enabled-but-idle; book-page note owed at PR)* regardless of subscribers
      (`versioning/change.rs:622,669`). Gate on eventing-enabled.
- [x] **13. Default `max_connections = 10`** (prod foot-gun; bench overrides
      to 50) — raise the default + document sizing (`db/settings.rs:29-31`).
- [x] **14. AQL per-row subtree reload** *(DONE — one unnest-array interval join per page; byte-identical oracle test)* — `read_subtree_canonical` is one
      SELECT per candidate row when a CONTAINS-anchored cell reloads
      (`storage/node_repo.rs:155-197`). Batch it / project via promoted
      columns.
- [x] **15. Validation walk cost is load-bearing but heavy** *(MEASURED — the deliverable: RM-invariant pass ~109 ms/IPS-commit vs terminology ~3.5 ms; prescribed fusions already satisfied/immaterial; the real hotspot became item 30)* — RM-invariant +
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
- [x] **20. Pipelined hot write path** (from F7): collapse the remaining
      independent statements per commit AND execute the commit sequence on a
      pipelined `tokio-postgres`/`deadpool-postgres` connection so the
      residual round trips flush together. Versioning semantics byte-equal
      (RM common master06); the signature-over-server-time ordering
      (audit→sign→vo_version) is the one hard sequential dependency.
      **DONE 8c6c6099f** — the sqlx-only CTE fold reaches ~4 round trips per
      create (upstream parity) when the signature has no DB dependency;
      signing-on keeps the split; the close stays a separate ordered
      statement (item 21's one-open-row indexes); the second-driver option
      evaluated and skipped with recorded reasoning (residual = WAL fsync +
      index maintenance, un-pipelinable).
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
- [x] **24. AQL ehr_id predicate is text-cast + duplicated is_queryable
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
      in-flight item-9 agent to avoid a file collision. **DONE 9ffd16abd**
      (uuid-typed equality = BASE master05 identifier semantics; one
      correlated EXISTS gate per join-connected component).
- [x] **23. Bench-profile tracing filter** *(DONE — warn on both SUTs via the harness; current pair ran info=info, still parity)* (from F14): confirm the composed
      server's log level; filter `tower_http::trace` spans out of the
      benchmark profile if enabled.
- [x] **18. Docs owed by the fixes** — book config page: `verified_cache_ttl_
      seconds` (+ any new knobs from 7/13); changelog entries per
      user-visible change (auth cache done; pool defaults, nodelay when they
      land).
      *(Done 2026-07-14: ECC 370·335·0 CORE+STANDARD PASS (4cf79bbe0); the
      v5 pair measured — rs 262.2 req/s (L=26, p99 195 ms) vs upstream
      160.5 (L=16), committed f77ac7cfc; README/COMPARISON/roadmap/plan
      pages rewritten numbers-first per the publication mandate; the java
      clean-start re-probe was started for symmetry and STOPPED by owner
      order — the v5 pair stands as published, with the post-storm-rung
      caveat recorded in the pair commit.)*
- [x] **19. The honest gate** — full ECC zero-drift run after the batch +
      the T5 fine re-ladder (both SUTs, populated valid workload) → README/
      COMPARISON refreshed with whatever the numbers say. No number is
      claimed from this checklist without it. *(ECC half done 2026-07-14:
      370·335·0, CORE+STANDARD PASS, committed pre-merge — 4cf79bbe0. The
      pair half is the v4 run in flight.)*
      **Publication mandate (owner, 2026-07-14, verbatim intent):** when the
      pair lands, rewrite ALL the comparison-facing markdown — root README
      (regenerate every SVG it embeds, incl. the new two-SUT knee/overlay
      curves), COMPARISON.md, the benchmark KNEE/REPORT pages, the website
      book pages — as an *appealing read*: the big measured numbers lead,
      short proper statements around them, no walls of text smothering the
      results. Both directions still published; fairness register stays.

- [x] **25. Benchmark reporting: RPS + TPM (owner, 2026-07-14).** Not a
      rewrite — a reporting addition. (a) Dual-unit display: every published
      throughput figure shows requests/minute beside requests/second (same
      measurement, friendlier unit). (b) The true TPC-style metric:
      **clinical events completed per minute** — an event (admission, med
      round, lab batch, discharge…) is a multi-request business transaction;
      the driver counts an event completed only when ALL its steps
      succeeded, and REPORT/COMPARISON/knee tables carry events/min per
      class + total alongside req/s. **DONE 1b1162ad6 (25a) + 80c93a475
      (25b — all-steps-success completion rule, warmup by the last step's
      planned send, wins-ledger row).**

- [x] **26. Knee ladder: geometric base + auto-bisection (owner,
      2026-07-14).** The default ladder becomes geometric from L=1
      (1,2,4,8,16,32,64,128) so EVERY SUT traces a real curve on the overlay
      chart (upstream produced a single breached point — no line), and the
      knee runner auto-bisects after the first breach (midpoints between the
      last sustained and the breached step, up to N refinement steps) so the
      knee is precise regardless of where it falls. Script default changes
      only after the in-flight run completes (never edit a running shell
      script). **DONE (bench binary; the harness default-steps env note
      stays as-is — the CLI default now carries the geometric ladder).**

- [ ] **27. Knee runs name their own bottleneck (owner, 2026-07-14).** Pair
      the bisector with the profiler: the knee runner (or the harness around
      it) resets pg_stat_statements before the last-sustained and breached
      steps and dumps the top statements + wait snapshot into the knee
      artefacts, so every knee run identifies WHAT saturated, not just where.
      Until automated: run scripts/profile.sh manually at the bisected knee
      load after each optimization wave (the pre-optimization profile's
      findings are all fixed — the bottleneck has moved and must be
      re-measured before choosing among items 20/22/14/15).

- [ ] **30. RM-invariant pass: per-node deep-clone + typed re-deserialization
      (~109 ms CPU per IPS commit — the largest known remaining per-commit
      cost; measured by item 15).** `openehr_rm::validate::run`
      (crates/openehr-rm/src/validate.rs:343) runs
      `serde_json::from_value::<T>(value.clone())` PER NODE on the live
      tree — every one of ~1,498 invariant calls deep-clones and fully
      typed-deserializes that node's whole subtree (overlapping subtrees →
      O(Σ subtree sizes) blowup). Fix direction: deserialize the typed tree
      ONCE at the root and validate in one pass, or run invariants against
      &Value without per-node full deserialization. Invariant-semantics-
      sensitive: the ECC ArchetypeValidation set + the openehr-flat
      validation suites gate it; hand-written validate.rs (spec-behaviour
      sibling), never the generated files.
      *(Done 2026-07-14: `validation/plan.rs` — `NodeWalk` caches each node's
      parsed relative constraint segments + child/slot sibling-name indices,
      built once by `prepare_walk` in `build_web_template` (fallback build for
      hand-made test nodes, one code path); passes 1+2 get an incremental
      push/pop path buffer materialized only on a recorded violation. Measured
      (IPS, 1,508 nodes, 50 iters, debug): pass-3 walk 4883.6 → ~2025 µs
      (~2.4×, ~2.9 ms/commit); full validate_composition 16.2 → ~14.3 ms.
      Passes 1+2 flat — their residual is `openehr_rm::validate` per-node work
      (item 30's crate), reported honestly. Messages byte-identical:
      openehr-flat 156, benchmark 107, ehrbase 506, clippy/fmt clean.)*
- [x] **31. FLAT walker per-node overhead (owner, 2026-07-14 — the residual
      after item 30).** With the RM-invariant pass's per-node deserialization
      removed (item 30), the remaining validation CPU is the openehr-flat
      walker's own per-node checks and path-string formatting (per-node
      `format!` path assembly, repeated segment parsing, per-check
      allocations). Rewrite the walk for allocation discipline: build paths
      incrementally (push/pop a single buffer, not format-per-node),
      pre-parse constraint paths once per WebTemplate (cacheable on the
      already-cached WebTemplate), and hoist per-node invariant lookups.
      Violation message text/paths stay byte-identical (the 151-test suite +
      ECC ArchetypeValidation pin them); the item-15 measurement harness is
      the before/after instrument. **Sequenced after item 28(b)/(c) lands —
      its agent is writing in openehr-flat now, and F7's fresh sibling
      routing lives in the same validation files.**
- [x] **32. `openehr_rm::validate` per-node residual (found closing item 31,
      2026-07-14 — now the dominant validation cost).** With the walker's
      template-static re-parsing gone (item 31) and the deep-subtree
      deserialization pruned (item 30), passes 1+2 still cost ~11–13 ms of
      the ~14.3 ms IPS debug validate_composition — dominated by
      `openehr_rm::validate::validate_rm_value`'s per-node work: even
      shallow-pruned, every one of ~1,498 nodes is typed-deserialized
      (`serde_json::from_value::<T>`) to run its class invariants. Fix
      direction: run the invariant checks against `&Value` directly (each
      class's `*_impl.rs` invariants mostly read a handful of leaf fields),
      or borrow-deserialize a minimal per-class view — never the full struct.
      Semantics pinned: openehr-rm 186 + openehr-flat 156 + ECC
      ArchetypeValidation; violation messages byte-identical; hand-written
      `validate.rs`/`*_impl.rs` only, never `// @generated` files; the
      item-15 harness is the before/after instrument.
      *(Done 2026-07-14: two-tier dispatch in `openehr-rm/src/validate/fast.rs`
      — a vouch-or-fall-back fast path checks structural conformance directly
      on `&Value` against the generated static RM model (`crate::model`, so the
      field tables regenerate with the spec) and runs the class invariants via
      shared `pub(crate)` cores the typed `Validate` impls now also call (one
      source per message, byte-identical by construction); anything unmodelled
      (DV_INTERVAL/REFERENCE_RANGE/DV_MULTIMEDIA/FEEDER_AUDIT/periodic
      HISTORY/…) falls back to the authoritative typed dispatch. Per-node map
      hashing removed on the hot path (single entry-iteration in the checker +
      linear-scan field access); the flat-side pass-1/2 walkers project their
      per-node probes in one iteration too. Equivalence pinned by corpus tests
      (10,957 corpus nodes, 9,523 fast-handled, + 2,856 per-key mutations —
      fast output == typed output on every one) + an IPS fast-coverage
      regression guard. Measured (IPS, 1,508 nodes, 50 iters, debug, idle
      host): passes 1+2 11–13 ms → **3.94 ms** (pass 1 ~8–9.5 → 3.16, pass 2
      ~1.5–3.5 → 0.72); full validate_composition ~14.3 → **5.95 ms** (pass-3
      walk unchanged ~1.9–2.1). openehr-rm 194 (186+8 new), flat 156,
      ehrbase 506, clippy/fmt clean.)*
- [x] **28. Upstream-422 triage (the final java run: 4/6 populated skeletons
      rejected — publication-blocking).** All 130 errors at java L=1 were
      composition-create 422s; per-skeleton reproduction against a fresh
      upstream stack names three classes:
      (a) **`DV_MULTIMEDIA` `Size_valid` on size=0** (IPS, ereferral) — the
      spec says `size >= 0` (our DV_MULTIMEDIA PORT NOTE already records
      archie's `size > 0` quirk); spec-valid on our side, but examples now
      carry a plausible non-zero size anyway (realism + sidesteps the
      quirk). DONE.
      (b) **coded names on name-differentiated siblings — OURS** (the full
      upstream logs sharpen this): eprescription and gp-data-set both
      differentiate `at0002` sibling ELEMENTs by NAME ('Date written' vs
      'Date discontinued'; 'Global exclusion of adverse reactions' vs
      'Global exclusion statement') with the name constrained as
      `DV_CODED_TEXT`; our generator/from_flat stamps plain `DV_TEXT`
      names, so archie cannot route the sibling at all ("RmObject … not in
      template") and the 'missing 1..1 occurrence' is the SAME defect (the
      unrouted sibling). Fix the name-constraint typing (emit the coded
      name with its code) in openehr-flat — the generator-side twin of F7's
      validator fix. Spec: AOM 1.4 name constraints (`master04`), RM common
      master03 §LOCATABLE.
      (c) **ISM state rubric — OURS**: gp-data-set immunisation ACTION's
      `ism_transition/current_state` carries value 'complete' where the
      openEHR terminology rubric for the state is 'completed' — adjudicate
      against the vendored TERM 3.1.0 assets and fix the pairing.
      Context for publication: upstream is archie/RM 1.1.0-era (the
      VERSIONS.md divergence note) — vintage-quirk rejections like (a) are
      framed with that version context in the fairness register, never as
      bare defects.
      After (b)/(c): regenerate the pack, re-verify all six commit on BOTH
      SUTs, re-run the java ladder for the honest pair. **DONE — 6/6 on
      both (rs 201, upstream 204); plus the duration-bound inclusivity
      chain fixed end to end and the incoherent-coded-name omission rule
      (empirically adjudicated: upstream rejects the template's own fixed
      coded name in every form).**

- [x] **29. Startup ASCII banner (owner, 2026-07-14).** The server greets
      with an ASCII-art banner on boot (like the reference implementation's
      Spring banner): the EHRbase-rs wordmark, the current version
      (`CARGO_PKG_VERSION`), maintainer credit **Ruben Talstra**, the
      project URL, and the load-bearing pins (RM 1.2.0 / ITS-REST /
      PostgreSQL 18) — followed by the existing structured startup logs.
      Crate choice verified LIVE on crates.io (figlet-rs vs a hand-written
      static banner — zero-dependency static is acceptable if the art is
      generated once and committed). **DONE** — hand-committed zero-dependency
      static banner in `app/ehrbase/src/banner.rs` (FIGlet "standard" wordmark
      vendored once; live check: figlet-rs 1.0.0 Apache-2.0 2026-03-12 vs
      text_to_ascii_art 0.1.10 MIT stale 2024 — no runtime dep taken for a
      fixed string). Printed by `main.rs::serve()` on stdout before telemetry
      init; suppressed under `EHRBASE_LOG_FORMAT=json`. Pins on their own lines
      (list), `rs` lowercased in the wordmark per owner. 3 unit tests
      (`test(banner)`) green.

- [ ] **33. The rows upstream still wins (owner PRIO, 2026-07-15 — "find
      every hot path, map it, design the best possible solution").** The
      fresh same-load hour (a07279607) leaves upstream ahead on exactly
      three rows: **dir-update p99 124 ms vs 84 ms** (also p50 38.3 vs
      34.1), **ehr-create p99 67 ms vs 55 ms** (p50 ~tied), and the
      **aql-ward median 36.8 ms vs 28.2 ms** (its p99 we win 76 vs 101).
      Plus upstream's lower p99-at-its-own-knee (32 ms at L=16 vs our
      195 ms at L=26 — different loads, noted). Two investigation agents
      map the full request paths (REST → service → storage → SQL →
      statement count/plans), root-cause each gap against upstream's
      approach as prior art, and deliver redesign proposals; complete
      rewrites preferred where they pay (greenfield mandate). Verification:
      per-class p99/p50 from a fresh hour pair after the fixes.

- [ ] **34. The exhaustive endpoint sweep (owner PRIO, 2026-07-15 — "go
      through ALL paths, find everything, not only a few high ones").**
      Item 33 proved the method: statement-count probing found 6–10 wasted
      round trips per op on paths the benchmark happened to expose
      (discarded post-commit re-reads, double-built summaries, repeated
      slot JOINs, meta overfetch, per-read attestation SELECTs). Now apply
      it to the WHOLE ITS-REST surface (~96 operations incl. demographic,
      contribution, query/stored-query, template/definition, admin, tags,
      versioned-object reads, EhrScape) plus the extension APIs: one probe
      harness drives every operation once against testcontainers PG18 with
      the sqlx tracing counter, producing an op → round-trips table;
      every outlier vs the minimal-necessary count gets the established
      treatments (committed_response, lean meta reads, threaded vo_ids,
      folded/lateral reads) or a redesign. Exit: the full table committed
      (before/after), no op paying discarded or duplicate statements.

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
