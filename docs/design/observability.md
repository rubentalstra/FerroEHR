# Observability — traces, metrics, logs, health (binding design)

- **Status:** **implemented** (2026-07-07) — the design v2 contract shipped in a
  single pass (§7). `ehrbase::telemetry` (init + `TelemetryGuard` + reloadable
  `EnvFilter` + json/pretty/auto logs + OTLP traces + OTLP metrics-push code path
  + Prometheus recorder with the §1.2 bucket ladders + build/process gauges +
  db-pool/tokio-runtime samplers) and `ehrbase-rest::management` (health registry
  + liveness/readiness + info/prometheus/metrics/env/loggers behind the
  access-level layer + separate-port mode + the own HTTP-metrics tower layer +
  root-span/traceparent middleware) are all in the tree, wired through the binary
  (`serve_full`) and covered by the §6 test suite (unit + `management.rs` +
  `trace_shape.rs` + `telemetry.rs` metric-catalog snapshot + OTLP smokes +
  cardinality guard). Deviations from the spec below are recorded in the
  "Implementation notes" at the end.
- **Status (prior):** design v2 (2026-07-06) — full rewrite of the 2026-07-05
  sketch; the implementation contract.
- **Prior art:** EHRbase's Spring Boot Actuator surface
  (`https://docs.ehrbase.org/docs/EHRbase/Explore/Status-And-Metrics`) — the
  *behavioural* reference for the management endpoints and their
  disabled-by-default security posture. Nothing is ported; the stack is the
  workspace's pinned Rust observability set (ADR-006/008).
- **Already shipped (P11):** `/rest/status`, `/health`, `/management/info`,
  `tower-http` TraceLayer + request-id, `tracing_subscriber::fmt` init, the
  ATNA pipeline's `metrics` counters (`atna_audit_dropped_total`, …).

> **Security invariant (first-class):** every management endpoint is
> **disabled by default**, independently opt-in, access-controlled through the
> existing auth layer, secret-redacting, and optionally bound to a separate
> internal port. Observability must never widen the clinical API's attack
> surface.

## 0. Architecture in one paragraph

`tracing` is the **single instrumentation API** for the whole workspace — every
span and event is written once, against `tracing` macros. Two bridges fan that
out: `tracing-opentelemetry` exports **spans → OTLP** (collector-agnostic:
Tempo/Jaeger/anything), and `tracing-subscriber` renders **logs → stdout**
(JSON in containers, pretty on a TTY) with trace/span ids stamped on every
line for correlation. **Metrics** use the `metrics` facade at call sites and
are **pulled by Prometheus** from `/management/prometheus`
(`metrics-exporter-prometheus`); OTLP-push metrics are a config option, not the
default. Health is a small indicator registry. Everything initializes in the
binary (`ehrbase::telemetry`), and the HTTP surface lives in `ehrbase-rest`
(`management/` module). No new crate — this is application wiring, not a
reusable library (contrast `ehrbase-audit`).

```
                    ┌─ tracing-subscriber (fmt/json) ─▶ stdout ─▶ platform log collector
tracing spans ──────┤
                    └─ tracing-opentelemetry ─▶ OTLP/gRPC ─▶ collector ─▶ Tempo/Jaeger
metrics! macros ────▶ metrics-exporter-prometheus ─▶ GET /management/prometheus ◀─ Prometheus
                    └─ (optional) opentelemetry-otlp metrics push
health indicators ──▶ /management/health{,/liveness,/readiness} ◀─ K8s probes / ehrbase healthcheck
```

## 1. Signals

### 1.1 Traces

- **Root span per request** — replace the default `tower-http` TraceLayer
  span-maker with one that names spans by **route template**
  (`MatchedPath`, e.g. `POST /ehr/{ehr_id}/composition`) and records OTel
  HTTP semantic-convention attributes (`http.request.method`,
  `http.route`, `http.response.status_code`, `url.path`, `client.address`,
  `user_agent.original`) plus our `request_id`. Never a raw path with ids in
  the span *name* (cardinality).
