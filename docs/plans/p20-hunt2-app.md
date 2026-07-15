# P20 hunt-2 — app stratum (checklist item 35 (b) + (e))

The second full hunt, app strata, run 2026-07-15 on branch
`claude/p20-hotpaths` after items 33/34 made the SQL lean
(`d75dd75d4..b71b6199d`). A concurrent agent worked the DB stratum on
testcontainers at the same time — **all wall-clock/CPU-load numbers here are
DIRECTIONAL** (shared 8-core host, competing testcontainers, and Docker Desktop
crashed twice under the combined load and had to be restarted). The **statement
COUNTS in (e) are deterministic and contention-independent** — they are exact.

Two strata, in the order the task set them:

- **(e) the full re-sweep** — the item-34 probe over the whole ITS-REST surface,
  BEFORE→AFTER. This is the map. **Complete.**
- **(b) app CPU/alloc under load** — profile the serving path.
  **Measured, with a hard caveat** (below).

---

## (e) The re-sweep — DONE, zero regressions

Full AFTER table appended to `docs/plans/p20-endpoint-sweep.md` (the item-34
exit artefact). Method reproduced exactly (full `ehrbase_rest::build_with`
router over ONE warm `EhrbaseService::new(pool)`, `tracing` Layer counting
`sqlx::query` events, testcontainers `postgres:18`, config = signing on /
eventing on / audit off / auth off / admin on). One method improvement: the
harness **pre-opens every pool connection in a concurrent burst before
measuring**, so the `after_connect` `SET search_path` never lands in a measured
op — this removed the original sweep's ±1 noise (the read family reproduced
byte-stable run-to-run).

### Headline

**ZERO regressions across the whole surface.** Every op's AFTER count ≤ its
BEFORE count. Finding → evidence → verdict:

| Finding | Evidence | Verdict |
|---|---|---|
| The waves' write reductions all HOLD | `ehr_status_update` 18→14 (repr) / 11 (min); `person_update` 18→10; `composition_update` 17→13 (repr) / 11 (min); `contribution_create` 10→8; `person`/`organisation_create` 9→6; `ehr_get_by_id` 5→4 — each matches or beats its wave target | HOLDS |
| w3's `is_modifiable` fold rippled wider than its commit claimed | `directory_update` 15→13 (target was 14), `directory_delete` 11→9 (target 10), `directory_create` 12→9, `composition_delete` 11→9 | improved beyond target |
| The wave targets quoted as single numbers are the **minimal** path | `composition_update`/`ehr_status_update` measured 11 under `return=minimal` (= w3/target), +2 under `return=representation` for the REST body re-read (DBR) — the benchmark drives minimal | consistent, not a regression |
| Read family unchanged at minimal-necessary | every versioned read 2–3, plain read 1–2, tags 1 — identical to BEFORE | HOLDS |

### What is now worst, and is it waste?

Ranked AFTER (desc): `ehr_status_update` 14/11 · `composition_update` 13/11 ·
`directory_update` 13 · `ehr_create_with_id` 11 · `ehr_create` 10 ·
`person_update` 10 · the 9-cluster (`directory_create`/`directory_delete`/
`composition_delete`/`person_delete`) · the 8s (`contribution_create`,
`composition_create` repr).

