# Kubernetes & Helm

The `deploy/helm/ehrbase-rs` chart deploys EHRbase-rs as a hardened,
production-shaped Kubernetes workload: non-root, read-only root filesystem,
default-deny ingress, connecting to an **external** PostgreSQL 18. This chapter
covers installing the chart, the database role model it expects, the security
posture it enforces, health probes, and the optional integrations. It assumes a
cluster at Kubernetes 1.25 or newer.

> [!IMPORTANT]
> There is **no in-chart PostgreSQL**. A CDR stores PHI, so its database must be
> an externally managed, backed-up, point-in-time-recoverable PostgreSQL 18 (a
> managed service or an operator-run cluster). The chart carries only the
> connection string, preferably from an existing Secret.

## Installing

Create a Secret holding the app-role connection string, then install the chart
pointing at it:

```shell
kubectl -n ehrbase create secret generic ehrbase-db \
  --from-literal=EHRBASE__DB__URL='postgres://ehrbase_app:***@pg-host:5432/ehrbase?sslmode=verify-full'

helm install ehrbase-rs deploy/helm/ehrbase-rs -n ehrbase \
  --set database.existingSecret=ehrbase-db \
  --set image.tag=3.0.0
```

Always pin `image.tag` to an immutable version or, better, a `@sha256` digest —
never `latest`. Every `values.yaml` key documents the exact `EHRBASE_*`
environment variable it maps to; the [configuration reference](configuration.md)
is the full list.

## Database roles — who runs migrations

The chart expects a **four-role** PostgreSQL model, so the runtime pod is never
a superuser:

| Role | Purpose |
|---|---|
| owner | owns the database (provisioning only) |
| `ehrbase_migrator` | runs the append-only schema migrations |
| `ehrbase_app` | day-to-day reads and writes — **the running pod connects as this** |
| `ehrbase_reader` | read-only, for replicas and reporting |

The binary calls its migrations on boot, so you choose one of two flows:

- **(a) Grant the runtime DSN the migrator role** — simplest for single-tenant
  or small deployments; the pod migrates itself at startup.
- **(b) Run migrations out of band** with a migrator DSN (a CI step or a
  one-shot `Job`), then start the pods with the lower-privileged `ehrbase_app`
  DSN — recommended for least-privilege production. Gate the Deployment rollout
  on the migration step so two server versions never race the schema.

The chart does not ship a migration Job; `migrations.runByMigratorRole` is an
informational marker surfaced in the install NOTES.

## Secrets and mounted config

Some settings do not fit cleanly in environment variables — the Basic-auth user
store, a full OIDC block, RBAC role-claim lists, ABAC policies, the external
terminology provider map, ATNA TLS certificates, and the PGP signing key. Supply
these as files via `config.files`, which the chart mounts read-only from a
Secret at `/etc/ehrbase/<key>`; point the matching `EHRBASE_*_CONFIG` /
`*_PATH` variable at the file through `extraEnv`. Secret-bearing scalar values
(DB DSN, HMAC secret, broker URLs, S3 keys, PGP passphrase) go into the chart's
Secret, never the ConfigMap.

## Security posture

The chart pins — and its `validate.sh` gate asserts on every render — the
following:

| Field | Value |
|---|---|
| `runAsNonRoot` | `true` (uid/gid 65532, the distroless `nonroot` user) |
| `readOnlyRootFilesystem` | `true` (a writable `emptyDir` is mounted at `/tmp`) |
| `allowPrivilegeEscalation` | `false` |
| `capabilities.drop` | `[ALL]` |
| `seccompProfile.type` | `RuntimeDefault` (pod and container) |
| ServiceAccount token | not mounted (the workload never calls the K8s API) |
| NetworkPolicy | default-deny ingress; only the API (and management) port admitted |

Egress restriction is opt-in (`networkPolicy.egress.enabled`) because egress
targets — the database, broker, terminology server — are deployment-specific;
when you enable it the chart always admits DNS and you add rules for the rest.
The database-side controls (TLS with `sslmode=verify-full`, pgaudit, at-rest
encryption, WAL archiving / PITR) belong to whoever provisions PostgreSQL — the
chart references them but cannot enforce them. See
[Operations](../operations.md).