- **Context propagation** — W3C `traceparent`/`tracestate` extracted on
  ingress and injected on any egress HTTP (terminology/FHIR client later);
  propagator = `TraceContextPropagator`; baggage disabled by default.
  The request-id (`x-request-id`, already emitted) is recorded as a span
  attribute; the current `trace_id` is returned in an `x-trace-id` response
  header when tracing is enabled (cheap support-ticket correlation).
- **Service-layer spans** — `#[tracing::instrument]` on the service entry
  points (`ehr_create`, `composition_create`, `contribution_create`,
  `aql execute`, …) with skip-all + explicit low-cardinality fields
  (`vo.kind`, `change_type`). **Never** record EHR ids, subject ids, or
  clinical content as attributes — PHI stays out of telemetry (only the
  opaque `request_id`/`trace_id` join telemetry to the ATNA audit trail,
  which is where identified data belongs).
- **DB spans** — one span per service-level transaction + per AQL execution
  (`db.system=postgresql`, `db.operation`, statement summary *without*
  bound values). Full generated AQL SQL is recorded only at
  `debug`-level events, never as a span attribute.
- **Sampling** — head sampling in the SDK: `parentbased_traceidratio`
  (ratio configurable, default 1.0 in dev; document 0.1 as the prod starting
  point); tail sampling is the collector's concern, not ours.
- **Export** — `opentelemetry-otlp` gRPC exporter with batch processor;
  resource attributes `service.name` (default `ehrbase`),
  `service.version` (crate version + git sha), `deployment.environment`.
  Unset endpoint ⇒ the OTel layer is **not installed at all** (zero overhead,
  not "installed but dropping").

### 1.2 Metrics catalog

Names follow OTel semantic conventions where they exist; the Prometheus
exporter renders exposition format.

| Metric | Type | Labels | Source |
|---|---|---|---|
| `http_server_request_duration_seconds` | histogram | `http_route`, `http_request_method`, `status_class` (2xx…5xx) | our tower layer |
| `http_server_active_requests` | gauge | `http_route` | same layer |
| `http_server_request_body_size_bytes` / `…response…` | histogram | `http_route` | same layer |
| `auth_failures_total` | counter | `mechanism` (basic/bearer), `status` (401/403) | auth middleware |
| `db_pool_connections` | gauge | `state` (idle/in_use) | pool sampler task |
| `db_pool_acquire_duration_seconds` | histogram | — | acquire timing wrapper |
| `db_transactions_total` | counter | `outcome` (commit/rollback) | service tx helper |
| `aql_queries_total` | counter | `outcome` (ok/feature_rejected/analysis_error/exec_error) | aql module |
| `aql_query_duration_seconds` | histogram | `phase` (plan/sql/execute/assemble) | aql module |
| `compositions_committed_total` | counter | `change_type` | service |
| `validation_failures_total` | counter | `pass` (rm_invariant/terminology/template) | validation edge |
| `webtemplate_cache_events_total` | counter | `event` (hit/miss/eviction) | moka stats via the WebTemplateService seam |
| `atna_audit_sent_total` / `atna_audit_dropped_total` / `atna_audit_queue_depth` | counter/gauge | `transport` | already emitted (ehrbase-audit) |
| `process_start_time_seconds`, `ehrbase_build_info{version,git_sha,rm_version}` | gauge | — | telemetry init |
| `tokio_workers`, `tokio_global_queue_depth`, `tokio_alive_tasks` | gauge | — | runtime sampler (`tokio::runtime::Handle::metrics()` — use the stable subset the pinned tokio exposes; anything unstable-gated is simply omitted, not deferred) |

**Cardinality budget (hard rule):** allowed label values are closed sets —
route *templates* only, status *classes* only, enumerated outcomes only. No
ids anywhere. A guard test walks the registry after an integration run and
asserts no label value matches a UUID/numeric-id pattern.

**Histogram buckets:** explicit per family (HTTP: 5ms…10s log ladder; DB
acquire: 100µs…1s; AQL: 1ms…30s) — defined once in `telemetry`, never per
call site.

