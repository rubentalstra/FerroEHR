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
never `latest`; the chart's `appVersion` is the release it was cut with. Pin them
together: the `config` tree is passed through to the server verbatim, so a key
the chart's defaults carry and your chosen image does not know is a boot refusal
(`unknown configuration key …`). `ferroehr config default` from that exact image
prints the key set it accepts.

That install alone **boots but answers `401` to everything**, deliberately:
`config.auth.enabled` is on and no mechanism is configured yet, and a server that
authenticates nothing is not a safe default. Add an `config.auth.oidc` issuer or
a `config.auth.basic.users` entry before expecting a request to succeed.

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

How a secret reaches the process differs by whether the configuration key has a
`*_file` sibling, and the difference is a security one: an environment variable
is readable through `/proc/<pid>/environ` and is inherited by every child
process, so the [OWASP Kubernetes Security Cheat
Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Kubernetes_Security_Cheat_Sheet.html)
asks for a read-only volume instead.

| Secret | How the chart delivers it |
|---|---|
| `secrets.authOidcHmacSecret` | mounted at `/etc/ferroehr-secrets/auth.oidc.hmac_secret`; only the path is env |
| `secrets.signingKeyPassphrase` | mounted at `/etc/ferroehr-secrets/signing.key_passphrase` |
| `secrets.multimediaSecretAccessKey` | mounted at `/etc/ferroehr-secrets/multimedia.secret_access_key` |
| the database DSN | `FERROEHR__DB__URL` env — `db.url` has no `*_file` sibling |
| `secrets.eventsUrl`, `secrets.fhirOutboundUrl` | env — no `*_file` sibling |
| `secrets.multimediaAccessKeyId` | env — an access key *id* is not secret (it is reported unredacted by `/management/env`) |

The mount is read-only, `0440`, owned `root:65532` so the non-root process reads
it through the group bit, and it is deliberately **not** a `subPath` mount,
because a `subPath`-mounted Secret never receives updates and a rotation would
not propagate. A Basic-auth `password_hash` under `config.auth.basic.users` is
*not* in this table: it is rendered into the ConfigMap, since the server has no
`password_hash_file` indirection — prefer `config.auth.oidc` where that matters.

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
| `enableServiceLinks` | `false` (see below — not a preference) |
| NetworkPolicy | default-deny ingress; only the API (and management) port admitted |

Verified on a running pod rather than inferred from the rendered manifest: the
container runs as uid/gid 65532 with an **empty** capability bounding set,
`noNewPrivileges`, a read-only root filesystem, and a default-deny seccomp
filter; the only writable path is the `emptyDir` at `/tmp`. The whole set
satisfies the Pod Security **Restricted** profile, so the chart installs
unchanged into a namespace labelled
`pod-security.kubernetes.io/enforce=restricted`.

`enableServiceLinks: false` is load-bearing, not hygiene. The kubelet injects a
[set of Service link environment
variables](https://kubernetes.io/docs/concepts/services-networking/service/#environment-variables)
for every Service in the namespace, and for a Service named `ferroehr*` those
land inside the server's reserved `FERROEHR_` namespace — whose strict boot-time
sweep rejects unknown variables and refuses to start. Leaving service links on
makes every install crash-loop.

Egress restriction is opt-in (`networkPolicy.egress.enabled`) because egress
targets — the database, broker, terminology server — are deployment-specific;
when you enable it the chart always admits DNS and you add rules for the rest.
In the default posture the server's only outbound traffic is DNS and PostgreSQL,
so those two rules suffice; each integration you switch on adds a target, and a
blocked one can fail silently.

Two limits worth stating plainly. First, with `networkPolicy.ingressFrom` empty
the rendered ingress rule carries no `from` selector, and a rule without `from`
admits **every** source — other namespaces included. Only the port list is
narrowed in that state, so set `ingressFrom` to your ingress controller for a PHI
workload. Second, a NetworkPolicy is only as real as the CNI that implements it:
on a cluster whose network plugin does not enforce NetworkPolicy the object is
documentation rather than a control, and nothing in Kubernetes reports that.
Confirm it by attempting a connection the policy should refuse.
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

Those annotations are honoured by a Prometheus that discovers targets in its own
scrape configuration. An **operator-managed** Prometheus
(kube-prometheus-stack) ignores them and selects targets through
[ServiceMonitor](https://prometheus-operator.dev/docs/developer/getting-started/)
objects instead, so set `metrics.serviceMonitor.enabled=true` there —
`metrics.serviceMonitor.labels` is where the label your Prometheus'
`serviceMonitorSelector` matches on goes. It needs the `monitoring.coreos.com`
CRDs installed first, or the install fails on an unknown kind.

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
| OTLP telemetry | `config.telemetry.*` | unset | `config.telemetry.otlp_endpoint` is all it takes (the `config` tree is the TOML verbatim); an unset endpoint means the OTel layer is not installed (zero overhead). With `networkPolicy.egress.enabled`, add a rule for the collector — a blocked exporter drops spans without an error. |

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

A change anywhere under `config` rewrites the ConfigMap, whose checksum is a pod
annotation, so `helm upgrade` rolls the pods for a configuration-only change too.
`helm uninstall` removes everything the chart created — the chart declares no
PersistentVolumeClaim, so nothing is left behind; your database, and the Secret
holding its DSN, are yours and survive.

Render and validate the chart before applying with `deploy/helm/validate.sh`
(helm lint + template + the security-field gate + golden-render diff).
