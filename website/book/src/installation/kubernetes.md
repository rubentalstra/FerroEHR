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

The chart is published to GHCR as an **OCI artifact**, beside the images it
deploys. Create a Secret holding the app-role connection string, then install:

```shell
kubectl create namespace ferroehr
kubectl -n ferroehr create secret generic ferroehr-db \
  --from-literal=FERROEHR__DB__URL='postgres://ferroehr_app:***@pg-host:5432/ferroehr?sslmode=verify-full'

helm install ferroehr oci://ghcr.io/rubentalstra/charts/ferroehr \
  --version 4.0.0 -n ferroehr \
  --set database.existingSecret=ferroehr-db \
  --set image.tag=3.17.3
```

> [!IMPORTANT]
> **`helm repo add` does not work for this chart, and never will.** There is no
> HTTP chart repository and no `index.yaml` — OCI is the only publication path, so
> that there is exactly one place a chart version can exist. The install form is
> the `oci://` reference above. Helm has treated OCI registries as first-class
> since 3.8, so the cost of this choice is real but narrow: a client older than
> Helm 3.8 cannot install this chart at all.

`helm show`, `helm pull`, `helm template` and `helm upgrade` all take the same
`oci://` reference. To read the chart's metadata without installing it:

```shell
helm show chart oci://ghcr.io/rubentalstra/charts/ferroehr --version 4.0.0
```

### Pin two versions, not one

**The chart version and the image tag move independently, and this is the thing
people get wrong first.** `--version` selects the *chart* (its templates and
values schema); `image.tag` selects the *server binary*. The chart's `appVersion`
is only the default for the second, and it names the release the chart was cut
against.

| | Selects | Pin with | Line |
|---|---|---|---|
| Chart version | templates, values schema, defaults | `--version 4.0.0` | SemVer over the chart's own contract |
| Image tag | the server binary | `--set image.tag=3.17.3` (or `image.digest`) | the application's SemVer line |

Always pin the image to an immutable version or, better, a `@sha256` digest —
never `latest`. Pin the two deliberately: the `config` tree is passed through to
the server, so a key the chart's defaults carry and your chosen image does not know
is a boot refusal (`unknown configuration key …`), which presents as
`CrashLoopBackOff`. A chart version is never republished with different content —
the publish lane refuses to overwrite one — so a pinned chart version is a fixed
artifact.

### Verifying what you installed