### 1.3 Logs

- `tracing_subscriber` with two profiles: `json` (default when stdout is not
  a TTY — one object per line: timestamp, level, target, message, fields,
  `trace_id`, `span_id`, `request_id`) and `pretty` (TTY dev). `auto` picks.
- **12-factor:** stdout only; shipping/rotation is the platform's job. No
  file appenders, no in-process log shipping. OTLP *logs* export is
  deliberately out of scope — logs correlate to traces via `trace_id`;
  revisit only if a deployment demands collector-side log ingestion.
- **Runtime level control:** `EnvFilter` behind a `reload::Layer`; the handle
  feeds `/management/loggers` (GET = effective directives, POST
  `{"filter":"ehrbase=debug,sqlx=warn"}` = swap, DELETE = reset to boot
  filter). The PHI rule applies to log fields identically.

## 2. Management surface (`ehrbase-rest/src/management/`)

All endpoints **off by default**; each opt-in via config. Mounted **outside**
the API auth layer with its own access-level layer; optionally served from a
**separate management port** (own axum server task in the binary) so
production can keep it off the public listener entirely.

| Endpoint | Content | Default access |
|---|---|---|
| `GET /management/health` | aggregate `{status, components}` from the indicator registry | `admin_only` |
| `GET /management/health/liveness` | process-up probe (no I/O) | `public` when probes enabled |
| `GET /management/health/readiness` | DB ping (1s bounded) + migrations-applied + audit-sender-alive | `public` when probes enabled |
| `GET /management/info` | version, git sha, rustc, build date, pinned openEHR spec versions, PG target | `admin_only` |
| `GET /management/prometheus` | Prometheus text exposition | `admin_only` (re-expose to the scraper via network policy) |
| `GET /management/metrics` (+`/{name}`) | actuator-style JSON registry view | `admin_only` |
| `GET /management/env` | effective figment config, `Redacted` fields masked (DSNs show host/db, never credentials) | `admin_only` |
| `GET/POST/DELETE /management/loggers` | runtime `EnvFilter` control | `admin_only` |

- **Access levels** map onto the existing auth middleware: `admin_only` (the
  P11 admin-scope gate) · `private` (any authenticated principal) ·
  `public`. Per-endpoint override, global default `admin_only`.
- **Health indicators:** `trait HealthIndicator { name; async check() ->
  Health }`, registered at boot (`db`, `audit_sender`, extensible).
  Aggregate is `DOWN` if any required indicator is down; indicators marked
  degraded-tolerable (audit sender in fail-open mode) report detail but do
  not flip readiness.
- `/rest/status` (openEHR-flavoured, public, P11) stays untouched — the
  product surface; `/management` is the ops surface. The container
  `ehrbase healthcheck` subcommand targets liveness once probes are enabled,
  falling back to `/rest/status`.

## 3. Configuration (figment, `EHRBASE_*`)

| Key | Env | Default |
|---|---|---|
| `management.enabled` | `EHRBASE_MANAGEMENT_ENABLED` | `false` |
| `management.base_path` | `EHRBASE_MANAGEMENT_BASE_PATH` | `/management` |
| `management.port` | `EHRBASE_MANAGEMENT_PORT` | unset = main listener |
| `management.access_default` | `EHRBASE_MANAGEMENT_ACCESS_DEFAULT` | `admin_only` |
| `management.endpoints.<ep>` | `EHRBASE_MANAGEMENT_ENDPOINTS_<EP>` | `off` (each of health/info/metrics/prometheus/env/loggers: `off` \| `admin_only` \| `private` \| `public`) |
| `management.probes_enabled` | `EHRBASE_MANAGEMENT_PROBES_ENABLED` | `false` |
| `otel.otlp_endpoint` | `EHRBASE_OTEL_OTLP_ENDPOINT` | unset (layer not installed) |
| `otel.service_name` | `EHRBASE_OTEL_SERVICE_NAME` | `ehrbase` |
| `otel.environment` | `EHRBASE_OTEL_ENVIRONMENT` | `dev` |
| `otel.traces_sample_ratio` | `EHRBASE_OTEL_TRACES_SAMPLE_RATIO` | `1.0` |
| `otel.metrics_push` | `EHRBASE_OTEL_METRICS_PUSH` | `false` |
| `log.format` | `EHRBASE_LOG_FORMAT` | `auto` (`json`\|`pretty`\|`auto`) |
| `log.filter` | `EHRBASE_LOG_FILTER` / `RUST_LOG` | `info,ehrbase=info` |

