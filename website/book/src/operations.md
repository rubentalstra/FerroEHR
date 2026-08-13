# Operations

Running a clinical data repository in production means more than starting the
binary: the database must be backed up and least-privileged, traffic must be
encrypted, upgrades must be safe while the service stays up, and you need to
see what the system is doing. This chapter is a production checklist —
database roles, TLS, backup and point-in-time recovery, upgrades and
migrations, observability, the health probes, and the management surface —
drawn from how the container image and Helm chart are built to run. The
operator-facing HTTP surfaces have their own page:
[Admin & messaging APIs](operations-admin-apis.md).

<!-- toc -->

## Database roles and least privilege

FerroEHR connects to an **external** PostgreSQL 18 — a managed service or an
operator-run cluster, never a chart-side sidecar, because a database holding
PHI must be independently backed up and recoverable. The server carries only a
connection string, ideally sourced from a secret.

The database uses a four-role model — never a superuser at runtime:

| Role | Purpose | Used by |
|---|---|---|
| owner | owns the database | provisioning only |
| `ferroehr_migrator` | runs the schema migrations; owns the helper functions | the migration step |
| `ferroehr_app` | reads and writes clinical data | the running server |
| `ferroehr_reader` | read-only | replicas and reporting |

The migrations create these roles idempotently, apply the per-schema grants,
and revoke the ability to create objects in the public schema. **The running
server connects as `ferroehr_app`** — its DSN should authenticate as that role,
not the migrator or the owner.

`ferroehr_app` holds `SELECT`/`INSERT`/`UPDATE`/`DELETE` on the clinical tables
and `EXECUTE` on the `ext` helper functions, and nothing else: it is not a
superuser, does not bypass row-level security, and cannot create, alter or drop
a table, index, schema or role. On the audit trail it is narrower still — it may
record an event and stamp it forwarded, and it holds no privilege that can
rewrite or remove one (see [Audit](audit.md)).

Which posture you actually get depends on `db.migrate`, because the server's
embedded migrations are DDL: a self-migrating deployment necessarily runs as a
role that can execute DDL. The single-container quickstart takes that path — its
DSN authenticates as a non-superuser role that owns the database and is a member
of both `ferroehr_migrator` and `ferroehr_app`.

## Applying migrations

`db.migrate` decides who runs the schema:

- **`apply`** (the default) — the server applies its embedded migrations at
  boot. This is what makes a fresh checkout and an empty database work with no
  configuration at all, and it is the right choice for development and for
  small single-tenant deployments. The runtime DSN must be a member of
  `ferroehr_migrator`, so the serving process holds DDL rights for its whole
  life. Least isolation.
- **`verify`** — the server issues **no DDL at all**. At boot it checks that the
  database carries exactly this build's migrations and refuses to start
  otherwise, naming the schema and what is wrong with it. The DSN can then be
  `ferroehr_app` only, which is the least-privilege production posture: an
  application-level SQL flaw can then reach rows, never the schema.

With `verify`, something else has to run the migrations first. Use the binary's
own subcommand under the migrator DSN — a CI/CD stage, a one-shot job, or the
Helm chart's `migrations.job.enabled` pre-install/pre-upgrade hook Job, which
Helm waits on so a failed migration fails the release (it takes its own
`migrations.job.existingSecret` holding the migrator DSN — deliberately a
different credential from the runtime one, and rendering fails without it):

```shell
FERROEHR__DB__URL='postgres://ferroehr_migrator:***@pg:5432/ferroehr' \
  ferroehr db migrate     # applies; exits when done
FERROEHR__DB__URL='postgres://ferroehr_app:***@pg:5432/ferroehr' \
  ferroehr db verify      # read-only check; exit 0 iff the schema is current
```

Gate the rollout on the migration step so two server versions never race the
schema. Note the difference in failure shape: a `verify` server refuses to boot
against an unmigrated database (loud, immediate), while an `apply` server that
loses its schema later stays up and reports readiness `DOWN` — the warning
below.

