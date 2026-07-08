# Benchmark report — ehrbase-rs

> Generated from a run (never hand-typed). **24 operations across 8 openEHR resource groups**, latency **and** throughput, on the machine below. Latencies are microseconds; the full distribution is shown for every operation, both directions. Methodology: `docs/design/benchmarking.md`.

## 1. Environment

> **Machine:** Apple M2 (8 cores / 8 threads, 16384 MiB RAM) · Darwin 26.5.1 · arm64

| Field | Value |
|---|---|
| Run date | 2026-07-08T07:06:08.724452Z |
| Host name | Rubens-MacBook-Air.local |
| CPU | Apple M2 (8 cores / 8 threads @ 3504 MHz) |
| Memory | 16384 MiB |
| OS | Darwin 26.5.1 (kernel 25.5.0) |
| Arch | arm64 |
| Payload | composition_evaluation_test — 63 KB canonical-JSON composition |
| Harness rev | unknown |
| Workload lock | `d6fe4549fe0a6d26` |
| Warmup / measure / runs | 20 / 100 / 2 |

> Numbers below are valid only for this machine. A report with a different **Machine** line is not directly comparable (design §3.1).

## 2. Coverage overview

Every openEHR REST resource group is exercised. A group is green only if **every** operation in it passed the pre-flight conformance gate (a wrong response is never timed — design §4.1).

| Resource group | Operations | ehrbase-rs | EHRbase Java |
|---|--:|:--:|:--:|
| EHR | 3 | ✓ | — |
| EHR_STATUS | 3 | ✓ | — |
| COMPOSITION | 5 | ✓ | — |
| VERSIONED_COMPOSITION | 3 | ✓ | — |
| CONTRIBUTION | 1 | ✓ | — |
| DIRECTORY | 4 | ✓ | — |
| QUERY | 2 | ✓ | — |
| DEFINITION | 3 | ✓ | — |

## 3. Latency & throughput — per operation

Median (p50) / tail (p99, p99.9) latency in µs, and sustained requests/second, per operation per server. `CoV` is inter-run variance (>0.10 = noisy).

### EHR

| Operation | Server | Gate | p50 | p90 | p99 | p99.9 | max | req/s | CoV |
|---|---|---|--:|--:|--:|--:|--:|--:|--:|
| ehr_create (create EHR) | ehrbase-rs | ✓ 201 | 4783 | 6043 | 8863 | 12447 | 12447 | 197 | 0.05 |
| ehr_get (get EHR by id) | ehrbase-rs | ✓ 200 | 3493 | 4119 | 4991 | 5731 | 5731 | 276 | 0.00 |
| ehr_get_by_subject (get EHR by subject) | ehrbase-rs | ✓ 200 | 3715 | 4455 | 6667 | 7123 | 7123 | 257 | 0.07 |

### EHR_STATUS

| Operation | Server | Gate | p50 | p90 | p99 | p99.9 | max | req/s | CoV |
|---|---|---|--:|--:|--:|--:|--:|--:|--:|
| ehr_status_get (get EHR_STATUS) | ehrbase-rs | ✓ 200 | 3543 | 4131 | 4983 | 5683 | 5683 | 272 | 0.01 |
| ehr_status_update (update EHR_STATUS) | ehrbase-rs | ✓ 200 | 5243 | 7939 | 14375 | 22991 | 22991 | 171 | 0.14 |
| versioned_ehr_status_get (get versioned EHR_STATUS) | ehrbase-rs | ✓ 200 | 3807 | 5355 | 14599 | 17759 | 17759 | 232 | 0.21 |

### COMPOSITION

| Operation | Server | Gate | p50 | p90 | p99 | p99.9 | max | req/s | CoV |
|---|---|---|--:|--:|--:|--:|--:|--:|--:|
| composition_create (create composition) | ehrbase-rs | ✓ 201 | 7555 | 9807 | 15031 | 37343 | 37343 | 123 | 0.02 |
| composition_get (get composition) | ehrbase-rs | ✓ 200 | 4167 | 5027 | 8447 | 8943 | 8943 | 227 | 0.05 |
| composition_update (update composition) | ehrbase-rs | ✓ 200 | 6719 | 8407 | 12087 | 15399 | 15399 | 140 | 0.07 |
| composition_delete (delete composition) | ehrbase-rs | ✓ 204 | 3143 | 3535 | 4035 | 4815 | 4815 | 312 | 0.02 |
| composition_get_at_time (get composition at time) | ehrbase-rs | ✓ 200 | 4003 | 4495 | 5147 | 5631 | 5631 | 244 | 0.01 |

