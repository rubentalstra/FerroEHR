# Operations

Running a clinical data repository in production means more than starting the
binary: the database must be backed up and least-privileged, traffic must be
encrypted, upgrades must be safe while the service stays up, and you need to
see what the system is doing. This chapter is a production checklist —
database roles, TLS, backup and point-in-time recovery, upgrades and
migrations, observability, the health probes, and the management surface —
drawn from how the container image and Helm chart are built to run.

<!-- toc -->

## Database roles and least privilege

EHRbase-rs connects to an **external** PostgreSQL 18 — a managed service or an
operator-run cluster, never a chart-side sidecar, because a database holding
PHI must be independently backed up and recoverable. The server carries only a
connection string, ideally sourced from a secret.

The database uses a four-role model — never a superuser at runtime:

| Role | Purpose | Used by |
|---|---|---|
| owner | owns the database | provisioning only |
| `ehrbase_migrator` | runs the schema migrations; owns the helper functions | the migration step |
| `ehrbase_app` | reads and writes clinical data | the running server |
| `ehrbase_reader` | read-only | replicas and reporting |

The migrations create these roles idempotently, apply the per-schema grants,
and revoke the ability to create objects in the public schema. **The running
server connects as `ehrbase_app`** — its DSN should authenticate as that role,
not the migrator or the owner.

## Applying migrations

The binary applies its embedded migrations on boot, so you choose how
migrations run:

- **Grant the runtime DSN the migrator role** — simplest, for single-tenant or
  small deployments; the server migrates itself at startup. Least isolation.
- **Run migrations out of band** with a migrator DSN, then start the server
  with the lower-privileged `ehrbase_app` DSN — recommended for
  least-privilege production. Run the migration as a CI/CD step or a one-shot
  job with the migrator credential _before_ rolling the deployment, and gate
  the rollout so two versions never race the schema.

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
manager — and is multi-architecture (amd64 and arm64) on GHCR. When run under
Kubernetes with the provided Helm chart, the pod is hardened and the chart's
validation asserts these on every render:

| Setting | Value |
|---|---|
| run as non-root | uid/gid 65532 |
| read-only root filesystem | yes (a writable `emptyDir` at `/tmp`) |
| privilege escalation | disallowed |
| Linux capabilities | all dropped |
| seccomp | `RuntimeDefault` |
| service-account token | not mounted (the workload never calls the Kubernetes API) |
| NetworkPolicy | default-deny ingress; only the API/management port admitted |

Egress restriction is opt-in because egress targets (database, broker,
terminology server) are deployment-specific. See
[Installation → Kubernetes & Helm](installation/kubernetes.md) for the chart
itself.

## Upgrades

- **Backward-compatible migrations.** Migrations are append-only and never
  edited once applied. A rolling upgrade must be compatible with the
  _previous_ schema for the window where both versions run: apply additive
  changes first, and defer destructive changes to a later release once every
  pod is on the new version.
- **Lock-safe DDL.** The migration runner bounds DDL with a `lock_timeout` and
  `statement_timeout` so a migration cannot block live traffic indefinitely;
  on a busy table use `CREATE INDEX CONCURRENTLY` and add constraints
  `NOT VALID` then `VALIDATE` later.
- **Pin the image.** Deploy an immutable tag or, better, a `@sha256` digest,
  never `latest`; roll back by re-pinning the prior digest — the schema's
  backward compatibility makes that safe.
- **Stay available.** Keep at least two replicas (or autoscaling) and a pod
  disruption budget so node drains and upgrades never fully interrupt the
  API. The default 30-second termination grace period covers the server's
  short shutdown drain of the audit and event outboxes.

## Observability