> [!WARNING]
> **Migration is a boot step, and nothing re-runs it.** A running instance whose
> database is replaced, wiped, or reachable-but-empty does not migrate. It reports
>
> ```json
> {"status":"DOWN","components":{"db":{"status":"UP"},
>  "migrations":{"status":"DOWN","detail":"core schema tables missing (migrations not applied)"}}}
> ```
>
> on `/health/readiness` (`503`), leaves the load balancer's rotation, and keeps
> passing liveness — correctly, since the process is healthy — so nothing restarts
> it. Under Kubernetes that is a Deployment sitting at `0/N` ready with no error
> after the first one.
>
> The readiness check re-tests the schema on **every probe**, so recovery does not
> require a restart *of that instance* — it goes back to `UP` within one probe
> interval of the schema existing, whoever created it. What needs a restart is the
> case where the only thing that would migrate is the instance itself: then
> `kubectl rollout restart deploy/ferroehr` (or a migration job) is the remedy.
>
> For the out-of-band flow this means: the migration step must **complete before**
> the first instance starts, or that instance sits unready until the schema
> appears — harmless but confusing, and it delays the rollout rather than failing
> it. Gate the rollout on the migration job.

### Recovering a partially wiped database

A wipe that removes *some* of the server's schemas is not a fresh start, and the
server refuses to migrate over one rather than doing something plausible with it.

Almost every object lives in `ehr`, but the cold archival tier lives in its own
`cold` schema, so `DROP SCHEMA ehr CASCADE` — a restore gone wrong, a recreated
volume, a wiped test database — takes the primary tier and the migration
bookkeeping and **leaves the archived clinical rows standing**. On the next boot
you get:

```text
migration: the cold archival tier (schema `cold`) is present but the primary tier
(`ehr.vo_version`) is not: the two are one repository and have been wiped apart.
```

The refusal is deliberate. Those mirror tables were created from the primary
tables as they stood when archiving was set up, so silently adopting a survivor
could leave the archive tier a different shape from the tier it mirrors — and
its rows are clinical content belonging to a repository that no longer exists.
Two remedies, and which one applies is your call, not the server's:

- **The data mattered.** Restore the whole database from backup — both schemas
  together, since they are one repository. Do not try to graft the surviving
  `cold` tables onto a fresh schema; their `ehr`, `contribution` and `audit`
  parents are gone, so they are fragments, not a recoverable archive.
- **The wipe was intended** (a test database, a recreated volume). Then
  `DROP SCHEMA cold CASCADE` and start the server again; it migrates from
  scratch.

The reverse partial wipe — dropping `cold` while `ehr` survives — is not caught at
boot, because the migration bookkeeping still records the archival tier as applied.
It surfaces the first time an archive, restore or whole-repository export runs.
Restore from backup; there is no forward path that invents the archived rows back.

> [!TIP]
> When you wipe a FerroEHR database deliberately, drop the **database**, not a
> schema. `DROP DATABASE` cannot leave half a repository behind, and it is the
> only wipe with no partial-state failure mode.

## TLS and database security

These are database-side settings that belong to whoever provisions PostgreSQL;
the deployment references them but cannot enforce them:

- **TLS in transit.** Require `hostssl` on the server and put
  `?sslmode=verify-full` in the DSN so the client verifies the server
  certificate.
- **pgaudit.** Run pgaudit as the database-layer complement to the openEHR
  audit and the ATNA trail — for example `pgaudit.log = 'ddl, role,
  connection'` globally plus object-level audit on the PHI tables — and ship
  the audit log to an immutable store with long (roughly six-year) retention.
- **Encryption at rest.** Encrypt at the volume or disk layer. Do not encrypt
  the stored clinical JSON with pgcrypto — it would break AQL's ability to
  query inside the data.

## Backup and point-in-time recovery