### VERSIONED_COMPOSITION

| Operation | Server | Gate | p50 | p90 | p99 | p99.9 | max | req/s | CoV |
|---|---|---|--:|--:|--:|--:|--:|--:|--:|
| versioned_composition_get (get versioned composition) | ehrbase-rs | ✓ 200 | 3905 | 4615 | 5411 | 5875 | 5875 | 247 | 0.01 |
| versioned_composition_revision_history (composition revision history) | ehrbase-rs | ✓ 200 | 3143 | 3573 | 4451 | 5059 | 5059 | 307 | 0.02 |
| versioned_composition_version_by_id (get composition version by id) | ehrbase-rs | ✓ 200 | 4087 | 4779 | 7631 | 10255 | 10255 | 232 | 0.05 |

### CONTRIBUTION

| Operation | Server | Gate | p50 | p90 | p99 | p99.9 | max | req/s | CoV |
|---|---|---|--:|--:|--:|--:|--:|--:|--:|
| contribution_get (get contribution) | ehrbase-rs | ✓ 200 | 4075 | 4547 | 5459 | 7479 | 7479 | 238 | 0.01 |

### DIRECTORY

| Operation | Server | Gate | p50 | p90 | p99 | p99.9 | max | req/s | CoV |
|---|---|---|--:|--:|--:|--:|--:|--:|--:|
| directory_create (create directory) | ehrbase-rs | ✓ 201 | 3409 | 3709 | 4379 | 5119 | 5119 | 288 | 0.01 |
| directory_get (get directory) | ehrbase-rs | ✓ 200 | 3337 | 3841 | 5391 | 6423 | 6423 | 286 | 0.03 |
| directory_update (update directory) | ehrbase-rs | ✓ 200 | 4107 | 4755 | 5827 | 7423 | 7423 | 236 | 0.06 |
| directory_delete (delete directory) | ehrbase-rs | ✓ 204 | 4135 | 5495 | 6587 | 8167 | 8167 | 227 | 0.13 |

### QUERY

| Operation | Server | Gate | p50 | p90 | p99 | p99.9 | max | req/s | CoV |
|---|---|---|--:|--:|--:|--:|--:|--:|--:|
| aql_simple (AQL: SELECT compositions) | ehrbase-rs | ✓ 200 | 11839 | 12991 | 15031 | 16719 | 16719 | 83 | 0.00 |
| aql_aggregate (AQL: COUNT aggregate) | ehrbase-rs | ✓ 200 | 3481 | 3875 | 5047 | 5107 | 5107 | 281 | 0.02 |

### DEFINITION

| Operation | Server | Gate | p50 | p90 | p99 | p99.9 | max | req/s | CoV |
|---|---|---|--:|--:|--:|--:|--:|--:|--:|
| template_upload (upload OPT template) | ehrbase-rs | ✓ 409 | 4367 | 4895 | 5403 | 5559 | 5559 | 227 | 0.01 |
| template_list (list templates) | ehrbase-rs | ✓ 200 | 3145 | 3593 | 4283 | 4479 | 4479 | 311 | 0.03 |
| template_get (get template) | ehrbase-rs | ✓ 200 | 3477 | 3755 | 4107 | 4991 | 4991 | 283 | 0.01 |

## 5. Where EHRbase wins

_No comparison run: EHRbase (Java) was not benchmarked here (single-target run). This section is mandatory in a comparative report; run `docker/benchmark/run.sh` for the head-to-head._

## 6. Methodology & limitations

- **Closed-loop, single-client latency + sustained throughput** per operation; the open-loop concurrency sweep and the empty→1M scale ladder (design §2.2–§2.3) are separate profiles.
- **No EHRbase Java comparison in this run** — a `X× faster` claim needs the head-to-head run with config parity (§3).
- **Warmup discarded, applied identically to both** — the JVM is warmed, not handicapped (§4.2).
- **Inter-run variance reported** (CoV); a difference inside the noise band is not a result (§4.4).
- Numbers depend on host, container resource pinning, and PostgreSQL config — recorded above; comparison is valid only within the same environment.