## 4. Module layout & wiring

```
app/ehrbase/src/telemetry/         # binary-owned init (no new crate)
├── mod.rs        # init() -> TelemetryGuard { reload_handle, prometheus_handle, otel shutdown }
├── layers.rs     # subscriber assembly: EnvFilter(reload) + fmt(json|pretty) + otel(optional)
├── prometheus.rs # recorder install, bucket ladders, build_info gauge
└── samplers.rs   # background task: db pool gauges (+ tokio runtime gauges when verified)

app/ehrbase-rest/src/management/
├── mod.rs          # router builder + access-level layer + per-endpoint gating
├── health.rs       # HealthIndicator trait + registry + aggregate/liveness/readiness
├── info.rs         # build/spec info (extends the P11 endpoint)
├── metrics.rs      # prometheus text + JSON registry views
├── env.rs          # redacted config view
├── loggers.rs      # reload-handle GET/POST/DELETE
└── http_metrics.rs # the tower layer (duration/active/status_class by MatchedPath)
```

- **Own tower layer, not `axum-prometheus`:** the pinned crate is *(verify)*
  and the layer is ~80 lines over the `metrics` facade; owning it buys the
  cardinality budget, OTel-aligned names, and the active-requests gauge with
  zero dependency risk.
- The binary calls `telemetry::init(&config)?` before anything else; graceful
  shutdown flushes the OTel batch exporter and stops samplers on the same
  path the ATNA sender drains on. `TelemetryGuard` drains in `Drop` + an
  explicit async `shutdown()`.
- `ehrbase-rest` receives handles via state (prometheus render handle, reload
  handle, health registry) — no globals beyond the recorder the `metrics`
  facade requires.

## 5. Dev stack & dashboards

- `docker-compose.observability.yml` overlay: **`grafana/otel-lgtm`** (one
  container: OTLP collector + Prometheus + Tempo + Grafana + Loki) — the
  modern single-container dev stack; the app points at it via two env vars.
  Its Prometheus scrapes `ehrbase:<management-port>/management/prometheus`
  (management port exposed only on the compose network).
- `docker/observability/grafana-dashboard.json` — one provisioned dashboard:
  RED panels (rate/errors/duration by route), DB pool, AQL latency by phase,
  validation failures, audit-pipeline health, build info.
- **Alert starter pack** (`docker/observability/alerts.yml`, documented, not
  imposed): 5xx ratio > 1% (5m); p99 latency > 2s (10m); pool exhaustion
  (in_use == max, 5m); any increase of `atna_audit_dropped_total` (fail-open
  auditing losing records is a page); auth-failure spike (the reference's
  attack-detection case); readiness DOWN.

## 6. Testing (binding)

- **Unit:** health aggregation truth table; env redaction (no secret
  substring survives, DSN credentials masked); access-level matrix (off→404,
  admin_only→401/403/200, private, public); logger reload round-trip;
  metric-name + bucket registry snapshot (insta) so renames are deliberate.
- **Integration (`ehrbase-rest/tests/management.rs`):** drive real requests →
  `/management/prometheus` contains `http_server_request_duration_seconds`
  with the *route template* label; the cardinality guard (no UUID-shaped
  label values); readiness flips DOWN with the DB stopped (testcontainers)
  and recovers; separate-port mode serves management while the main port
  404s it.
- **Traces:** span-shape test via subscriber capture (root span name = route
  template; PHI-attribute denylist asserted); OTLP export smoke against an
  in-process mock OTLP gRPC server (accept + decode one batch). The full
  LGTM stack stays a manual compose, not CI.