Enable WAL archiving and point-in-time recovery from day one (pgBackRest or a
managed PITR), because a CDR's data is not reconstructible. Clinical and audit
tables are never `UNLOGGED`. Test your restore, not just your backup.

## The container image and pod hardening

The published image is distroless and non-root — shell-less, with no package
manager — and is multi-architecture (amd64 and arm64) on GHCR. It is not a
static binary: the server links glibc and `libgcc_s` dynamically, which is why
the base image is the `cc` distroless variant rather than the smaller one, and
why the runtime needs no OpenSSL, no JVM and nothing else from a package
manager.

Under Kubernetes the pod runs the Kubernetes `restricted` profile — non-root
uid/gid 65532, read-only root filesystem with one writable `/tmp`, no
privilege escalation, an empty capability bounding set, `RuntimeDefault`
seccomp, and no service-account token (the workload never calls the Kubernetes
API). The chart's own gates assert that per container on every render, and the
posture read back from a running container, plus the two things the chart
cannot do for you (applying the namespace enforcement label, and narrowing the
NetworkPolicy's ingress sources — the shipped policy narrows the *ports* and,
until you set `networkPolicy.ingressFrom`, admits every source on them), are
covered in
[Installation → The workload: security context & admission](installation/hardening-workload.md)
and [Namespaces, network & policy](installation/hardening-network-policy.md).

The Compose path carries a comparable floor, and one guard keeps it:
`scripts/checks/compose-hardening.sh` runs over every committed compose
artifact and fails on a service definition without `cap_drop: [ALL]` and
`no-new-privileges:true`, on any `privileged: true`, on a `seccomp=unconfined`
or `apparmor=unconfined` override, on a mounted Docker daemon socket, and on a
published port that names no host address. The compose files go further than
the guard checks — capabilities are added back one at a time only where an
entrypoint provably needs them, file-descriptor limits are bounded, ports
default to the loopback interface, and most services run a read-only root
filesystem — but those extras are conventions, not enforced properties, so
check them when you adapt a file.

Two services in the quickstart file deliberately do not run read-only, and each
says why in place. The S3 gateway writes its volume store; the server cannot,
because Compose refuses an inline `config` in a read-only service and that
inline config is what makes the file standalone. A deployment that mounts its
configuration from a file instead can add `read_only: true` and a tmpfs at
`/tmp`.

### What the host owes, and we cannot enforce

Four controls belong to whoever runs the daemon. They are stated here because a
container hardening story that ignores them is misleading — the strongest pod
security context in the world sits on top of these.

**Keep the host kernel and Docker Engine current.** A container is a kernel
namespace, not a virtual machine: a kernel privilege-escalation bug is a container
escape, and every runtime hardening above assumes the kernel enforcing it is
patched. Track your distribution's kernel updates and the Docker Engine release
notes with the same urgency you would give a public-facing service.

**Prefer rootless mode.** Running the daemon as a non-root user means a container
escape lands as an unprivileged user rather than as root on the host
(<https://docs.docker.com/engine/security/rootless/>). The images here need no
privileged operation, no host networking, and no daemon socket, so nothing in this
deployment prevents rootless — the constraints are usually the host's (cgroup v2,
`newuidmap`/`newgidmap`, and no privileged ports below 1024, which is why the
server binds 8080 rather than 80).

**Set the daemon log level to `info` (the default) and keep it.** Docker's `debug`
level records request payloads and can put secret material into daemon logs, which
are typically world-readable to anyone with host log access and are shipped
wholesale to log aggregators.

**Control who can pull and push your images.** GHCR access is the deployment's
authorization boundary for what runs in production: whoever can push a tag your
manifests reference can run their code with your database credentials. Restrict
push rights, prefer digest pins over mutable tags for anything you deploy (the
compose files pin the third-party images by digest for exactly this reason), and
verify the published attestations before rollout — the verification command is in
[Installation → Kubernetes & Helm](installation/kubernetes.md).

