# Kubernetes & Helm

The `deploy/helm/ferroehr` chart deploys FerroEHR as a hardened,
production-shaped Kubernetes workload: non-root, read-only root filesystem,
default-deny ingress, connecting to an **external** PostgreSQL 18. This chapter
covers installing the chart, the database role model it expects, the security
posture it enforces, the health probes, and the optional integrations. It
assumes a cluster at Kubernetes 1.25 or newer.

> [!IMPORTANT]
> There is **no in-chart PostgreSQL**. A CDR stores PHI, so its database must be
> an externally managed, backed-up, point-in-time-recoverable PostgreSQL 18 (a
> managed service or an operator-run cluster). The chart carries only the
> connection string, preferably from an existing Secret.

## Installing

Create a Secret holding the app-role connection string, then install the chart
pointing at it:

```shell
kubectl -n ferroehr create secret generic ferroehr-db \
  --from-literal=FERROEHR__DB__URL='postgres://ferroehr_app:***@pg-host:5432/ferroehr?sslmode=verify-full'

helm install ferroehr deploy/helm/ferroehr -n ferroehr \
  --set database.existingSecret=ferroehr-db \
  --set image.tag=3.17.3
```

Always pin `image.tag` to an immutable version or, better, a `@sha256` digest —
never `latest`; the chart's `appVersion` is the release it was cut with.

The chart carries the server's whole configuration under one key, **`config`**,
which is rendered **verbatim into a `ferroehr.toml`** ConfigMap mounted at
`/etc/ferroehr/ferroehr.toml`. Its keys are therefore exactly the TOML keys of
the [configuration reference](configuration.md) — `config.server.bind`,
`config.authz.rbac.enabled`, `config.spec_profile`, and so on — so anything
that reference documents can be set without waiting for a bespoke chart key.
Secret-bearing scalars are the exception: they live under `secrets:` and are
injected as `FERROEHR_*` environment (which overrides the file). `extraEnv` is
the escape hatch for anything neither surfaces.

## Database roles — who runs migrations

The chart expects a **four-role** PostgreSQL model, so the runtime pod is never
a superuser:

| Role | Purpose |
|---|---|
| owner | owns the database (provisioning only) |
| `ferroehr_migrator` | runs the append-only schema migrations |
| `ferroehr_app` | day-to-day reads and writes — **the running pod connects as this** |
| `ferroehr_reader` | read-only, for replicas and reporting |

The binary calls its migrations on boot, so you choose one of two flows:

- **(a) Grant the runtime DSN the migrator role** — simplest for single-tenant
  or small deployments; the pod migrates itself at startup.
- **(b) Run migrations out of band** with a migrator DSN (a CI step or a
  one-shot `Job`), then start the pods with the lower-privileged `ferroehr_app`
  DSN — recommended for least-privilege production. Gate the Deployment rollout
  on the migration step so two server versions never race the schema.

The chart does not ship a migration Job; `migrations.runByMigratorRole` is an
informational marker surfaced in the install NOTES.

## Secrets and mounted config

Some material is file-shaped rather than a value — ABAC policy files, ATNA TLS
certificates, terminology-server client certificates, a JWKS blob, and the PGP
signing key. Supply these under `config.files`, whose entries the chart mounts
read-only from a Secret at `/etc/ferroehr/<key>` (and which is deliberately
*not* part of the rendered TOML); point the matching in-TOML `*_file` /
`*_path` key at the mounted path. Secret-bearing scalar values go under
`secrets:` — `authOidcHmacSecret`, `signingKeyPassphrase`, `eventsUrl`,
`fhirOutboundUrl`, `multimediaAccessKeyId`, `multimediaSecretAccessKey` — and
the database DSN comes from `database.existingSecret` (key
`database.existingSecretKey`, default `FERROEHR__DB__URL`). None of these ever
reach the ConfigMap.

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

Probes use the always-on, unauthenticated, PHI-free health routes on the main
HTTP port. They need no configuration at all — no management surface, no
access level, nothing to forget:

