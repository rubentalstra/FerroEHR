# ferroehr

Pure-Rust, openEHR-conformant clinical data repository (ITS-REST 1.1.0 + AQL 1.1). A single static binary deployed with a hardened-by-default security posture: runs as a non-root, read-only-rootfs workload whose NetworkPolicy admits its serving port only, and that connects to an EXTERNAL PostgreSQL 18 as an unprivileged app role (migrations are run out of band by a separate migrator role).

![Version: 6.0.12](https://img.shields.io/badge/Version-6.0.12-informational?style=flat-square) ![Type: application](https://img.shields.io/badge/Type-application-informational?style=flat-square) ![AppVersion: 4.0.0](https://img.shields.io/badge/AppVersion-4.0.0-informational?style=flat-square)

FerroEHR is a pure-Rust openEHR Clinical Data Repository: ITS-REST 1.1.0 at the
API, AQL 1.1 as the query language, PostgreSQL 18-native storage, shipped as a
single static binary. This chart deploys the server.

## Before you install: three things that surprise people

**This chart does not deploy a database.** It expects an **external PostgreSQL
18** (18.6 or newer) and will not start without one. Point it at your own
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
  --version 6.0.12 \
  --namespace ferroehr --create-namespace \
  --set database.existingSecret=ferroehr-db \
  --set image.tag=4.0.0
```

OCI registries require Helm 3.8 or newer.

### Pin two versions, not one

They are independent SemVer lines and they move independently:

| What | Set with | This release |
|---|---|---|
| the **chart** (templates, defaults, this document) | `--version` | `6.0.12` |
| the **server image** | `image.tag` | `4.0.0` |

`appVersion` is the image the chart defaults to; pinning `image.tag` explicitly
is what keeps an upgrade of one from silently moving the other.

### Verify what you pulled

The chart carries two keyless Sigstore artifacts, and they answer different
questions. A **cosign signature** — who signed this:

```console
cosign verify ghcr.io/rubentalstra/charts/ferroehr:6.0.12 \
  --certificate-identity-regexp '^https://github\.com/rubentalstra/FerroEHR/\.github/workflows/publish-chart\.yml@' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

A **SLSA build provenance attestation** — what source it was built from, and how:

```console
gh attestation verify oci://ghcr.io/rubentalstra/charts/ferroehr:6.0.12 \
  -R rubentalstra/FerroEHR
gh attestation verify oci://ghcr.io/rubentalstra/ferroehr:4.0.0 \
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

A NetworkPolicy ships enabled, and it admits inbound traffic to the API (and
management) port only. **It narrows PORTS, not SOURCES, until you set
`networkPolicy.ingressFrom`**: an ingress rule with no `from` admits every
source, including other namespaces
([NetworkPolicies](https://kubernetes.io/docs/concepts/services-networking/network-policies/)),
so the shipped policy reads as default-deny while admitting everything on that
port. Set `ingressFrom` to your ingress controller, or set
`networkPolicy.ingressAllowAll=false` to have the chart refuse to render the
open state at all. And note that **none of it does anything unless your CNI
enforces NetworkPolicy** — check yours; several do not.

## Requirements

Kubernetes: `>=1.36.0-0`

> [!NOTE]
> Rows under `config.*` carry no description here on purpose. Those keys are the
> **server's**, not the chart's — the chart renders the `config` tree verbatim
> into `ferroehr.toml` — and they are documented once, in the configuration
> reference. Restating them here would fork two copies that drift. The same
> reasoning keeps `config.*` out of `values.schema.json`.
>
> That also means the table is not the boundary of what you can set: **any** key
> in the configuration reference is reachable as `config.<the.toml.path>`,
> whether or not it appears below.

## Values

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| adminUi.affinity | object | `{}` | Affinity. |
| adminUi.auth.oidc.clientId | string | `""` | OAuth2 client id. |
| adminUi.auth.oidc.enabled | bool | `false` | Authenticate console users through OIDC. Off means the console's own session auth is whatever its defaults provide — acceptable for a private cluster, not for anything reachable by a person who should not see PHI. |
| adminUi.auth.oidc.issuer | string | `""` | Issuer URL (must be https for anything but local development). |
| adminUi.auth.oidc.publicBaseUrl | string | `""` | The console's own externally-reachable base URL, used to build the OIDC redirect. Must match what the identity provider has registered. |
| adminUi.enabled | bool | `false` | Deploy the admin console alongside the CDR. |
| adminUi.existingSecret | string | `""` | Name of an existing Secret holding the console's OIDC client secret. The chart mounts that key as a FILE and passes only its path, so the credential never enters the pod's environment — the same discipline the server's DSN uses. |
| adminUi.existingSecretKey | string | `"FERROEHR_ADMIN__AUTH__OIDC__CLIENT_SECRET"` | Key within `existingSecret` carrying the client secret. |
| adminUi.extraEnv | list | `[]` | Extra environment for the console (escape hatch). |
| adminUi.image.digest | string | `""` | Image digest (`sha256:…`); wins over `tag` when set, exactly as the server's `image.digest` does. |
| adminUi.image.pullPolicy | string | `"IfNotPresent"` | Pull policy. |
| adminUi.image.repository | string | `"ghcr.io/rubentalstra/ferroehr-admin-ui"` | Console image repository. |
| adminUi.image.tag | string | `""` | Image tag. Empty falls back to .Chart.appVersion, so the console and the server move together by default. |
| adminUi.ingress.annotations | object | `{}` | Extra annotations for the console Ingress. |
| adminUi.ingress.className | string | `""` | IngressClass name. |
| adminUi.ingress.enabled | bool | `false` | Publish the console through an Ingress. The console is a human-facing web UI, so unlike the API this is the normal way to reach it. |
| adminUi.ingress.hosts | list | `[]` | Hosts and paths. |
| adminUi.ingress.tls | list | `[]` | TLS blocks. |
| adminUi.networkPolicy.enabled | bool | `true` | Install a NetworkPolicy for the console. Egress admits the CDR Service and DNS, and nothing else — the console is a REST client of the CDR by mandate, so that half is enforceable rather than aspirational. Ingress narrows the console's PORT unconditionally and its SOURCES only when `ingressFrom` is set. |
| adminUi.networkPolicy.ingressAllowAll | bool | `true` | Admit every source while `ingressFrom` is empty — the posture this chart SHIPS for the console, stated as a value rather than left implicit in an empty list. Set it to `false` to have the render REFUSED while `ingressFrom` is empty, instead of quietly exposing the console to everything. It only decides the empty case — a non-empty `ingressFrom` always narrows. |
| adminUi.networkPolicy.ingressFrom | list | `[]` | Ingress `from` selectors admitted to the console port. Empty means the rule carries no `from`, which admits EVERY source — other namespaces and off-cluster clients included (https://kubernetes.io/docs/concepts/services-networking/network-policies/). This is the human-facing login surface, so SET this to your ingress-controller namespace/pods. |
| adminUi.nodeSelector | object | `{}` | Node selector. |
| adminUi.podSecurityContext | object | `{"fsGroup":65532,"fsGroupChangePolicy":"OnRootMismatch","runAsGroup":65532,"runAsNonRoot":true,"runAsUser":65532,"seccompProfile":{"type":"RuntimeDefault"},"supplementalGroupsPolicy":"Strict"}` | Pod-level security context. Mirrors the server's; a second workload is where a hardened posture is most easily lost. The user-namespace setting is NOT mirrored here — it is the release-wide `hostUsers` key, because a posture that differs between two workloads of one release is a posture nobody can state. |
| adminUi.preStopSleepSeconds | int | `5` | Lame-duck pause before SIGTERM, in seconds (0 disables) — the same endpoint-propagation race the server's `preStopSleepSeconds` covers. |
| adminUi.replicaCount | int | `1` | Replica count. The console holds session state in process, so more than one replica needs sticky sessions at the ingress or users get logged out on a reroute; left at 1 deliberately. |
| adminUi.resources | object | `{"limits":{"memory":"512Mi"},"requests":{"cpu":"50m","memory":"128Mi"}}` | Resource requests/limits. |
| adminUi.securityContext | object | `{"allowPrivilegeEscalation":false,"capabilities":{"drop":["ALL"]},"privileged":false,"readOnlyRootFilesystem":true,"runAsNonRoot":true,"runAsUser":65532,"seccompProfile":{"type":"RuntimeDefault"}}` | Container-level security context. Mirrors the server's. |
| adminUi.service.port | int | `3000` | Service port. The container always listens on 3000. |
| adminUi.service.type | string | `"ClusterIP"` | Service type for the console. |
| adminUi.terminationGracePeriodSeconds | int | `20` | Termination grace period for the console. Shorter than the server's: it holds no write in flight and nothing to drain but in-flight page renders. |
| adminUi.tolerations | list | `[]` | Tolerations. |
| affinity | object | `{}` | Pod affinity/anti-affinity rules. Empty = none. |
| autoscaling.behavior | object | `{}` | Scaling behaviour, passed through verbatim to `spec.behavior`. Empty leaves the documented defaults, which are already asymmetric in the right direction for a clinical API: scale-up is immediate, scale-down waits out a 300-second stabilization window so a traffic trough cannot tear down capacity that is about to be needed. Set this only to make it MORE conservative — e.g. a `scaleDown.policies` entry capping how many pods may go per minute (https://kubernetes.io/docs/tasks/run-application/horizontal-pod-autoscale/#configurable-scaling-behavior). |
| autoscaling.enabled | bool | `false` | Horizontal pod autoscaling. When on, the chart omits replicas and the HPA owns the count. |
| autoscaling.maxReplicas | int | `6` | Upper bound the HPA may scale to. |
| autoscaling.minReplicas | int | `2` | Lower bound the HPA may scale to. |
| autoscaling.targetCPUUtilizationPercentage | int | `75` | Target average CPU. 0 removes the metric; removing BOTH metrics is refused, since an HPA with none never scales. |
| autoscaling.targetMemoryUtilizationPercentage | int | `0` | Target average memory. 0 removes the metric. A CDR is usually CPU-bound, so this is off by default. |
| config.admin.enabled | bool | `false` |  |
| config.audit.enabled | bool | `true` |  |
| config.audit.store.enabled | bool | `true` |  |
| config.audit.store.retention_days | int | `0` |  |
| config.auth.enabled | bool | `true` |  |
| config.authz.abac.enabled | bool | `false` |  |
| config.authz.rbac.admin_role | string | `"ADMIN"` |  |
| config.authz.rbac.enabled | bool | `true` |  |
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
| extraVolumeMounts | list | `[]` | Mounts for extraVolumes, in the server container. |
| extraVolumes | list | `[]` | Extra volumes / volumeMounts (e.g. an external secret store for the PGP key). |
| fullnameOverride | string | `""` | Override the full resource name. |
| hostUsers | bool | `false` | Run the pods in their own USER NAMESPACE, so container UIDs map to unprivileged host UIDs and a container escape lands as nobody rather than as the UID it ran under (KEP-127, stable v1.36 — the reason this chart's kubeVersion floor is 1.36). `false` here is the Kubernetes field spelling and means user namespaces are ON; set it to `true` to share the host's user namespace, which is the API default.  Set it to `true` if your nodes cannot support it. The requirement is a node-level one the chart cannot check: a Linux node whose container runtime implements it (containerd >= 2.0 or CRI-O >= 1.25) with idmap-mount support in the kernel. On a node without it the pod does not start — a loud failure, not a silent downgrade (https://kubernetes.io/docs/tasks/configure-pod-container/user-namespaces/). |
| image.digest | string | `""` | Image digest (`sha256:…`). Set it and the pod runs `repository@digest`, ignoring `tag` entirely: a digest is what the provenance attestation is made over, so deploying by digest is what makes verification bind to the running image. A tag can be moved afterwards; a digest cannot. |
| image.pullPolicy | string | `"IfNotPresent"` | Pull policy. IfNotPresent + an immutable pinned tag/digest in production. |
| image.repository | string | `"ghcr.io/rubentalstra/ferroehr"` | Image repository. Multi-arch distroless (gcr.io/distroless/cc-debian13:nonroot base). |
| image.tag | string | `""` | Image tag. Empty string falls back to .Chart.appVersion. Pin a version in production, never `latest` — and prefer `digest` below, which a tag cannot be substituted for once it is set. |
| imagePullSecrets | list | `[]` | imagePullSecrets for private registries. |
| ingress.annotations | object | `{}` | Ingress annotations (TLS issuer, body size, timeouts — controller-specific). |
| ingress.className | string | `""` | IngressClass name. Empty leaves the cluster default to apply. |
| ingress.enabled | bool | `false` | Create an Ingress. TLS termination belongs here (or at a gateway). |
| ingress.hosts[0].host | string | `"ferroehr.example.com"` |  |
| ingress.hosts[0].paths[0].path | string | `"/ferroehr"` |  |
| ingress.hosts[0].paths[0].pathType | string | `"Prefix"` |  |
| ingress.tls | list | `[]` | TLS blocks, passed through verbatim. |
| metrics.enabled | bool | `false` | Add prometheus.io scrape annotations to the pods. |
| metrics.serviceMonitor.enabled | bool | `false` | Render a Prometheus Operator ServiceMonitor. Needs the monitoring.coreos.com CRDs installed, or the install fails on an unknown kind. |
| metrics.serviceMonitor.interval | string | `"30s"` | Scrape interval / timeout. |
| metrics.serviceMonitor.labels | object | `{}` | Extra labels, for the `serviceMonitorSelector` your Prometheus matches on. |
| metrics.serviceMonitor.namespace | string | `""` | Namespace for the ServiceMonitor; empty = the release namespace. |
| metrics.serviceMonitor.scrapeTimeout | string | `"10s"` | Per-scrape timeout. Must be shorter than the interval. |
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
| networkPolicy.enabled | bool | `true` | Install a NetworkPolicy that admits inbound traffic to the API (and management) port and nothing else. Strongly recommended for a PHI workload — but it narrows PORTS unconditionally and SOURCES only when `ingressFrom` is set, so "networkPolicy.enabled=true" alone is not a source restriction. |
| networkPolicy.ingressAllowAll | bool | `true` | Admit every source while `ingressFrom` is empty — which is the posture this chart SHIPS, stated as a value rather than left implicit in an empty list. Set it to `false` to make "no open ingress" a machine-checked fact: with `ingressFrom` still empty the render is then REFUSED instead of quietly producing a policy that admits everything while reading as default-deny. It only decides the empty case — a non-empty `ingressFrom` always narrows. |
| networkPolicy.ingressFrom | list | `[]` | Ingress `from` selectors admitted to the API port. Empty means the rule carries no `from` at all, and a NetworkPolicy ingress rule without `from` admits EVERY source — other namespaces and off-cluster clients included, not just this namespace (https://kubernetes.io/docs/concepts/services-networking/network-policies/). Only the port list is narrowed in that state, so SET this to your ingress-controller namespace/pods for a PHI workload. |
| nodeSelector | object | `{}` | Scheduling. |
| podAnnotations | object | `{}` | Extra annotations on the pod template. Note a change here rolls the Deployment. |
| podDisruptionBudget.enabled | bool | `true` | Protect availability during voluntary disruption (drains, upgrades). |
| podDisruptionBudget.minAvailable | int | `1` | Pods that must stay available. Used only when maxUnavailable is unset. |
| podDisruptionBudget.unhealthyPodEvictionPolicy | string | `"AlwaysAllow"` | Whether a node drain may evict pods that are already unhealthy. `AlwaysAllow` is the documented recommendation; the alternative, `IfHealthyBudget`, is the API default and makes a drain wait for pods to become healthy first — which never completes when they are unhealthy because of the drain itself. |
| podLabels | object | `{}` | Extra pod labels / annotations. |
| podSecurityContext.fsGroup | int | `65532` | Supplemental group owning mounted volumes, so a non-root process can read them. |
| podSecurityContext.fsGroupChangePolicy | string | `"OnRootMismatch"` | Only chown volumes whose ownership differs, avoiding a full relabel on every mount. |
| podSecurityContext.runAsGroup | int | `65532` | GID to run as. |
| podSecurityContext.runAsNonRoot | bool | `true` | Refuse to start as root (pod level). Required by the restricted profile. |
| podSecurityContext.runAsUser | int | `65532` | UID to run as (pod level). 65532 is the distroless nonroot user. |
| podSecurityContext.seccompProfile.type | string | `"RuntimeDefault"` | Seccomp profile. RuntimeDefault is required by the restricted profile. |
| podSecurityContext.supplementalGroupsPolicy | string | `"Strict"` | Whether the groups in the IMAGE's /etc/group are merged into the process's supplemental groups. `Strict` uses only the groups this manifest names, so a group baked into an image cannot silently widen file access (KEP-3619, stable v1.35; https://kubernetes.io/docs/tasks/configure-pod-container/security-context/). |
| preStopSleepSeconds | int | `5` | Lame-duck pause before SIGTERM, in seconds (0 disables). Endpoint removal and SIGTERM happen concurrently on pod deletion, so without a pause a terminating pod can still be sent requests. Uses the native `preStop.sleep` action (this image has no shell, so an exec hook cannot run). Must stay comfortably under terminationGracePeriodSeconds: the sleep runs INSIDE that budget and the server still needs its own drain window after it. |
| probes.liveness.failureThreshold | int | `3` | Liveness probe: consecutive failures before the pod is acted on. |
| probes.liveness.initialDelaySeconds | int | `10` | Liveness probe: delay before the first check. |
| probes.liveness.periodSeconds | int | `15` | Liveness probe: seconds between checks. |
| probes.liveness.timeoutSeconds | int | `3` | Liveness probe: per-check timeout. |
| probes.readiness.failureThreshold | int | `3` | Readiness probe: consecutive failures before the pod is acted on. |
| probes.readiness.initialDelaySeconds | int | `5` | Readiness probe: delay before the first check. |
| probes.readiness.periodSeconds | int | `10` | Readiness probe: seconds between checks. |
| probes.readiness.timeoutSeconds | int | `3` | Readiness probe: per-check timeout. |
| probes.startup.enabled | bool | `true` | Startup probe. Gives a slow first boot (migrations) room before liveness applies. |
| probes.startup.failureThreshold | int | `30` | Startup probe: consecutive failures before the pod is acted on. |
| probes.startup.initialDelaySeconds | int | `5` | Startup probe: delay before the first check. |
| probes.startup.periodSeconds | int | `5` | Startup probe: seconds between checks. |
| probes.startup.timeoutSeconds | int | `3` | Startup probe: per-check timeout. |
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
| securityContext.allowPrivilegeEscalation | bool | `false` | Block setuid/file-capability escalation. Required by the restricted profile. |
| securityContext.capabilities.drop[0] | string | `"ALL"` |  |
| securityContext.privileged | bool | `false` | Never true. A privileged container is effectively root on the node. |
| securityContext.readOnlyRootFilesystem | bool | `true` | Immutable root filesystem; writable paths are explicit emptyDir mounts. |
| securityContext.runAsNonRoot | bool | `true` | Refuse to start as root (container level). Required by the restricted profile. |
| securityContext.runAsUser | int | `65532` | UID to run as (container level). 65532 is the distroless nonroot user. |
| securityContext.seccompProfile.type | string | `"RuntimeDefault"` | Seccomp profile at container level. RuntimeDefault is required by the restricted profile. |
| service.annotations | object | `{}` | Extra annotations. |
| service.port | int | `8080` | Public API port. |
| service.trafficDistribution | string | `""` | Topology preference for how the Service picks an endpoint (KEP-4444, stable v1.33). Empty leaves the API default: spread across every ready endpoint cluster-wide. `PreferSameZone` (`PreferClose` is its older spelling) keeps traffic inside the client's zone while endpoints there are ready, which cuts cross-zone latency and inter-zone egress billing; `PreferSameNode` is stricter still. Both trade EVENNESS for locality — a zone with one replica and most of the callers will take most of the load — so it is off unless you have measured that you want it (https://kubernetes.io/docs/reference/networking/virtual-ips/#traffic-distribution). |
| service.type | string | `"ClusterIP"` | Service type. ClusterIP + an Ingress/gateway in front is the norm. |
| serviceAccount.annotations | object | `{}` | Extra annotations (e.g. IRSA/Workload-Identity role bindings for S3). |
| serviceAccount.automountServiceAccountToken | bool | `false` | The workload never calls the K8s API, so no token is mounted. |
| serviceAccount.create | bool | `true` | Create a dedicated ServiceAccount. |
| serviceAccount.name | string | `""` | Name to use; generated when empty. |
| strategy.rollingUpdate.maxSurge | int | `1` | Extra pods allowed above `replicaCount` while rolling. |
| strategy.rollingUpdate.maxUnavailable | int | `0` | Pods allowed to be unavailable while rolling. 0 = capacity never drops. |
| strategy.type | string | `"RollingUpdate"` | Rollout strategy. RollingUpdate with maxUnavailable 0 never drops capacity; Recreate takes the service down. |
| terminationGracePeriodSeconds | int | `30` | Termination grace period (audit/outbox drain has a 5s window in-binary). |
| tolerations | list | `[]` | Tolerations for tainted nodes. Empty = none. |
| topologySpreadConstraints | list | `[]` | Spread replicas across nodes. Empty does NOT mean "no spreading": it means the chart's own default constraint applies — one soft `maxSkew: 1` over `kubernetes.io/hostname`, so two replicas prefer two nodes and a node failure does not take the whole CDR with it. It is `ScheduleAnyway`, not `DoNotSchedule`, so a single-node or capacity-constrained cluster still schedules rather than leaving a pod Pending forever.  A non-empty list REPLACES that default entirely — give the full constraint, including its own `labelSelector`. Add a `topology.kubernetes.io/zone` constraint here if your cluster spans zones; the chart does not assume one (https://kubernetes.io/docs/concepts/scheduling-eviction/topology-spread-constraints/). |

## More

Full deployment documentation, including the least-privilege database roles and
the observability wiring, is at
<https://ferroehr.eu/docs/latest/installation/kubernetes.html>.

----------------------------------------------
Autogenerated from chart metadata using [helm-docs v1.14.2](https://github.com/norwoodj/helm-docs/releases/v1.14.2)