`tracing` is the single instrumentation API. From it, three signal families
fan out, and **identified data never enters any of them** — telemetry uses
only closed-set labels and opaque request/trace ids, so correlation to a
patient is possible only through the
[ATNA audit trail](security.md#atna-audit-trail).

- **Logs** go to stdout — JSON when not attached to a terminal, pretty on a
  TTY — each line stamped with the trace and span id. Shipping and rotation
  are the platform's job. `EHRBASE__LOG__FORMAT` (`auto`/`json`/`pretty`) and
  `EHRBASE__LOG__FILTER` (or `RUST_LOG`, default `info,ehrbase=info`) control
  them, and the level can be changed at runtime through the `loggers` endpoint
  below. On boot the server prints a one-time ASCII banner (version, maintainer,
  project URL, and spec pins) to stdout ahead of the logs; it is suppressed
  under `EHRBASE__LOG__FORMAT=json` so machine log consumers see only structured
  lines.
- **Traces** export to any OpenTelemetry collector (Tempo, Jaeger, and so on)
  over OTLP — but only when you configure an endpoint; with none set, the
  tracing layer is not installed at all (zero overhead). Root spans are named
  by route template, never by a path containing ids.
- **Metrics** are exposed for Prometheus to scrape at `/management/prometheus`
  (OTLP metrics push is an option). The catalogue includes HTTP request
  duration and active requests, authentication failures and authorization
  decisions, database pool state, AQL query counts and latency, compositions
  committed (by openEHR audit change type), validation failures, and the audit
  pipeline's health.

The telemetry environment variables:

| Environment variable | Default | Meaning |
|---|---|---|
| `EHRBASE__TELEMETRY__OTLP_ENDPOINT` | unset (layer not installed) | OTLP collector endpoint |
| `EHRBASE__TELEMETRY__SERVICE_NAME` | `ehrbase` | reported service name |
| `EHRBASE__TELEMETRY__ENVIRONMENT` | `dev` | reported deployment environment |
| `EHRBASE__TELEMETRY__TRACES_SAMPLE_RATIO` | `1.0` | head sampling ratio (start at `0.1` in production) |
| `EHRBASE__TELEMETRY__METRICS_PUSH` | `false` | also push metrics over OTLP |

> [!TIP]
> A single-container dev stack (`grafana/otel-lgtm`, bundling an OTLP
> collector, Prometheus, Tempo, Grafana, and Loki) ships as a Compose overlay,
> together with a provisioned Grafana dashboard (request rate/errors/duration,
> database pool, AQL latency, validation failures, audit health) and a starter
> alert pack — point the server at it with the two OTLP variables above.

## The admin API: physical deletion

Normal openEHR deletes are *logical* — history is retained. The admin API is
the exception: **physical, irreversible** removal, for legal erasure requests
and test-data cleanup. It is enabled only by
`EHRBASE__ADMIN__ENABLED=true` and is classed under admin authorization (the
`ADMIN` role). While it is disabled every admin route answers
**405 Method Not Allowed** with an empty `Allow` header — the resource exists
but currently serves no method (see
[Error bodies](using-the-api/content-negotiation.md#405-always-names-the-allowed-methods)).

- `DELETE {base}/admin/ehr/{ehr_id}` — physically delete one EHR and
  everything in it. **204** on success, **404** for an unknown id.
- `DELETE {base}/admin/ehr/all` — bulk delete. **With no `ehr_id` parameter
  this deletes every EHR on the server.** To delete a subset, pass
  `?ehr_id=<uuid>` — repeatable (`?ehr_id=a&ehr_id=b`) or comma-separated
  (`?ehr_id=a,b`). Returns **204** with no body.
- `DELETE {base}/admin/template/{template_id}` — physically delete one
  operational template. **204** on success, **404** for an unknown id, and
  **409** if any stored composition was committed against the template
  (delete those compositions first — a physical delete never orphans clinical
  data).
- `DELETE {base}/admin/query/{qualified_query_name}/{version}` — physically
  delete one stored-query version (a single `name`/`version` row). **204** on
  success, **404** for an unknown name or version.
- `GET {base}/admin/config` — the **effective configuration** as a JSON tree
  (the merged result of the config file, `EHRBASE_*` environment, and
  `--set` overrides). Every secret-bearing value is **redacted**: passwords
  and password hashes, HMAC and signing-key secrets, and S3 secret keys
  render as `***`, and connection URLs (database, AMQP) keep their host and
  path but mask the embedded credentials (`postgres://***@host:5432/db`).
  Non-secret identifiers — usernames, roles, an OIDC issuer — stay visible.
  **200** with the redacted tree. Redaction is a structural property of the
  configuration's secret types, not a key-name filter, so no secret value can
  reach this response.

> [!NOTE]
> The template and stored-query deletes and the config view are ehrbase-rs
> extensions — the openEHR admin API defines only EHR deletes. They share the
> same admin gate and authorization as the EHR deletes.

## The admin API: activity report

Four **read-only** counters over the repository's change history, behind the
same `EHRBASE__ADMIN__ENABLED` gate and `ADMIN` role as the deletes above.
Every route takes `a_service` — the service whose versioned content to report
on, one of `Admin`, `Definitions`, `Ehr`, `Ehr_index`, `Demographic`,
`Message`, `Query`, `System_log` (case-insensitive) — and an optional
`time_interval=<lower>/<upper>` of ISO 8601 date-times matched inclusively
against each commit time. Either bound may be left empty for an open interval
(`?time_interval=2026-01-01T00:00:00Z/`); an absent parameter reports over all
time. A service that holds no versioned content reports an empty list or `0`
rather than failing.

- `GET {base}/admin/report/contribution` — the ids of the matching
  CONTRIBUTIONs, ordered by commit time. **200** with a JSON array.
- `GET {base}/admin/report/contribution/count` — how many there are. **200**
  with a bare JSON number.
- `GET {base}/admin/report/versioned_composition/count` — how many distinct
  COMPOSITION version containers had a version committed in the interval.
- `GET {base}/admin/report/composition_version/count` — how many individual
  COMPOSITION versions were committed in the interval.

An unknown `a_service`, or a `time_interval` that is not `<lower>/<upper>`
with valid ISO 8601 bounds, is **400**.

## The admin API: archiving

Two routes that mark a selected set of records as archived, behind the same
gate and role. Archiving is **not** a delete: it is a lifecycle marker, and
the archived records stay fully readable through the normal API.

- `POST {base}/admin/archive/ehrs` with `{"ehr_ids": ["…"]}` — mark every
  named EHR (and all its versioned content) archived. **204** on success.
- `POST {base}/admin/archive/parties` with `{"party_ids": ["…"]}` — mark every
  named demographic party archived. **204** on success.

Both are **all-or-nothing and idempotent**: a malformed id is **400** and an
id that names nothing is **404**, in both cases before anything is marked;
re-archiving an already-archived record changes nothing. An empty list
succeeds and archives nothing.

## The admin API: dump and load

Two routes that move the whole repository to and from an archive on the
**server's** file system, behind the same `EHRBASE__ADMIN__ENABLED` gate and
`ADMIN` role. Both answer **200** with a JSON array of per-entity failure
reports — an **empty** array means everything succeeded.

- `POST {base}/admin/dump` — write an archive of every EHR. The body is
  `{"file_sys_loc": "…"}` plus the optional export settings:

  | Field | Values | Default |
  |---|---|---|
  | `logical_format` | `openehr_canonical_json` | canonical JSON |
  | `compression_format` | `zip` or `7z` — omit for loose files | uncompressed |
  | `segment_split_size` | segment size in kb (a positive integer) | `1024` |

- `POST {base}/admin/load` with `{"file_sys_loc": "…"}` — populate the
  repository from an archive. It takes the location and nothing else: the
  container (loose files, a single `archive.zip`, or a single `archive.7z`)
  is detected from what the location holds, so a load never has to be told
  how the dump was written.

The archive is a directory holding a `manifest.json`, one or more
`segment-NNNN.json` files, and a `blobs/` subdirectory for any externalized
multimedia — or exactly those entries packed into one `archive.zip` or
`archive.7z` when `compression_format` is set.

The repository being loaded into **need not be empty**; an EHR whose id is
already present is reported and skipped rather than failing the load, so the
response array names each one:

```json
[ { "entity_type": "EHR",
    "entity_id": "7d44b88c-4199-4bad-97dc-d78268e01398",
    "dump_status": false,
    "error": "an EHR with this id already exists" } ]
```

A missing or blank `file_sys_loc`, a format value that is not one of the ones
listed above, a non-positive `segment_split_size`, or an `encoding` field is
**400**. `openehr_canonical_xml` is **501**: the openEHR service model names
it, this server does not implement it yet, and it says so rather than
silently writing a different format.

> [!NOTE]
> The activity report, the archive routes, and the dump/load pair are
> ehrbase-rs extensions too — the openEHR *service model* defines these
> operations, but the released REST API surfaces no endpoint for them, so their
> URLs are our own. They gate no openEHR conformance claim; see
> [Conformance](conformance.md).

> [!WARNING]
> `DELETE /admin/ehr/all` without a parameter empties the repository — there
> is no confirmation step and no undo. Keep the admin API disabled unless a
> workflow needs it, and gate the `ADMIN` role tightly.

## The messaging API: EHR Extract and TDD import

A group of six routes under `{base}/message` that move whole records between
systems and accept documents in the template-data (TDD) form. Unlike the admin
extensions above, these are **not** admin-gated: they carry the same ordinary
authentication as the clinical API, because they read and write the same
clinical content.

### EHR Extract

- `GET {base}/message/export/{ehr_id}` — export one whole EHR. **200** with a
  JSON array holding one `EXTRACT` that carries every versioned object of the
  EHR, latest versions only. **404** if the EHR does not exist; **400** if
  `ehr_id` is not a well-formed identifier, which is refused before any lookup.
- `POST {base}/message/export` with an `EXTRACT_SPEC` body — export by
  specification. **200** with one `EXTRACT` per manifest entity, in manifest
  order; a manifest with no entities yields `[]`. Each entity must name the
  record by `ehr_id` or `subject_id`, otherwise **400**; an identifier that
  names nothing is **404**. `extract_type` must be one of the codes the
  openEHR Reference Model names — `openehr-ehr`, `openehr-demographic`,
  `openehr-synchronisation`, `openehr-generic`, `generic-emr` — or the
  catch-all `other`; anything else is **400**.
- `POST {base}/message/import` with an `EXTRACT` body — clone a whole EHR.
  Add `?ehr_id=<uuid>` to fix the identifier the clone lands under; leave it
  off and the source identifier the extract carries is re-used. **201** with
  `{"uid": "<ehr_id>"}`, so a caller that supplied no id still learns what was
  created. The target must not already exist (**409**), and the extract must
  carry an `EHR_STATUS` (**400**).
- `POST {base}/message/import/{ehr_id}` with an `EXTRACT` body — add the
  extract's content to an **existing** EHR as new versions. **204**. **404**
  if the EHR does not exist; **409** if the extract carries an `EHR_STATUS` or
  `EHR_ACCESS` other than the one the EHR already holds.

### TDD import

- `POST {base}/message/tdd/{ehr_id}` with an `application/xml` body — import
  one Template Data Document. It is converted against the operational template
  its root names and committed through the ordinary validated composition
  path, so **201** with `{"uid": "<version_uid>"}`. The template must already
  be uploaded through the definition API (**404** otherwise); a body that does
  not conform to it is **400**, and a document that is not well-formed XML is
  **422**.
- `POST {base}/message/tdd/{ehr_id}/batch` with a JSON array of TDD documents
  — import several at once. **201** with the created version ids in input
  order. The batch is **all-or-nothing**: every document is converted before
  any is committed, so one bad document rejects the whole batch and commits
  nothing. An **empty** array is a fulfilled no-op — **200** with `[]`, since
  nothing was created — but the target EHR is still checked, so an unknown one
  is **404** whatever the batch holds. The batch has no limit on how many
  documents it may carry; the only bound is the server-wide request-body
  limit, which answers **413** when exceeded.

> [!NOTE]
> The whole `/message` group is an ehrbase-rs extension: the openEHR service
> model defines a Message component, but the released REST API publishes no
> message, extract, or TDD endpoints at all. These URLs are our own and gate no
> openEHR conformance claim; see [Conformance](conformance.md).

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
| `GET /health/readiness` | `200` when the aggregate is up or degraded, `503` when a required component is down; JSON body with every indicator (database ping, migrations applied, audit sender, events), each bounded to one second | Kubernetes `readinessProbe`, ops dashboards |
| `GET /ehrbase/rest/status` | product status document: server version, ITS-REST version, timestamp | version/identity checks; the URL the container's `ehrbase healthcheck` subcommand probes |
| `GET /management/*` | ops introspection — see below | operators, off by default, enable deliberately |

There is exactly one health surface — the `/health` family above. `/health` and
`/health/liveness` are two conventional names for the same constant answer (a
load balancer wants the bare path, an orchestrator wants the `liveness`/
`readiness` pair); `/ehrbase/rest/status` is a different contract, and no health
endpoint exists under the REST root.

> [!IMPORTANT]
> Liveness and readiness are deliberately different: liveness never touches a
> dependency, so a database outage takes the instance out of rotation
> (readiness `503`) instead of getting the container killed and restarted in a
> loop. Wire `livenessProbe` and `startupProbe` to `/health/liveness` and
> `readinessProbe` to `/health/readiness`. The Helm chart does exactly this out
> of the box; `probes.exec.enabled=true` switches to the container's
> `ehrbase healthcheck` subcommand if the kubelet cannot reach the HTTP port.

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
| `EHRBASE__MANAGEMENT__ENABLED` | `false` | enable the management surface |
| `EHRBASE__MANAGEMENT__BASE_PATH` | `/management` | base path for the surface |
| `EHRBASE__MANAGEMENT__PORT` | unset (main listener) | serve management on its own port |
| `EHRBASE__MANAGEMENT__ACCESS_DEFAULT` | `admin_only` | default access level |

The ops endpoints:

| Endpoint | Purpose | Default access |
|---|---|---|
| `GET {base}/info` | build, version, and pinned spec versions | `admin_only` |
| `GET {base}/prometheus` | Prometheus text exposition | `admin_only` (re-expose to the scraper via network policy) |
| `GET {base}/metrics` | JSON registry view | `admin_only` |
| `GET {base}/env` | effective configuration, with secrets redacted | `admin_only` |
| `GET`/`POST`/`DELETE` `{base}/loggers` | read and change the log level at runtime | `admin_only` |

> [!WARNING]
> `{base}/env` and `{base}/loggers` expose and change server internals — keep
> them `admin_only`, and prefer binding the surface to an internal-only port.

With the surface enabled, the admin console grows an **Operations** screen over
it — dependency health, build provenance, the metric registry, and runtime log
control — which appears only while the CDR serves `{base}/info`. See
[Admin console → Operations panel](admin-ui/operations.md).

For the full list of configuration keys across every subsystem, see
[Installation → Configuration reference](installation/configuration.md); to
explore the API itself, open the API reference at `/ehrbase-rs/api/` (also
linked from the toolbar on every page).