| Probe | Route | Contract |
|---|---|---|
| liveness | `/health/liveness` | 200 while the process is up; touches no dependency |
| readiness | `/health/readiness` | 200 (UP/DEGRADED) or 503 (DOWN): checks DB ping, migrations applied, audit sender, events — each 1s-bounded |
| startup | `/health/liveness` | gates a slow first boot |

That split is deliberate: a database outage must fail *readiness* (the pod stops
receiving traffic) and never liveness (which would restart the container in a
loop). If the kubelet cannot reach the HTTP port, set `probes.exec.enabled=true`
to use the binary's `healthcheck` subcommand instead.

The management surface is independent of the probes and stays ops-only
(`/management/info`, `/prometheus`, `/metrics`, `/env`, `/loggers`,
`/flamegraph`). Unlike the bare binary, the chart ships
`config.management.enabled: true` with **every endpoint off**, so nothing is
exposed until you opt one in. Set
`config.management.endpoints.prometheus: public` plus `metrics.enabled=true`
to add the `prometheus.io/*` scrape annotations; set `config.management.port`
to serve the surface on its own internal listener, so `/management` is never
reachable on the clinical API port. The health probes stay on the main port
regardless of all of this.

## Optional integrations

Every switch below lives in the chart's `config` tree, so its key *is* the
TOML key from the [configuration reference](configuration.md). Most are **off
by default**; the ones that ship on are marked, and enabling any of the others
is an explicit, auditable decision:

| Integration | Chart key | Default | Notes |
|---|---|---|---|
| Specification generation | `config.spec_profile` | `development` | One coupled choice; `stable` runs the released generations. |
| ADMIN API | `config.admin.enabled` | off | Physical, irreversible delete. Gate behind admin RBAC. |
| Terminology extension API | `config.terminology.api_enabled` | off | 404 when off. |
| Event-subscription API | `config.events.admin_api` | off | Admin CRUD over event filters. |
| Multi-tenancy | `config.tenancy.enabled` | off | Tenant from a JWT claim (`config.tenancy.claim`); never set `tenancy.header` in production. Pairs with PG row-level security. |
| OAuth2/OIDC auth | `config.auth.oidc.*` | unset | Prefer JWKS/discovery over the HS256 `secrets.authOidcHmacSecret`. |
| RBAC | `config.authz.rbac.enabled` | **on** | The coarse role gate (active while `config.auth.enabled`). |
| ABAC | `config.authz.abac.enabled` | off | Cedar (policies via a `config.files` mount) or a remote policy decision point. |
| Eventing → AMQP | `config.events.enabled` | off | Envelopes are **PHI-free** by design. Use `config.events.tls: true`; URL via `secrets.eventsUrl`. |
| FHIR inbound/façade | `config.fhir.api_enabled` | off | Read façade + inbound mapping. |
| FHIR outbound → AMQP | `config.fhir.outbound.enabled` | off | ⚠ **Carries PHI** (the mapped FHIR resource). Separate exchange; TLS broker only; URL via `secrets.fhirOutboundUrl`. |
| S3 multimedia | `config.multimedia.enabled` | off | ⚠ Offloaded blobs are PHI. Private, encrypted, HTTPS bucket; keys via `secrets.multimedia*`. |
| External terminology | `config.terminology.external.enabled` | off | FHIR terminology server; the provider map is more `config.terminology.external.providers.*` keys. |
| ATNA audit trail | `config.audit.enabled` | **on** | On with the local store only; forwarding (`config.audit.syslog`, `config.audit.fhir_feed`) is opt-in per sink. |
| Version signing | `config.signing.*` | **on** (`digest`) | `mode: pgp` needs a `config.files` key plus `secrets.signingKeyPassphrase`, and fails closed at boot without a usable key. |
| OTLP telemetry | `FERROEHR__TELEMETRY__*` via `extraEnv` | unset | The chart surfaces no telemetry key of its own; an unset endpoint means the OTel layer is not installed (zero overhead). |

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