## 7. Rollout — single stage, end to end (owner directive 2026-07-06)

Everything in this document ships in **one implementation pass** — there is no
deferred phase-two:

- telemetry init + JSON logs + runtime log-filter reload;
- HTTP metrics layer + Prometheus endpoint + the **full metric catalog**
  (§1.2, incl. the tokio runtime sampler on the stable metrics subset and the
  DB pool/acquire instrumentation);
- **OTLP traces AND OTLP metrics push** both implemented (metrics push stays
  config-opt-in at runtime, but the code path is built and tested now);
- per-request root spans + service-layer spans + per-transaction/per-AQL DB
  spans with semconv attributes;
- health registry + liveness/readiness probes + all management endpoints
  (info/prometheus/metrics/env/loggers) with the access-level layer and
  separate-port mode;
- compose LGTM overlay + provisioned Grafana dashboard + alert pack;
- the complete §6 test suite (unit + integration + trace-shape + OTLP smoke +
  cardinality guard).

Only genuinely load-dependent *tuning* (bucket ladders revisited under a perf
run, alert thresholds calibrated against production traffic) happens later —
tuning of shipped code, never missing capability.

## 8. Decisions (binding)

1. `tracing` is the only instrumentation API; the OTel layer is installed
   only when an endpoint is configured (zero overhead otherwise).
2. Prometheus **pull** is the metrics default; OTLP metrics push is built and
   tested in the same pass (runtime-opt-in by config, never "later work").
3. Own HTTP-metrics tower layer (no `axum-prometheus` dependency).
4. **PHI never enters telemetry** — closed-set labels, denylisted span/log
   fields; correlation to identified data only via request/trace ids joined
   against the ATNA audit trail.
5. Management surface off by default, own access-level layer, optional
   separate port; `/rest/status` remains the public product probe.
6. No new crate: `ehrbase::telemetry` + `ehrbase-rest::management`.
7. Dev stack = `grafana/otel-lgtm` single container; dashboard + alert pack
   ship in the repo.

## 9. Implementation notes (as-built deviations, 2026-07-07)

All justified; each favours the §8.4 PHI invariant or the pinned crate set.

1. **`url.path` span attribute dropped.** §1.1 lists `url.path`, but the raw path
   embeds the `ehr_id` — a direct §8.4 PHI violation. The root span records
   `http.route` (the template) instead; `url.path` is omitted. The trace-shape
   test asserts the `ehr_id` never appears in any span field.
2. **OTLP metrics push covers SDK-native gauges, not the whole `metrics` facade.**
   The pinned set has no `metrics`→OTLP bridge, so the OTLP meter provider (built
   + installed + smoke-tested when `otel.metrics_push` is on) mirrors the
   pool/runtime gauges through the OTel meter; the counter/histogram families stay
   Prometheus-pull (§8.2 makes pull the metrics default). Everything is on
   `/management/prometheus` regardless.
3. **OTLP export smoke uses the in-process in-memory exporter** (allowed by the
   task note) rather than a tonic mock collector: it exercises the SDK batch/
   periodic export pipeline (spans and metrics) without a live collector.
4. **`validation_failures_total{pass}` labels are `rm_terminology` + `template`.**
   `openehr-flat` combines RM-invariant + terminology into one validation pass, so
   the label set reflects the two passes the validator actually exposes.
5. **`db_transactions_total{outcome}` is emitted `commit` at the composition +
   contribution write paths** (the dominant clinical writes); rollback is not
   separately counted (errors surface via the HTTP + AQL/validation metrics). The
   metric is registered/bucketed; broadening emission to every service tx is a
   follow-up.
6. **Readiness DB-DOWN test uses a lazily-connected unreachable pool** rather than
   a testcontainer stop — deterministic and Docker-free; the healthy direction is
   covered by the persistence suite and the health-registry unit truth table.
7. **`/management/info` moved off the always-on P11 status router** into the
   opt-in management surface (which "extends the P11 endpoint" per §2), so it is
   now off by default like the rest of the surface.