## Health probes

Probes use the management surface's public, unauthenticated, PHI-free health
routes:

| Probe | Route | Contract |
|---|---|---|
| liveness | `{management.basePath}/health/liveness` | 200 while the process is up |
| readiness | `{management.basePath}/health/readiness` | 200 (UP/DEGRADED) or 503 (DOWN): checks DB ping, migrations applied, audit sender, events — each 1s-bounded |
| startup | liveness route | gates a slow first boot |

Because the bare binary ships the management surface **off**, the chart turns it
on (`management.enabled=true`, `management.probesEnabled=true`) as a deliberate
deployment deviation — the probe routes carry no PHI. Set `management.port` to
serve the management surface on its own internal listener, so `/management` is
never reachable on the clinical API port. To probe without HTTP (management
off), set `probes.exec.enabled=true` to use the binary's `healthcheck`
subcommand instead.

Set `metrics.enabled=true` to expose `{management.basePath}/prometheus`
(access level `public`) with the `prometheus.io/*` scrape annotations. The
JSON `/management/metrics`, `/info`, `/env`, and `/loggers` endpoints stay
`admin_only` or off unless you opt in.

## Optional integrations

Every integration is **off by default**, matching the binary — enabling one is
an explicit, auditable decision. Each has a `values.yaml` switch and maps to a
config env prefix:

| Integration | Values key | Notes |
|---|---|---|
| ADMIN API | `rest.adminEnabled` | Physical, irreversible delete. Gate behind admin RBAC. |
| Terminology extension API | `rest.terminologyEnabled` | 404 when off. |
| Event-subscription API | `rest.eventSubscriptionEnabled` | Admin CRUD over event filters. |
| Multi-tenancy | `tenancy.enabled` | Tenant from a JWT claim; leave `tenancy.header` unset in prod. Pairs with PG row-level security. |
| OAuth2/OIDC auth | `auth.oidc.*` | Prefer JWKS/discovery over an HS256 secret. |
| ABAC | `authz.abac.enabled` | Cedar or a remote policy decision point; policies via a mounted TOML. |
| Eventing → AMQP | `events.enabled` | Envelopes are **PHI-free** by design. Use `amqps://`/`tls=true`. |
| FHIR inbound/façade | `rest.fhirEnabled` | Read façade + inbound mapping. |
| FHIR outbound → AMQP | `fhirOutbound.enabled` | ⚠ **Carries PHI** (the mapped FHIR resource). Separate exchange; TLS broker only. |
| S3 multimedia | `multimedia.enabled` | ⚠ Offloaded blobs are PHI. Private, encrypted, HTTPS bucket. |
| External terminology | `externalTerminology.enabled` | FHIR terminology server; provider map via a mounted TOML. |
| ATNA system log | `atna.enabled` | Use `transport: tls` for PHI-adjacent audit. |
| Version signing | `signing.*` | On by default (`digest`). `pgp` mode fails closed at boot without a usable key. |
| OTLP telemetry | `telemetry.otel.*` | Unset endpoint ⇒ the OTel layer is not installed (zero overhead). |

Full detail on each is in [Beyond the core](../beyond-core/index.md),
[Security & multi-tenancy](../security.md), and [Operations](../operations.md).

## Upgrades

Migrations are **append-only** — a schema change is a new file, never an edit to
an applied one — so a rolling upgrade stays compatible with the previous schema
during the window where both versions run: additive DDL first, destructive
changes in a later release once all pods are on the new version. Keep
`replicaCount >= 2` (or autoscaling) and the default PodDisruptionBudget so
upgrades and node drains never fully interrupt the API; the default
`terminationGracePeriodSeconds` covers the binary's shutdown drain. Roll back by
re-pinning the prior image tag or digest.

Render and validate the chart before applying with `deploy/helm/validate.sh`
(helm lint + template + the security-field gate + golden-render diff).
