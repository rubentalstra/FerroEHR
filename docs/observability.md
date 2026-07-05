# Status, health & metrics — Rust-native design (OpenTelemetry + Prometheus)

- **Status:** design (partially realized: `/rest/status`, `/health`,
  `/management/info` shipped in P11; the rest below is the target)
- **Stage:** Stage 1 foundation (health/info/prometheus + core HTTP metrics) →
  fuller actuator parity + OTLP tracing phased into **P20** (optimization &
  observability)
- **Date:** 2026-07-05
- **Reference (prior art, not a port target):** EHRbase → Explore → Status and
  Metrics (`https://docs.ehrbase.org/docs/EHRbase/Explore/Status-And-Metrics`).
  EHRbase exposes these via **Spring Boot Actuator**. We do **not** port
  Actuator; we build the equivalent natively in Rust on the observability stack
  already pinned in the workspace (ADR-006/008).

> **Security warning (inherited):** status/metrics endpoints can leak sensitive
> operational detail (config, versions, internal timings, DB state). They are
> **disabled by default**, access-controlled, and should sit behind TLS and/or a
> private management port in production. This is a first-class part of the design
> below, not an afterthought.

## 1. The Rust-native stack (already in `Cargo.toml`)

| Concern | Crate(s) | Role |
|---|---|---|
| Structured logs + spans | `tracing`, `tracing-subscriber` | request/span instrumentation, runtime log-level control |
| Distributed tracing export | `tracing-opentelemetry`, `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp` | export spans/traces to an OTLP collector (Tempo/Jaeger/Grafana) |
| Metrics facade + registry | `metrics` | `counter!`/`histogram!`/`gauge!` call sites throughout the app |
| Prometheus exposition | `metrics-exporter-prometheus` | render the registry as Prometheus text for `/management/prometheus` |
| HTTP metrics middleware | `axum-prometheus` (or a hand-rolled `tower` layer over `metrics`) | per-route request count / latency / status-class |
| OTLP metrics (optional) | `opentelemetry-otlp` (metrics) | push metrics to an OTLP collector in addition to Prometheus pull |

**OpenTelemetry is the spine:** traces (and optionally metrics) are exported via
**OTLP**; Prometheus is the pull-based metrics surface. `tracing` spans bridge
into OTel through `tracing-opentelemetry`, so one instrumentation layer feeds
both logs and traces.

## 2. Endpoint map (`/management/*`)

Base path configurable (`/management` default), optionally bound on a **separate
port** (an internal management port), and each endpoint independently opt-in.

| Endpoint | Rust source | Notes |
|---|---|---|
| `/management/health` | health aggregator | overall `UP`/`DOWN` from registered indicators |
| `/management/health/liveness` | liveness probe | process is up (K8s liveness) |
| `/management/health/readiness` | readiness probe | can serve traffic — includes DB readiness (`sqlx` pool `ping`) |
| `/management/info` | build/spec info | server version, git sha, pinned openEHR spec versions, PG target |
| `/management/metrics` | `metrics` registry view | actuator-style JSON list of metric names + current values |
| `/management/metrics/{name}` | single-metric view | value + tags for one metric |
| `/management/prometheus` | `metrics-exporter-prometheus` handle | Prometheus text exposition (the scrape target) |
| `/management/env` | config view | **read-only**, secrets redacted (reuses the `Redacted` newtype) |
| `/management/loggers` | `tracing_subscriber` reload handle | view + set log-filter levels at runtime (GET/POST) |

Health indicators are a small registry of `HealthIndicator` trait objects
(`db`, `disk`, …); each returns `UP`/`DOWN` + detail. The **DB indicator** pings
the `sqlx` pool and reports pool stats (size, idle, in-use). `/rest/status`
(shipped in P11) remains the lightweight openEHR-flavoured liveness view; the
`/management` surface is the ops/actuator-style detail.

## 3. What we instrument

- **HTTP:** request count, latency histogram, and in-flight gauge, labelled by
  matched route template (from `MatchedPath`) + method + status class. A `tower`
  layer emitting `metrics` macros (or `axum-prometheus`).
- **Auth:** counters for `401`/`403` by mechanism — enables the "attack
  detection via high authz-error rate" use case the reference calls out.
- **Database:** `sqlx` pool gauges (connections, idle, acquire-wait histogram) —
  surfaces pool exhaustion / slow DB the reference calls out.
- **Domain (as phases land):** compositions created/updated, AQL executions +
  latency, template cache hit/miss (`moka` exposes stats).