None of these four is something this project can assert on your behalf, which is
why they are written as your checklist rather than as our claim.

## Upgrades

- **Backward-compatible migrations.** Migrations are append-only and never
  edited once applied. A rolling upgrade must be compatible with the
  _previous_ schema for the window where both versions run: apply additive
  changes first, and defer destructive changes to a later release once every
  pod is on the new version.
- **Bound the DDL yourself.** Every pooled connection carries the
  `db.statement_timeout_ms` value (60 seconds by default), and the migration
  step runs on that pool, so a runaway statement is cut off — but there is **no
  `lock_timeout`**, so a migration that waits behind a long transaction waits
  as long as that transaction lasts. On a busy table use
  `CREATE INDEX CONCURRENTLY`, add constraints `NOT VALID` and `VALIDATE`
  later, and set `lock_timeout` in the migration session (or on the migrator
  role) if you need the wait bounded.
- **Pin the image.** Deploy an immutable tag or, better, a `@sha256` digest,
  never `latest`; roll back by re-pinning the prior digest — the schema's
  backward compatibility makes that safe.
- **Stay available.** Keep at least two replicas (or autoscaling) and a pod
  disruption budget so node drains and upgrades never fully interrupt the
  API. The default 30-second termination grace period covers the server's
  short shutdown drain of the audit and event outboxes.

### How long the version you pinned is supported

Plan the upgrade cadence around this, because it is short and it is deliberate:

- **Only the most recent release receives security fixes.** There are no
  maintenance branches, no long-term-support line, and no backports. A version
  stops receiving fixes the moment a newer release exists.
- **A fix normally arrives as the next patch** on the current minor, so taking
  it does not oblige you to take new behaviour — but that is the usual case,
  not a promise. Where a fix is only correct alongside a behavioural change,
  the release carrying it carries the change, and the changelog entry says so.
- **A published release is never repaired in place.** Release immutability
  means the assets and the tag of a published release cannot be modified at
  all, so the remedy for any defect is a new version.
- **The Helm chart and the published crates follow their own version lines**,
  each supported at its newest published version only. A chart-only fix ships
  as a new chart version between server releases.