- **The one broad removable round-trip left is `EVT`** — the `event_outbox`
  INSERT on *every* write (all 13 write ops), present in the default
  eventing-on config with no subscribers.
  **Evidence:** `app/ehrbase/src/versioning/change.rs` emits it inside the held
  write tx on every commit; item 12 gated the *consumer* at boot but kept the
  INSERT (at-least-once for enabled-but-idle).
  **Proposal:** an eventing-*off* fast path that skips the INSERT when no
  consumer is configured (cross-cutting finding #5 in the sweep). Expected
  impact: −1 round trip inside the write tx on every write; broad but bounded
  (one statement, no server-side work). Not a correctness item.
- **Representation DBR re-read (+2 on updates under `return=representation`):**
  legitimate — the client asked for the body; `minimal` avoids it. Not waste.
- **Read family: no structural waste.** Versioned reads are inherently 2–3
  (version resolution + node reassembly); nothing discarded or duplicated. The
  read family the task flagged as "only grazed by waves 1–3" was already lean.

Conclusion: the ITS-REST surface is lean. Beyond the eventing fast-path, no op
pays a discarded or duplicate statement.

---

## (b) App CPU/alloc under load — measured, IO-bound is the headline

Profiled the real release server (`cargo build --release -p ehrbase`, signing
off, auth off, pool 50, shedding off) against a local `postgres:18` seeded to
the 10k benchmark shape, driven under load and sampled with `/usr/bin/sample`
(macOS built-in — `samply` and `cargo-instruments` are not installed here; the
homebrew `sample` shim on `$PATH` is a different tool, use the absolute path).
Two windows sampled: (1) a mixed commit+read `bench knee` window, (2) a
read-heavy hammer (parallel GET-composition + AQL) chosen because reads are the
app-CPU-bound path (node reassembly + canonical JSON encode) whereas commits are
PG-bound.

### Finding 1 (the headline): the app is NOT CPU-bound — it is IO/PG-await-bound

**Evidence:** in BOTH sample windows the eight `tokio-rt-worker` threads were
overwhelmingly parked. At the knee window: ~100% in `kevent`/`__psynch_cvwait`,
essentially zero serving frames. Under the read hammer: **92.9% of samples idle**
(`__psynch_cvwait` 83.7% + `kevent` 9.2%), only **7.1% on-CPU**.
**Interpretation:** the ehrbase application code is not the saturation
bottleneck — the workers spend their time awaiting Postgres (writes) and the
network/client (reads). This is measured corroboration of the whole item-33/34
+ F3/F7 story: **the limiter is the PG round-trip write path, not app compute.**
**Directional caveat:** the local client could not saturate 8 cores
(`curl`-per-request, no keep-alive), and PG + the sibling agent + Docker
instability competed for cores — so the *absolute* idle % is inflated. The
qualitative conclusion (not CPU-bound) is robust: it reproduced across both
windows and both load shapes.

### Finding 2: within on-CPU serving work, canonical JSON dominates app logic

Self-time buckets, **normalized to on-CPU (non-idle) samples** from the
read-hammer window (leaf-attribution parse of the `sample` call tree):

| bucket | % of on-CPU | note |
|---|---:|---|
| socket syscalls (accept/send/recv/writev) | 17.0% | **inflated** by no-keep-alive client (fresh TCP + `__accept` per request); ~0 with a real keep-alive client |
| axum/hyper/http framework | 16.0% | request routing + response futures |
| **serde_json (canonical JSON codec)** | **14.4%** | de + ser + `Value` drop |
| alloc/free | 12.3% | mostly `serde_json::Value` tree + `Bytes` churn |
| time (`mach_absolute_time`/`clock_gettime`) | 12.1% | tokio timer + metrics timestamps |
| sqlx/pg driver | 6.3% | |
| tokio runtime/futures | 6.2% | |
| **indexmap (preserve_order maps)** | **5.0%** | `IndexMap` clone/insert — canonical JSON's `preserve_order` backing |
| memcpy/memmove | 3.8% | |
| observability/cache bg | 2.1% | prometheus recorder + moka upkeep |
| hashing | 1.7% | |
| canonical (its) | 0.8% | |
| aql engine + sea_query | 1.3% | |
| node codec / reassemble | 0.5% | |
| uuid | 0.1% | |
| validation | 0.0% | write-path only; 0 on reads (correct) |
| TLS / compression | 0.0% | off locally, as expected |

Answering the task's specific questions:

- **Canonical JSON encode/decode: ~19% of on-CPU** (serde_json 14.4% + the
  `preserve_order` IndexMap 5.0%, plus a chunk of the alloc/free from building
  and dropping `serde_json::Value` trees). This is the **single largest
  application-code consumer** on the read path.
- **Node codec decompose/reassemble: ~0.5% — cheap.** Building the RM tree from
  the decomposed node rows is negligible; the cost is turning that tree into
  canonical JSON, not assembling it.
- **REST negotiation / axum extraction: ~16%** framework, but inflated by the
  no-keep-alive client's per-request `__accept`; not representative of a real
  client.
- **uuid/text conversions: ~0.1% — negligible.**
- **TLS/compression: 0** (off locally, confirmed — the trace-visible `ring`
  frames are a rounding artefact).
- **Validation: 0 on reads** (write-path only; measured elsewhere at ~6 ms/commit
  by items 30/32 — not re-profiled here).

### Proposals (honestly bounded)

1. **App-CPU micro-optimization has LOW expected impact on the saturation knee.**
   The knee is PG-bound (finding 1 + items 33/34/F3–F7); shaving serving CPU
   does not move a PG-limited knee. **Verified-not-a-primary-problem: app CPU is
   not the bottleneck.** Do not spend the optimization budget here while the PG
   write path dominates.
2. **The one app-CPU candidate worth recording (do NOT act now):** the read path
   materializes a full `serde_json::Value` tree (with `preserve_order` = IndexMap)
   from the reassembled node fragments and then serializes it — ~19% of on-CPU +
   much of the alloc churn. IF a workload ever becomes read-CPU-bound (very large
   `RESULT_SET`s, or after the write path is fully pipelined and PG stops being
   the limiter), serializing canonical JSON **directly from the stored node
   fragments** (they are already canonical JSON, ADR-008) instead of round-tripping
   through an intermediate `Value` tree would cut most of that ~19% + the
   associated allocations. Bounded upside; premature while PG dominates. Candidate,
   not a fix.
3. **Follow-up to get a saturating CPU profile** (this run could not, for the
   reasons above): drive a **read-only closed-loop** load with a keep-alive HTTP
   client at fixed concurrency ≥ num_cpus against the seeded store, on a host not
   sharing Docker with another agent, and re-sample. That removes the
   `__accept`/idle inflation and gives a clean on-CPU distribution if one is ever
   needed — but per proposal 1, it is not on the critical path.

---

## Verification / hygiene

- Production code untouched (READ-ONLY honoured). The only file written under the
  tree was the throwaway `app/ehrbase/tests/zz35app_sweep.rs`, **deleted** after
  the AFTER table was captured; the local `postgres:18` profiling container was
  removed; all background server/driver processes killed.
- ECC zero-drift + the fresh-hour pair + a knee re-ladder remain the item-35 exit
  gate (unchanged by this read-only hunt).