The chart and the images are published with **signed, keyless
[Sigstore](https://docs.sigstore.dev/) provenance**, bound to this repository's
build identity, so every claim here is checkable rather than asserted:

```shell
# the chart
gh attestation verify oci://ghcr.io/rubentalstra/charts/ferroehr:<chart-version> -R rubentalstra/FerroEHR
# the images it deploys
gh attestation verify oci://ghcr.io/rubentalstra/ferroehr:<tag> -R rubentalstra/FerroEHR
```

> [!IMPORTANT]
> Signing was added to the publishing lanes **after** `3.17.3` was built, so that
> tag and everything before it carry no attestation and the command above answers
> `no attestations found` for them. Attestations exist from the first release
> published by the signing lane onward. If you are pinning an older tag and need
> provenance, pin a newer one.

> [!NOTE]
> `helm install --verify` and `helm verify` do **not** apply: they check a PGP
> `.prov` file, and this chart ships none. That is deliberate — a `.prov` needs a
> long-lived private key in CI, which is the exact thing this project's publishing
> lanes are built to avoid (the crates.io lane uses OIDC Trusted Publishing and
> holds no token at all). Keyless attestation gives a stronger guarantee with no
> key to leak, and it is the same command and the same trust root as the images.

The chart is also listed on **[Artifact
Hub](https://artifacthub.io/packages/helm/ferroehr/ferroehr)**, which renders this
chapter's metadata plus a security report over the published images.

> [!WARNING]
> Between releases the chart's `config` defaults track development and can be
> **ahead of `appVersion`'s image**. Check the pairing before you install, with
> the image itself as the authority:
>
> ```shell
> helm template ferroehr oci://ghcr.io/rubentalstra/charts/ferroehr --version 4.0.0 \
>   -s templates/configmap.yaml --set database.existingSecret=ferroehr-db \
>   | sed -n '/ferroehr.toml/,$p' | sed '1d;s/^    //' > /tmp/ferroehr.toml
> docker run --rm -v /tmp/ferroehr.toml:/etc/ferroehr/ferroehr.toml:ro \
>   -e FERROEHR__DB__URL=postgres://u:p@db:5432/ferroehr \
>   --entrypoint /usr/local/bin/ferroehr ghcr.io/rubentalstra/ferroehr:<tag> config check
> ```
>
> Exit 0 means the image accepts the rendered configuration. A reported unknown
> key means the image is older than the chart's defaults: use a newer tag, or set
> that key to `null` for this deployment. A *published* chart is checked this way
> before it is published — the publish lane refuses to ship a chart whose defaults
> its own `appVersion` image rejects — so this matters mainly when you install from
> a checkout of `develop`, or pin an image older than the chart.

That install alone **boots but answers `401` to everything**, deliberately:
`config.auth.enabled` is on and no mechanism is configured yet, and a server that
authenticates nothing is not a safe default. Add an `config.auth.oidc` issuer or
a `config.auth.basic.users` entry before expecting a request to succeed.

The chart carries the server's whole configuration under one key, **`config`**,
rendered into a `ferroehr.toml` ConfigMap mounted at
`/etc/ferroehr/ferroehr.toml`. Its keys are therefore exactly the TOML keys of
the [configuration reference](configuration.md) — `config.server.bind`,
`config.authz.rbac.enabled`, `config.spec_profile`, and so on — so anything
that reference documents can be set without waiting for a bespoke chart key.

**Secret-bearing keys are the one exception: a credential never reaches that
ConfigMap.** A ConfigMap is not a sensitive object — it is readable with namespace
read, quoted wholesale into issues and support tickets, collected by backup tooling
that skips Secrets, and not covered by Secret encryption at rest. The chart
therefore classifies every key it renders and takes one of two actions.

**A secret the chart routes** — `auth.oidc.hmac_secret`, `signing.key_passphrase`,
`multimedia.secret_access_key`, a terminology `client_secret`, and the four
URL-shaped ones (`db.url`, `events.url`, `fhir.outbound.url`,
`audit.fhir_feed.url`) — has a `secrets:` key of its own, so a value under
`config:` is a mistake and fails the render, naming the key that belongs there:

```text
Error: execution error at (ferroehr/templates/deployment.yaml:18:28):
  refusing to render a secret into the ConfigMap (a ConfigMap is not a sensitive object …):
  - config.auth.oidc.hmac_secret: set `secrets.authOidcHmacSecret` instead
```

**A secret the chart cannot route** — today only a Basic user's
`auth.basic.users[].password_hash`, whose config key has no `password_hash_file`
sibling to point at a mounted file — moves the **whole rendered `ferroehr.toml`
into the chart's Secret, and no ConfigMap is created at all**. The install notes
say which object your release used, and so does the projected volume. Read the
effective configuration in that mode with:

```shell
kubectl -n ferroehr get secret ferroehr-config \
  -o jsonpath='{.data.ferroehr\.toml}' | base64 -d
```

Classification is by **name shape**, not by a list of today's keys: any key whose
name carries `password`, `passphrase`, `secret`, `credential`, `private_key`,
`api_key` or a trailing `token` is treated as a credential unless it ends in
`_file`, `_path` or `_dir` (those hold a path), and the URL-shaped secrets are
matched by path. That is what makes a secret key added to the server's
configuration tree in a future release move to the Secret rather than leak
silently. `extraEnv` is the escape hatch for anything neither `config:` nor
`secrets:` surfaces.

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
`fhirOutboundUrl`, `auditFhirFeedUrl`, `multimediaAccessKeyId`,
`multimediaSecretAccessKey`, `terminologyOauth2ClientSecrets` — and the database
DSN comes from `database.existingSecret` (key `database.existingSecretKey`,
default `FERROEHR__DB__URL`). None of these ever reach the ConfigMap.

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
| `secrets.terminologyOauth2ClientSecrets.<name>` | mounted at `/etc/ferroehr-secrets/terminology.external.oauth2_clients.<name>.client_secret`; the chart injects the matching `client_secret_file` into the rendered TOML |
| the database DSN | `FERROEHR__DB__URL` env — `db.url` has no `*_file` sibling |
| `secrets.eventsUrl`, `secrets.fhirOutboundUrl`, `secrets.auditFhirFeedUrl` | env — no `*_file` sibling |
| `secrets.multimediaAccessKeyId` | env — an access key *id* is not secret (it is reported unredacted by `/management/env`) |

The mount is read-only, `0440`, owned `root:65532` so the non-root process reads
it through the group bit, and it is deliberately **not** a `subPath` mount,
because a `subPath`-mounted Secret never receives updates and a rotation would
not propagate.

> [!NOTE]
> A Basic user is the one secret with no `*_file` route: the server has no
> `password_hash_file`, so an Argon2id hash cannot be delivered as a mounted file.
> Configuring `config.auth.basic.users` therefore switches the whole configuration
> into the Secret (above) rather than leaving the hash in a ConfigMap. It is still
> not as good as a mounted file — the value sits in the pod's configuration object
> instead of a rotatable secret path — so prefer `config.auth.oidc` where the
> choice exists. An Argon2id hash is not a plaintext password, but it is an offline
> cracking target, which is exactly what the boot-time OWASP parameter floor exists
> to make expensive.

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

> [!WARNING]
> **Migrations run only at boot, so a replaced or wiped database leaves the pods
> NotReady until something migrates it.** Readiness reports
> `"migrations": {"status":"DOWN","detail":"core schema tables missing (migrations
> not applied)"}` while `"db"` reads `UP` — the pod reaches PostgreSQL, finds no
> schema, and does not migrate again, because migration is a startup step.
> Liveness keeps passing (correctly: the process is healthy), so the kubelet never
> restarts the container and the Deployment sits at `0/N` ready with no error in
> the logs after the first one:
>
> ```shell
> kubectl -n ferroehr get pods                      # Running, 0/2 READY
> curl -s http://ferroehr:8080/health/readiness     # 503, "migrations" DOWN, "db" UP
> kubectl -n ferroehr rollout restart deploy/ferroehr
> ```
>
> The check re-tests the schema on every probe, so a pod recovers on its own within
> one `readiness.periodSeconds` of the schema existing — verified on a two-replica
> Deployment: after the schema was dropped both pods went `READY false` with zero
> restarts, and when a *replacement* pod migrated at boot, the untouched pod
> returned to `READY true` with `RESTARTS 0` and its original start time. What
> needs the restart is the case where the only thing that would migrate is the pod
> itself.
>
> That is what makes the interaction with flow (b) load-bearing: when migrations
> run out of band, the migration step must complete *before* the Deployment rolls,
> or the first pods sit unready waiting for a schema. Gate the rollout on the
> migration Job rather than starting both together. Durable storage is the other
> half — an `emptyDir`-backed or otherwise disposable PostgreSQL puts a clinical
> repository one node eviction away from this state.
>
> One recovery path does **not** self-heal, and it is worth knowing before you try
> it: restoring or dropping *part* of the schema set. The archival tier's tables
> live in their own `cold` schema, so a `DROP SCHEMA ehr CASCADE` leaves them
> behind, and the baseline migration then fails permanently with
> `relation "vo_version" already exists` — the pod crash-loops, and restarting it
> retries the same failure. Recreate the whole database rather than one schema.

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

A change anywhere under `config`, `config.files` or `secrets` changes the
`checksum/config` pod annotation, so `helm upgrade` rolls the pods for a
configuration-only change too — including a rotated secret or an edited ABAC
policy, both of which are read at boot and would otherwise reach the volume while
every running pod kept using the old value.
`helm uninstall` removes everything the chart created — the chart declares no
PersistentVolumeClaim, so nothing is left behind; your database, and the Secret
holding its DSN, are yours and survive.

Preview an upgrade against what you have installed with
`helm diff`, or render the new chart version and read it:

```shell
helm template ferroehr oci://ghcr.io/rubentalstra/charts/ferroehr --version 4.0.0 \
  -n ferroehr -f my-values.yaml | less
```

Working from a checkout instead, `deploy/helm/validate.sh` runs the chart's full
gate — the helm-version pin, the secret-leak gate, lint, render, the
security-field assertions and the golden-render diff. It is the same gate CI runs
on every change to the chart, so a local run and a pull request agree by
construction.