- **Runtime:** process/uptime, and tokio metrics where useful.

Spans wrap each request (method, route, status, duration, request-id) and export
via OTLP; the same span data drives structured logs.

## 4. Configuration (Rust-native)

`figment`, `EHRBASE_MANAGEMENT_` / `EHRBASE_OTEL_`-prefixed, in a `ManagementConfig`
folded into `RestConfig`. Names mirror EHRbase's `MANAGEMENT_*` behaviour; keys
are ours. **All endpoints disabled by default** (opt-in), matching EHRbase.

| Setting | Env | Meaning |
|---|---|---|
| per-endpoint access | `EHRBASE_MANAGEMENT_<EP>_ACCESS` | `none` \| `read_only` (opt-in each of env/health/info/metrics/prometheus/loggers) |
| DB health access | `EHRBASE_MANAGEMENT_HEALTH_DB_ACCESS` | `none` \| `read_only` |
| health probes | `EHRBASE_MANAGEMENT_HEALTH_PROBES_ENABLED` | expose liveness/readiness outside K8s |
| web exposure | `EHRBASE_MANAGEMENT_WEB_EXPOSURE` | list of exposed endpoints |
| base path | `EHRBASE_MANAGEMENT_BASE_PATH` | default `/management` |
| server port | `EHRBASE_MANAGEMENT_SERVER_PORT` | bind management router on its own port (internal-only exposure) |
| access level | `EHRBASE_MANAGEMENT_ACCESS_LEVEL` | `admin_only` \| `private` \| `public` (see §5) |
| OTLP endpoint | `EHRBASE_OTEL_OTLP_ENDPOINT` | collector URL; unset → no export |
| service name | `EHRBASE_OTEL_SERVICE_NAME` | resource attribute (default `ehrbase`) |
| trace sampling | `EHRBASE_OTEL_TRACES_SAMPLER_RATIO` | head sampling ratio |

## 5. Security (reuses the P11 auth layer)

The reference defines three access levels; we map them directly onto our
existing `auth` middleware (no new mechanism):

- **`admin_only`** (default) — reuses the **coarse admin-scope gate** already in
  `auth` (`AuthConfig::admin_scope`): the `/management` subtree requires the
  admin scope, exactly like `/admin/*`. This is the Stage-1 seam; full RBAC is
  Stage 2.
- **`private`** — any authenticated principal (Basic or Bearer) may access.
- **`public`** — no authentication (only for genuinely non-sensitive endpoints
  like `liveness`; discouraged for `env`/`metrics`).

`liveness`/`readiness` are typically `public` (orchestrators probe them
unauthenticated) even when the rest of `/management` is `admin_only`. Secrets in
`/management/env` are always redacted regardless of access level. The management
router is mounted **outside** the API auth layer and carries its own
access-level layer, so its policy is independent of the clinical API's.

## 6. Wiring & layout

- Observability init (subscriber + OTLP exporter + Prometheus recorder) lives in
  the `ehrbase` binary's startup (extends the `tracing_subscriber::fmt` init
  already in `main.rs`); the reload handle for `/management/loggers` is created
  there and shared via state.
- The `/management` router is built in `ehrbase-rest` (beside the existing
  `status` module), gated by the access-level layer, optionally bound on the
  management port by the binary.
- Metric call sites use the `metrics` facade so the exporter is swappable
  (Prometheus pull now; OTLP push optional).

## 7. Testing

- **Unit:** health aggregation (`UP` when all indicators up, `DOWN` on any
  down); env redaction; access-level → status (`admin_only` denies non-admin;
  `public` allows anonymous).
- **Integration:** `/management/health` → `200 {status:UP}`;
  `/management/prometheus` → `200` Prometheus text containing our HTTP metric
  names after driving a request; `/management/env` redacts secrets; DB readiness
  flips to `DOWN` when the pool is unavailable (testcontainer stopped).
- **OTLP:** export against a mock OTLP collector (or the `opentelemetry` stdout
  exporter) in a focused test; full Grafana/Tempo/Prometheus stack is a manual
  docker-compose, not CI.

## 8. Relationship to what already exists (P11)

P11 shipped the public, always-on basics: `/rest/status` (openEHR liveness +
spec version), `/health`, and `/management/info`. This design extends that into
the full opt-in, access-controlled actuator-equivalent surface with real
OpenTelemetry tracing and a Prometheus metrics registry. The P11 endpoints are
forward-compatible: `/rest/status` stays as the openEHR-flavoured probe, and
`/management/info` gains build/spec detail.
