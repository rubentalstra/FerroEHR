# ferroehr

Pure-Rust, openEHR-conformant clinical data repository (ITS-REST 1.1.0 + AQL 1.1). A single static binary deployed with a hardened-by-default security posture: runs as a non-root, read-only-rootfs, default-deny-ingress workload that connects to an EXTERNAL PostgreSQL 18 as an unprivileged app role (migrations are run out of band by a separate migrator role).

![Version: 5.1.0](https://img.shields.io/badge/Version-5.1.0-informational?style=flat-square) ![Type: application](https://img.shields.io/badge/Type-application-informational?style=flat-square) ![AppVersion: 3.17.3](https://img.shields.io/badge/AppVersion-3.17.3-informational?style=flat-square)

FerroEHR is a pure-Rust openEHR Clinical Data Repository: ITS-REST 1.1.0 at the
API, AQL 1.1 as the query language, PostgreSQL 18-native storage, shipped as a
single static binary. This chart deploys the server.

## Before you install: three things that surprise people

**This chart does not deploy a database.** It expects an **external PostgreSQL
18** (18.4 or newer) and will not start without one. Point it at your own
instance, a managed service, or a separate PostgreSQL chart — `database.url` or,
for production, `database.existingSecret`.

**With no authentication mechanism configured, the server boots and answers
`401` to everything.** `auth.enabled` defaults to `true`, and a deployment with
no `[auth.oidc]` issuer and no Basic user has nothing to authenticate against.
That is fail-closed and deliberate: the alternative is an openEHR repository
serving patient data to anonymous callers. Configure a mechanism, or set
`config.auth.enabled: false` for a throwaway evaluation.

**A secret set in the wrong place fails the render on purpose.** See
[Secrets](#secrets) — this chart refuses to put a credential in a ConfigMap
rather than doing it quietly.

## Install

The chart is published as an **OCI artifact**. There is no chart repository to
add — `helm repo add` does not apply to this chart and never will:

```console
helm install ferroehr oci://ghcr.io/rubentalstra/charts/ferroehr \
  --version 5.1.0 \
  --namespace ferroehr --create-namespace \
  --set database.existingSecret=ferroehr-db \
  --set image.tag=3.17.3
```

OCI registries require Helm 3.8 or newer.

### Pin two versions, not one

They are independent SemVer lines and they move independently:

| What | Set with | This release |
|---|---|---|
| the **chart** (templates, defaults, this document) | `--version` | `5.1.0` |
| the **server image** | `image.tag` | `3.17.3` |

`appVersion` is the image the chart defaults to; pinning `image.tag` explicitly
is what keeps an upgrade of one from silently moving the other.

### Verify what you pulled

The chart carries two keyless Sigstore artifacts, and they answer different
questions. A **cosign signature** — who signed this:

```console
cosign verify ghcr.io/rubentalstra/charts/ferroehr:5.1.0 \
  --certificate-identity-regexp '^https://github\.com/rubentalstra/FerroEHR/\.github/workflows/publish-chart\.yml@' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

A **SLSA build provenance attestation** — what source it was built from, and how:

```console
gh attestation verify oci://ghcr.io/rubentalstra/charts/ferroehr:5.1.0 \
  -R rubentalstra/FerroEHR
gh attestation verify oci://ghcr.io/rubentalstra/ferroehr:3.17.3 \
  -R rubentalstra/FerroEHR
```

`helm install --verify` does **not** apply: it reads a PGP `.prov` provenance
file, and this chart ships none — deliberately, because keeping a long-lived
private key in CI is a worse posture than not having one. The two commands above
are what replace it.

### Your values file is validated

The chart ships a `values.schema.json`, so `helm install`, `upgrade`, `lint` and
`template` refuse a values file that misspells a key of the chart's own
vocabulary, gets a type wrong, or names a value outside the permitted set —
instead of rendering and ignoring it.

Everything under `config:` stays deliberately open: that vocabulary is the
**server's** (`ferroehr config default` prints it in full), it is validated by
the binary at boot, and duplicating it here would fork it. So a typo under
`config:` is caught when the pod starts, not when the chart renders.

## Secrets

Every value under `secrets:` is carried by a chart-managed Secret, and **how it
reaches the process differs by design**. A secret with a `*_file` sibling in the
server's configuration is **mounted read-only** at
`/etc/ferroehr-secrets/<config.path>`, and only the *path* is passed as an
environment variable — because an environment variable is readable through
`/proc/<pid>/environ` and inherited by every child process. The few keys with no
`*_file` sibling still pass their value as env; that is a gap in the
configuration tree, not a choice this chart made.

**Setting a secret under `config:` refuses to render.** Values under `config:`
become a ConfigMap, which is not a sensitive object — it is readable with
namespace read, collected by backup tooling that skips Secrets, and unencrypted
at rest even where Secret encryption is enabled. The refusal names the
`secrets:` key that carries the value safely.

**A secret with no `secrets:` route at all moves the whole configuration into the
Secret**, and then **no ConfigMap is created**. This is how a Helm-configured
Basic user's Argon2id hash is delivered. If your automation reads
`kubectl get configmap <release>`, read the Secret instead.

The classification is by **name shape**, not a fixed list, so a secret-looking
key added to the server's configuration tomorrow is caught without anyone
remembering to update this chart.

## Hardened by default

The pod runs as uid/gid 65532 with `runAsNonRoot`, an empty capability set,
`allowPrivilegeEscalation: false`, `readOnlyRootFilesystem: true`, and
`seccompProfile: RuntimeDefault` — the Pod Security Standards **Restricted**
profile. `automountServiceAccountToken` is off because the workload never calls
the Kubernetes API, and `enableServiceLinks` is off because the kubelet's
injected `FERROEHR_*` Service variables would otherwise collide with the
server's own configuration namespace and stop it booting.

A default-deny-ingress NetworkPolicy ships enabled. **It does nothing unless
your CNI enforces NetworkPolicy** — check yours; several do not.

## Requirements

Kubernetes: `>=1.25.0-0`

## Values

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| affinity | object | `{}` |  |
| autoscaling.enabled | bool | `false` |  |
| autoscaling.maxReplicas | int | `6` |  |
| autoscaling.minReplicas | int | `2` |  |
| autoscaling.targetCPUUtilizationPercentage | int | `75` |  |
| autoscaling.targetMemoryUtilizationPercentage | int | `0` |  |
| config.admin.enabled | bool | `false` |  |
| config.audit.enabled | bool | `true` |  |
| config.audit.store.enabled | bool | `true` |  |
| config.audit.store.retention_days | int | `0` |  |
| config.auth.enabled | bool | `true` |  |
| config.authz.abac.enabled | bool | `false` |  |
| config.authz.rbac.admin_role | string | `"ADMIN"` |  |
| config.authz.rbac.enabled | bool | `true` |  |
| config.authz.rbac.management_access | string | `"admin_only"` |  |
| config.authz.rbac.user_role | string | `"USER"` |  |
| config.db.acquire_timeout_secs | int | `30` |  |
| config.db.max_connections | int | `10` |  |
| config.db.migrate | string | `"apply"` |  |
| config.db.min_connections | int | `0` |  |
| config.db.statement_timeout_ms | int | `60000` |  |
| config.events.enabled | bool | `false` |  |
| config.events.exchange | string | `"ferroehr.events"` |  |
| config.events.tls | bool | `false` |  |
| config.fhir.api_enabled | bool | `false` |  |
| config.fhir.outbound.enabled | bool | `false` |  |
| config.fhir.outbound.exchange | string | `"ferroehr.fhir"` |  |
| config.fhir.outbound.tls | bool | `false` |  |
| config.files | object | `{}` |  |
| config.log.format | string | `"json"` |  |
| config.management.access_default | string | `"admin_only"` |  |
| config.management.base_path | string | `"/management"` |  |
| config.management.enabled | bool | `true` |  |
| config.management.endpoints.env | string | `"off"` |  |
| config.management.endpoints.flamegraph | string | `"off"` |  |
| config.management.endpoints.info | string | `"off"` |  |
| config.management.endpoints.loggers | string | `"off"` |  |
| config.management.endpoints.metrics | string | `"off"` |  |
| config.management.endpoints.prometheus | string | `"off"` |  |
| config.multimedia.allow_http | bool | `false` |  |
| config.multimedia.bucket | string | `"openehr-multimedia"` |  |
| config.multimedia.enabled | bool | `false` |  |
| config.multimedia.region | string | `"us-east-1"` |  |
| config.server.base_path | string | `"/ferroehr/rest/openehr/v1"` |  |
| config.server.bind | string | `"0.0.0.0:8080"` |  |
| config.server.connection.header_read_timeout_secs | int | `10` |  |
| config.server.connection.http2_keep_alive_interval_secs | int | `30` |  |
| config.server.connection.http2_keep_alive_timeout_secs | int | `10` |  |
| config.server.connection.max_concurrent_streams | int | `256` |  |
| config.server.cors_permissive | bool | `false` |  |
| config.server.limits.body_bytes | int | `16777216` |  |
| config.server.limits.bulk_body_bytes | int | `67108864` |  |
| config.server.rate_limit.address_burst | int | `4096` |  |
| config.server.rate_limit.address_per_second | int | `2048` |  |
| config.server.rate_limit.enabled | bool | `true` |  |
| config.server.rate_limit.principal_burst | int | `2048` |  |
| config.server.rate_limit.principal_per_second | int | `1024` |  |
| config.server.swagger_ui | bool | `true` |  |
| config.signing.enabled | bool | `true` |  |
| config.signing.mode | string | `"digest"` |  |
| config.signing.verify_on_read | string | `"strict"` |  |
| config.spec_profile | string | `"development"` |  |
| config.tenancy.claim | string | `"tenant"` |  |
| config.tenancy.enabled | bool | `false` |  |
| config.terminology.api_enabled | bool | `false` |  |
| config.terminology.external.enabled | bool | `false` |  |
| config.terminology.external.fail_on_error | bool | `false` |  |
| database.existingSecret | string | `""` | Reference an existing Secret holding the app-role DSN (STRONGLY preferred for production — keeps the credential out of chart values and git). The secret's value must be a full `postgres://ferroehr_app:...@host:5432/ferroehr` (optionally `?sslmode=verify-full`). |
| database.existingSecretKey | string | `"FERROEHR__DB__URL"` | Key WITHIN existingSecret that holds the DSN. This is a Secret key name, not an environment variable name: the chart mounts that key as a file and passes only its PATH as `FERROEHR__DB__URL_FILE`, so the DSN never enters the pod's environment. The default spelling is kept for compatibility with existing Secrets created for the older env-borne arrangement. |
| database.url | string | `""` | Inline DSN (DEV/TEST ONLY — lands in a chart-managed Secret). Leave empty and use existingSecret in production. Ignored when existingSecret is set. |
| extraEnv | list | `[]` | Extra raw env vars (list of {name,value} or {name,valueFrom}). Escape hatch for anything not surfaced above (array-valued keys via comma-separated values, one-off FERROEHR_* overrides). |
| extraEnvFrom | list | `[]` | Extra envFrom sources (configMapRef/secretRef). |
| extraVolumeMounts | list | `[]` |  |
| extraVolumes | list | `[]` | Extra volumes / volumeMounts (e.g. an external secret store for the PGP key). |
| fullnameOverride | string | `""` | Override the full resource name. |
| image.digest | string | `""` | Image digest (`sha256:…`). Set it and the pod runs `repository@digest`, ignoring `tag` entirely: a digest is what the provenance attestation is made over, so deploying by digest is what makes verification bind to the running image. A tag can be moved afterwards; a digest cannot. |
| image.pullPolicy | string | `"IfNotPresent"` | Pull policy. IfNotPresent + an immutable pinned tag/digest in production. |
| image.repository | string | `"ghcr.io/rubentalstra/ferroehr"` | Image repository. Multi-arch distroless (gcr.io/distroless/cc-debian12:nonroot base). |
| image.tag | string | `""` | Image tag. Empty string falls back to .Chart.appVersion. Pin a version in production, never `latest` — and prefer `digest` below, which a tag cannot be substituted for once it is set. |
| imagePullSecrets | list | `[]` | imagePullSecrets for private registries. |
| ingress.annotations | object | `{}` |  |
| ingress.className | string | `""` |  |
| ingress.enabled | bool | `false` | Create an Ingress. TLS termination belongs here (or at a gateway). |
| ingress.hosts[0].host | string | `"ferroehr.example.com"` |  |
| ingress.hosts[0].paths[0].path | string | `"/ferroehr"` |  |
| ingress.hosts[0].paths[0].pathType | string | `"Prefix"` |  |
| ingress.tls | list | `[]` |  |
| metrics.enabled | bool | `false` | Add prometheus.io scrape annotations to the pods. |
| metrics.serviceMonitor.enabled | bool | `false` | Render a Prometheus Operator ServiceMonitor. Needs the monitoring.coreos.com CRDs installed, or the install fails on an unknown kind. |
| metrics.serviceMonitor.interval | string | `"30s"` | Scrape interval / timeout. |
| metrics.serviceMonitor.labels | object | `{}` | Extra labels, for the `serviceMonitorSelector` your Prometheus matches on. |
| metrics.serviceMonitor.namespace | string | `""` | Namespace for the ServiceMonitor; empty = the release namespace. |
| metrics.serviceMonitor.scrapeTimeout | string | `"10s"` |  |
| migrations.job.activeDeadlineSeconds | int | `600` | Hard ceiling on the migration step; a migration blocked behind live traffic must fail the release rather than hang it. |
| migrations.job.backoffLimit | int | `3` | Retries before the Job (and therefore the release) is declared failed. |
| migrations.job.enabled | bool | `false` | Run the migrations as a pre-install/pre-upgrade hook Job under the migrator DSN. Pair it with config.db.migrate=verify and an app-role-only runtime DSN for the least-privilege posture. |
| migrations.job.existingSecret | string | `""` | REQUIRED when enabled: an existing Secret holding the MIGRATOR DSN (`postgres://ferroehr_migrator:...@host:5432/ferroehr`). Deliberately a different credential from `database.*`, which carries the runtime app-role DSN — rendering fails if it is empty. |
| migrations.job.existingSecretKey | string | `"FERROEHR__DB__URL"` | Key WITHIN existingSecret holding the migrator DSN. Mounted as a file; only its PATH reaches the pod's environment. |
| migrations.job.nodeSelector | object | `{}` | Node selector for the migration pod. |
| migrations.job.podAnnotations | object | `{}` | Extra annotations on the migration pod. |
| migrations.job.resources | object | `{}` | Resource requests/limits for the migration pod. |
| migrations.job.tolerations | list | `[]` | Tolerations for the migration pod. |
| migrations.job.ttlSecondsAfterFinished | int | `600` | How long the finished Job's pod is kept for its logs. |
| migrations.runByMigratorRole | bool | `true` | Purely informational marker rendered into NOTES for the operator. |
| nameOverride | string | `""` | Override the chart name portion of resource names. |
| networkPolicy.egress.database | object | `{"port":5432,"to":[]}` | The database, which is NOT optional: the server cannot pass readiness without it. Rendering an egress policy with no database destination is a refusal, not a warning (see the template) — an egress policy that forgets the DSN is a total outage that looks like a database failure. `to` takes raw NetworkPolicyPeer entries: a `podSelector`/ `namespaceSelector` for an in-cluster database, or an `ipBlock` for a managed one. `port` is the DSN's port. |
| networkPolicy.egress.enabled | bool | `false` | Refuse all outbound traffic except DNS, `database` and `rules`. |
| networkPolicy.egress.rules | list | `[]` | Every other destination, as raw NetworkPolicyEgressRule entries — one per integration you have switched on. See the book's table. |
| networkPolicy.enabled | bool | `true` | Install a default-deny-ingress NetworkPolicy that only admits traffic to the API (and management) port. Strongly recommended for a PHI workload. |
| networkPolicy.ingressFrom | list | `[]` | Ingress `from` selectors admitted to the API port. Empty means the rule carries no `from` at all, and a NetworkPolicy ingress rule without `from` admits EVERY source — other namespaces and off-cluster clients included, not just this namespace (https://kubernetes.io/docs/concepts/services-networking/network-policies/). Only the port list is narrowed in that state, so SET this to your ingress-controller namespace/pods for a PHI workload. |
| nodeSelector | object | `{}` | Scheduling. |
| podAnnotations | object | `{}` |  |
| podDisruptionBudget.enabled | bool | `true` |  |
| podDisruptionBudget.minAvailable | int | `1` |  |
| podLabels | object | `{}` | Extra pod labels / annotations. |
| podSecurityContext.fsGroup | int | `65532` |  |
| podSecurityContext.fsGroupChangePolicy | string | `"OnRootMismatch"` |  |
| podSecurityContext.runAsGroup | int | `65532` |  |
| podSecurityContext.runAsNonRoot | bool | `true` |  |
| podSecurityContext.runAsUser | int | `65532` |  |
| podSecurityContext.seccompProfile.type | string | `"RuntimeDefault"` |  |
| probes.exec | object | `{"enabled":false}` | Use `ferroehr healthcheck` exec probes instead of httpGet. |
| probes.liveness.failureThreshold | int | `3` |  |
| probes.liveness.initialDelaySeconds | int | `10` |  |
| probes.liveness.periodSeconds | int | `15` |  |
| probes.liveness.timeoutSeconds | int | `3` |  |
| probes.readiness.failureThreshold | int | `3` |  |
| probes.readiness.initialDelaySeconds | int | `5` |  |
| probes.readiness.periodSeconds | int | `10` |  |
| probes.readiness.timeoutSeconds | int | `3` |  |
| probes.startup.enabled | bool | `true` |  |
| probes.startup.failureThreshold | int | `30` |  |
| probes.startup.initialDelaySeconds | int | `5` |  |
| probes.startup.periodSeconds | int | `5` |  |
| probes.startup.timeoutSeconds | int | `3` |  |
| replicaCount | int | `2` | Number of replicas (ignored when autoscaling.enabled is true). |
| resources | object | `{"limits":{"cpu":"2","memory":"1Gi"},"requests":{"cpu":"250m","memory":"256Mi"}}` | Resource requests/limits. Sized for a modest API replica; tune for load. |
| secrets.auditFhirFeedUrl | string | `""` | FHIR base URL of the external Audit Record Repository for [audit.fhir_feed] (may carry basic-auth credentials in its userinfo) → FERROEHR__AUDIT__FHIR_FEED__URL env: audit.fhir_feed.url has no `*_file` sibling either. |
| secrets.authOidcHmacSecret | string | `""` | Symmetric HS256 secret for [auth.oidc] (dev/test). MOUNTED as /etc/ferroehr-secrets/auth.oidc.hmac_secret (auth.oidc.hmac_secret_file). |
| secrets.basicUserPasswordHashes | object | `{}` | Argon2id password hashes for [[auth.basic.users]], keyed by username. Each is MOUNTED as /etc/ferroehr-secrets/auth.basic.users.<username>.password_hash and the chart injects the matching `password_hash_file` into the rendered TOML. Declare the user itself — `username`, `roles` — under config.auth.basic.users; a username with no matching entry is a render error. A hash under `config:` is refused: it would reach the ConfigMap. |
| secrets.eventsUrl | string | `""` | AMQP broker URL for [events] (carries credentials). MOUNTED as /etc/ferroehr-secrets/events.url (events.url_file). |
| secrets.fhirOutboundUrl | string | `""` | AMQP broker URL for [fhir.outbound] (carries credentials). MOUNTED as /etc/ferroehr-secrets/fhir.outbound.url (fhir.outbound.url_file). |
| secrets.multimediaAccessKeyId | string | `""` | S3 access key id for [multimedia] → FERROEHR__MULTIMEDIA__ACCESS_KEY_ID env. Not a secret in the server's own model (it is reported unredacted by /management/env); prefer IRSA/Workload-Identity (leave empty) on cloud. |
| secrets.multimediaSecretAccessKey | string | `""` | S3 secret access key for [multimedia]. MOUNTED as /etc/ferroehr-secrets/multimedia.secret_access_key (multimedia.secret_access_key_file). |
| secrets.signingKeyPassphrase | string | `""` | PGP key passphrase (config.signing.mode=pgp). MOUNTED as /etc/ferroehr-secrets/signing.key_passphrase (signing.key_passphrase_file). |
| secrets.terminologyOauth2ClientSecrets | object | `{}` | OAuth2 client secrets for [terminology.external.oauth2_clients.<name>], keyed by client name. Each is MOUNTED as /etc/ferroehr-secrets/terminology.external.oauth2_clients.<name>.client_secret and the chart injects the matching `client_secret_file` into the rendered TOML (a path is not sensitive). Declare the client itself — token_url, client_id, scopes — under config.terminology.external.oauth2_clients.<name>; a name with no such declaration is a render error. |
| securityContext.allowPrivilegeEscalation | bool | `false` |  |
| securityContext.capabilities.drop[0] | string | `"ALL"` |  |
| securityContext.privileged | bool | `false` |  |
| securityContext.readOnlyRootFilesystem | bool | `true` |  |
| securityContext.runAsNonRoot | bool | `true` |  |
| securityContext.runAsUser | int | `65532` |  |
| securityContext.seccompProfile.type | string | `"RuntimeDefault"` |  |
| service.annotations | object | `{}` | Extra annotations. |
| service.port | int | `8080` | Public API port. |
| service.type | string | `"ClusterIP"` | Service type. ClusterIP + an Ingress/gateway in front is the norm. |
| serviceAccount.annotations | object | `{}` | Extra annotations (e.g. IRSA/Workload-Identity role bindings for S3). |
| serviceAccount.automountServiceAccountToken | bool | `false` | The workload never calls the K8s API, so no token is mounted. |
| serviceAccount.create | bool | `true` | Create a dedicated ServiceAccount. |
| serviceAccount.name | string | `""` | Name to use; generated when empty. |
| strategy.rollingUpdate.maxSurge | int | `1` | Extra pods allowed above `replicaCount` while rolling. |
| strategy.rollingUpdate.maxUnavailable | int | `0` | Pods allowed to be unavailable while rolling. 0 = capacity never drops. |
| strategy.type | string | `"RollingUpdate"` |  |
| terminationGracePeriodSeconds | int | `30` | Termination grace period (audit/outbox drain has a 5s window in-binary). |
| tolerations | list | `[]` |  |
| topologySpreadConstraints | list | `[]` | Spread replicas across nodes/zones. Empty = none. |

## More

Full deployment documentation, including the least-privilege database roles and
the observability wiring, is at
<https://ferroehr.eu/docs/latest/installation/kubernetes.html>.

----------------------------------------------
Autogenerated from chart metadata using [helm-docs v1.14.2](https://github.com/norwoodj/helm-docs/releases/v1.14.2)
