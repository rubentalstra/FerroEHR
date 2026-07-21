# Benchmark comparison stack — ehrbase-rs vs. EHRbase (Java)

The dual-stack environment for `docs/design/benchmarking.md`. Two servers, each
with its own PostgreSQL, on distinct ports; both configured with HTTP Basic auth
(`ehrbase` / `ehrbase`) so the harness drives them through the identical
credential path.

| Stack | Server port | Image (override with env)                                              |
|---|---|------------------------------------------------------------------------|
| ehrbase-rs | `8090` | `EHRBASE_RS_IMAGE` (default `ghcr.io/rubentalstra/ehrbase-rs:develop`) |
| EHRbase Java | `8091` | `EHRBASE_JAVA_IMAGE` (default `ehrbase/ehrbase:2.34.0`)                |

## Run

```shell
# Full comparison (honest one-at-a-time protocol, §3.1):
docker/benchmark/run.sh                 # ≥5 runs per scenario — publishable
docker/benchmark/run.sh --smoke         # fast, proves the pipeline
docker/benchmark/run.sh --only rs       # just ehrbase-rs
docker/benchmark/run.sh --scenario W2   # one scenario
```

The runner brings each stack up alone, waits for its `/rest/status`, benchmarks
it, and takes it down — so neither server contends with the other for host
resources. Results land in `docs/benchmarks/REPORT.md` + `results.json`, with
the **host machine auto-captured** (a number is not comparable across hardware).

## Honesty notes (see the design)

- **PG-version confound** (§3.3): ehrbase-rs runs on PG 18, EHRbase Java on the
  PG 16 its image ships. This is the "recommended" deployment comparison. To
  isolate the engine from the database, add a controlled run with both on PG 16
  (a follow-up; the design describes it).
- **JVM warmup** (§4.2): the harness discards a warmup phase, applied identically
  to both — the JVM is warmed, not handicapped. EHRbase Java also needs ~60–90 s
  to *boot* before it is ready; the runner waits for `/rest/status`.
- **Config parity** (§3.4): Basic auth on both, template-overwrite on both,
  default connection pools. Record any residual asymmetry in the report.
- Pin and record the exact image **digests** in the environment block before
  publishing any claim.