The consequence for change control: budget for taking every release, or budget
for maintaining a fork. There is no third option, and the full policy — with
the reasoning and what to do if you need something stronger — is
[SECURITY.md](https://github.com/rubentalstra/FerroEHR/blob/develop/SECURITY.md#supported-versions).

## Observability

`tracing` is the single instrumentation API. From it, three signal families
fan out, and **identified data never enters any of them** — telemetry uses
only closed-set labels and opaque request/trace ids, so correlation to a
patient is possible only through the [audit trail](audit.md).

- **Logs** go to stdout — JSON when not attached to a terminal, pretty on a
  TTY — each line stamped with the trace and span id. Shipping and rotation
  are the platform's job. `FERROEHR__LOG__FORMAT` (`auto`/`json`/`pretty`) and
  `FERROEHR__LOG__FILTER` (or `RUST_LOG`, default `info,ferroehr=info`) control
  them, and the level can be changed at runtime through the `loggers` endpoint
  below. On boot the server prints a one-time ASCII banner (version, maintainer,
  project URL, and spec pins) to stdout ahead of the logs; it is suppressed
  under `FERROEHR__LOG__FORMAT=json` so machine log consumers see only structured
  lines.
- **Traces** export to any OpenTelemetry collector (Tempo, Jaeger, and so on)
  over OTLP — but only when you configure an endpoint; with none set, the
  tracing layer is not installed at all (zero overhead). Root spans are named
  by route template, never by a path containing ids.
- **Metrics** come from **one** OpenTelemetry meter provider with up to two
  readers: a Prometheus reader behind `/management/prometheus`, and — when
  `telemetry.metrics_push` is on — a periodic OTLP reader. Every instrument
  reaches both by construction, so a family can never exist on the scrape
  surface and be missing from the push. The catalogue covers HTTP request
  duration, active requests and body sizes; authentication failures and
  authorization decisions (Cedar and remote-PDP); database pool state,
  acquire latency and transaction counts; AQL query counts, latency and plan-
  cache events; compositions committed (by openEHR audit change type);
  validation failures and version-signature faults; WebTemplate cache events;
  events published; the whole ATNA audit pipeline; Tokio runtime gauges;
  process start time; and the `ferroehr_build_info` identity.

  Instrument names carry **no** `_total` suffix and no unit suffix — units are
  declared on the instrument and the Prometheus exporter derives
  `_total`/`_seconds`/`_bytes` itself. Read the exposition to learn the exact
  rendered names rather than assuming either spelling.

The telemetry environment variables:

| Environment variable | Default | Meaning |
|---|---|---|
| `FERROEHR__TELEMETRY__OTLP_ENDPOINT` | unset (layer not installed) | OTLP collector endpoint |
| `FERROEHR__TELEMETRY__SERVICE_NAME` | `ferroehr` | reported service name |
| `FERROEHR__TELEMETRY__ENVIRONMENT` | `dev` | reported deployment environment |
| `FERROEHR__TELEMETRY__TRACES_SAMPLE_RATIO` | `1.0` | head sampling ratio (start at `0.1` in production) |
| `FERROEHR__TELEMETRY__METRICS_PUSH` | `false` | add the periodic OTLP metrics reader beside the Prometheus one |
| `FERROEHR__TELEMETRY__FLAME_FILE` | unset (layer not installed) | write folded span-timing samples to this file for offline rendering — diagnostic sessions only |

Neither reader is reachable until you say so: the Prometheus surface needs
`management.enabled` plus an access level on the `prometheus` endpoint (see
[the management surface](#the-management-surface)), and the OTLP reader needs
both `otlp_endpoint` and `metrics_push`. A server with neither still records
every instrument; nothing exports it.

> [!TIP]
> A single-container dev stack (`grafana/otel-lgtm`, bundling an OTLP
> collector, Prometheus, Tempo, Grafana, and Loki) ships as a Compose overlay,
> together with a provisioned Grafana dashboard (request rate/errors/duration,
> database pool, AQL latency, validation failures, audit health) and a starter
> alert pack — point the server at it with the two OTLP variables above.

**On Kubernetes**, the same keys arrive through the chart's `config`
passthrough, and the metrics half has a second switch that is easy to miss:

```yaml
# values.yaml
config:
  telemetry:
    otlp_endpoint: http://otel-collector.observability:4317
    environment: production
    traces_sample_ratio: 0.1
    metrics_push: true
  management:
    enabled: true
    endpoints:
      prometheus: admin_only   # the scrape endpoint is off until you name it
metrics:
  enabled: true                # adds the prometheus.io/* pod annotations
  serviceMonitor:
    enabled: false             # or true, with the Prometheus Operator CRDs installed
```

`metrics.enabled` only adds the scrape **annotations**; the endpoint itself is
opened by `config.management.endpoints.prometheus`. Both are needed for an
annotation-discovering Prometheus, and neither is needed if you push over OTLP
instead. **To turn telemetry off**, drop `otlp_endpoint` — the tracing layer is
not installed at all when it is unset.

> [!WARNING]
> With the chart's default-deny egress policy on, add the collector to
> `networkPolicy.egress.rules` (port 4317). An OTLP exporter that cannot reach
> its collector fails **silently** — no traces, no error.

## The admin and messaging APIs

Two operator-facing HTTP surfaces have a page of their own, because each route
carries its own switch, authorization class and status-code contract:

- **The admin API** (`{base}/admin/…`, off by default) — physical deletion, the
  activity report, archiving to the cold tier, and whole-repository dump and
  load.
- **The messaging API** (`{base}/message/…`, always mounted) — EHR Extract
  export and import, and Template Data Document import.

The full reference is [Admin & messaging APIs](operations-admin-apis.md).

> [!WARNING]
> Enabling the admin API puts irreversible, whole-repository operations on the
> wire — `DELETE {base}/admin/ehr/all` with no parameter empties the
> repository. Keep it off unless a workflow needs it, turn RBAC on, and gate
> the admin role tightly.



## Health probes

The health endpoints are **always served, on the main API port, without
authentication**. There is nothing to enable and nothing to remember: they are
mounted outside the API's authentication and overload-shedding layers, so an
orchestrator can probe a server whose management surface, admin API, and every
optional integration are switched off — and a saturated server still answers
its own probes.

### Choosing a health endpoint

| Endpoint | Contract | Use it for |
|---|---|---|
| `GET /health` | constant `200 OK` (plain text `OK`), touches nothing | load balancers, `docker` `HEALTHCHECK`, anything that must never be auth-gated |
| `GET /health/liveness` | identical to `/health` — the same constant answer under the orchestrator-conventional path | Kubernetes `livenessProbe` and `startupProbe` |
| `GET /health/readiness` | `200` when the aggregate is up or degraded, `503` when a **required** component is down; JSON body with every indicator, each bounded to one second | Kubernetes `readinessProbe`, ops dashboards |
| `GET /ferroehr/rest/status` | product status document: `status`, `server_version`, `openehr_rest_api_version`, `timestamp` | version/identity checks; the URL the container's `ferroehr healthcheck` subcommand probes |
| `GET /management/*` | ops introspection — see below | operators, off by default, enable deliberately |

There is exactly one health surface — the `/health` family above. `/health` and
`/health/liveness` are two conventional names for the same constant answer (a
load balancer wants the bare path, an orchestrator wants the `liveness`/
`readiness` pair); `/ferroehr/rest/status` is a different contract, and no health
endpoint exists under the REST root.

Not every indicator blocks readiness, and the distinction is deliberate:

| Indicator | Checks | Blocks readiness |
|---|---|---|
| `db` | a pooled connection answers | yes |
| `migrations` | this build's schema is present, re-tested on every probe | yes |
| `audit_sender` | whether ATNA forwarding is enabled | no — reports `DEGRADED`, never `503` |
| `events` | the event publisher's broker delivery (present only when eventing is enabled) | no — reports `DEGRADED`, never `503`: the outbox buffers while the broker is down |

An instance whose event broker is unreachable therefore keeps taking traffic
and says so in the body; an instance that cannot reach its database, or whose
database lost the schema, leaves rotation. The `detail` strings are written to
carry no connection information — no DSN host, database name or role — because
this surface is unauthenticated by design.

> [!IMPORTANT]
> Liveness and readiness are deliberately different: liveness never touches a
> dependency, so a database outage takes the instance out of rotation
> (readiness `503`) instead of getting the container killed and restarted in a
> loop. Wire `livenessProbe` and `startupProbe` to `/health/liveness` and
> `readinessProbe` to `/health/readiness`. The Helm chart does exactly this out
> of the box, and only the timings are tunable (`probes.liveness`,
> `probes.readiness`, `probes.startup`) — there is deliberately no option to
> point the probes at the container's `ferroehr healthcheck` subcommand
> instead, because that subcommand probes the status document rather than a
> health endpoint, which would leave readiness never touching the database.

## The management surface

The management surface is **ops introspection only** — build info, Prometheus,
the metric views, the effective configuration, and runtime log control. It is
**off by default** on the bare binary, and each endpoint is independently
opt-in with an access level (`admin_only`, `private`, or `public`). It can be
bound to its own internal port so it never appears on the public API listener.
Keeping it off costs you nothing operationally: the health probes above do not
depend on it.

| Environment variable | Default | Meaning |
|---|---|---|
| `FERROEHR__MANAGEMENT__ENABLED` | `false` | enable the management surface |
| `FERROEHR__MANAGEMENT__BASE_PATH` | `/management` | base path for the surface |
| `FERROEHR__MANAGEMENT__PORT` | unset (main listener) | serve management on its own port |
| `FERROEHR__MANAGEMENT__ENDPOINTS__<NAME>` | `off` | the access level for ONE endpoint — `off`, `private`, `admin_only` or `public`. There is no global default beside it: an endpoint you do not name is not mounted and answers `404`. |

The ops endpoints:

Every one of them ships `off` — nothing is mounted until you name the endpoint
and the level it should answer at. The right-hand column is the level to choose,
not a default you already have.

| Endpoint | Endpoint name to set | Purpose | Level to give it |
|---|---|---|---|
| `GET /management/info` | `info` | product name and version, build SHA, build date, `rustc`, the active `spec_profile`, the openEHR specification versions that profile selects, and the PostgreSQL target | `admin_only` |
| `GET /management/prometheus` | `prometheus` | Prometheus text exposition | `admin_only`, or `public` only when the port is not reachable outside the cluster — a `public` endpoint is served OUTSIDE authentication |
| `GET /management/metrics` | `metrics` | JSON list of the registered metric names | `admin_only` |
| `GET /management/metrics/{name}` | `metrics` | the current value(s) of one metric; `404` for a name that is not registered | `admin_only` |
| `GET /management/env` | `env` | effective configuration, with secrets redacted | `admin_only` |
| `GET`/`POST`/`DELETE` `/management/loggers` | `loggers` | read and change the log level at runtime | `admin_only` |
| `GET /management/flamegraph` | `flamegraph` | on-demand CPU flamegraph of the running server | `admin_only` |

Paths above show the default `management.base_path`; change it and every path
moves with it. The `metrics` name covers both metric routes, and the
`prometheus`, `metrics` and `loggers` routes additionally need their backing
machinery present — without it they are simply not mounted, which is the same
`404` as leaving them `off`.

> [!WARNING]
> `/management/env` and `/management/loggers` expose and change server
> internals — keep them `admin_only`, and prefer binding the surface to an
> internal-only port.

### Profiling: the on-demand CPU flamegraph

When the server is measurably slow, the metrics tell you *how slow*;
`/management/flamegraph` tells you **where the time goes**. The endpoint samples the
whole process with an in-process sampling profiler (the
[`pprof`](https://docs.rs/pprof/latest/pprof/) crate) for a bounded window and
answers with a rendered flamegraph SVG — open it in a browser and read the wide
frames.

```bash
# sample 10 s at 99 Hz (the defaults) and open the result
curl -u admin:… -o flamegraph.svg \
  "http://cdr.internal:9100/management/flamegraph?seconds=10&frequency=99"
```

- `seconds` (default 10) and `frequency` (default 99 Hz) are capped by
  `management.profiling.max_seconds` and `max_frequency` (30 and 999 out of the
  box); a request beyond a cap is refused with `400`, never silently clamped.
- **One sample window at a time**: a second request while one runs answers
  `409` — retry when the window completes.
- Sampling is low-overhead but not free; profile under the real load you are
  diagnosing, and keep the endpoint `admin_only` on an internal port like the
  rest of the surface.
- Best results come from the container images and release builds, which keep
  line tables (`debug = "line-tables-only"`) so frames resolve to `file:line`.

With the surface enabled, the admin console grows an **Operations** screen over
it — dependency health, build provenance, the metric registry, and runtime log
control — which appears only while the CDR serves `/management/info`. See
[Admin console → Operations panel](admin-ui/operations.md).

## Next

- [Admin & messaging APIs](operations-admin-apis.md) — the operator-facing
  HTTP surfaces, route by route.
- [Configuration reference](installation/configuration.md) — every setting,
  with the server, database and telemetry keys on
  [their own page](installation/config-server.md).
- The API reference at `/ferroehr/api/` (the **API** tab in the toolbar) — the
  document the server itself generates.
