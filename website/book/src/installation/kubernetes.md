# Kubernetes & Helm

The `ferroehr` Helm chart deploys FerroEHR as a hardened, production-shaped
Kubernetes workload (non-root, read-only root filesystem, a NetworkPolicy that
admits its serving port only, its own user namespace) against an **external**
PostgreSQL 18. This chapter
covers installing it, verifying what you installed, the database role model it
expects, the security posture it pins, the health probes, the optional
integrations, and upgrades.

<!-- toc -->

> [!IMPORTANT]
> **The chart requires Kubernetes 1.36 or newer** (`kubeVersion: ">=1.36.0-0"`).
> That is a compatibility floor, not a support opinion: 1.36 is the release where
> the newest field the chart renders (`hostUsers`, which gives every pod its own
> user namespace) became stable, so nothing has to be version-gated into silence.
> Your nodes must be Linux with containerd 2.0 or newer, or CRI-O 1.25 or newer;
> without that support the pod does not start, which is the loud failure rather
> than a silent downgrade. Set `hostUsers: true` to opt out and share the host's
> user namespace. What the user namespace buys is in
> [Cluster hardening](./kubernetes-hardening.md).

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
  --version 6.0.17 -n ferroehr \
  --set database.existingSecret=ferroehr-db \
  --set image.tag=4.0.3
```

> [!IMPORTANT]
> **`helm repo add` does not work for this chart, and never will.** There is no
> HTTP chart repository and no `index.yaml`: OCI is the only publication path, so
> that there is exactly one place a chart version can exist. Every command takes
> the `oci://` reference above. Helm has treated OCI registries as first-class
> since 3.8, so the cost of this choice is real but narrow: a client older than
> Helm 3.8 cannot install this chart at all.

`helm show`, `helm pull`, `helm template` and `helm upgrade` all take the same
reference. To read the chart's metadata without installing it:

```shell
helm show chart oci://ghcr.io/rubentalstra/charts/ferroehr --version 6.0.17
```

### Pin two versions, not one

**The chart version and the image tag move independently, and this is the thing
people get wrong first.** `--version` selects the *chart* (its templates and
values schema); `image.tag` selects the *server binary*. The chart's `appVersion`
is only the default for the second, and it names the release the chart was cut
against.

| | Selects | Pin with | Line |
|---|---|---|---|
| Chart version | templates, values schema, defaults | `--version 6.0.17` | SemVer over the chart's own contract |
| Image tag | the server binary | `--set image.tag=4.0.3` (or `image.digest`) | the application's SemVer line |

Always pin the image to an immutable version or, better, a `@sha256` digest,
never `latest`. Pin the two deliberately: the `config` tree is passed through to
the server, so a key the chart's defaults carry and your chosen image does not know
is a boot refusal (`unknown configuration key …`), which presents as
`CrashLoopBackOff`. A chart version is never republished with different content
(the publish lane refuses to overwrite one) so a pinned chart version is a fixed
artifact.

> [!NOTE]
> A published chart version being immutable is a property of that refusal, not of
> the registry: an OCI tag is mutable, and `helm push` over an existing one would
> silently replace it. That is why a correction always ships as a new chart
> version, and why the `6.0.3` → `6.0.4` bump exists: the chart's `appVersion`
> moved to a new release while its own version had already been published.

### Verifying what you installed

Everything published here is signed **keyless** through
[Sigstore](https://docs.sigstore.dev/), bound to this repository's build
identity, so every claim is checkable rather than asserted. Two different
artifacts answer two different questions, and you can ask both.

**Who signed this chart:** a [cosign](https://docs.sigstore.dev/cosign/)
signature over the chart's digest:

```shell
cosign verify ghcr.io/rubentalstra/charts/ferroehr:<chart-version> \
  --certificate-identity-regexp '^https://github\.com/rubentalstra/FerroEHR/\.github/workflows/build-chart\.yml@' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

Both flags matter. Without them `cosign verify` would accept a signature from
*any* identity in the transparency log; with them you are requiring that this
repository's chart-publishing workflow, authenticated by GitHub's OIDC issuer,
is what signed the bytes you pulled.

**What it was built from:** a SLSA build provenance attestation:

```shell
# the chart
gh attestation verify oci://ghcr.io/rubentalstra/charts/ferroehr:<chart-version> -R rubentalstra/FerroEHR
# the image it deploys
gh attestation verify oci://ghcr.io/rubentalstra/ferroehr:<tag> -R rubentalstra/FerroEHR
```

The publish lane reads both back from the registry before it reports success, so
a run that produced only one of them fails instead of going green.

> [!IMPORTANT]
> Both commands verify what the publishing lanes produce now. A published
> artifact is never replaced, so if one you pinned answers `HTTP 404: Not Found`
> it carries no attestation, which is the honest state rather than a verification
> failure, and the fix is to pin a current version. Image signing landed during
> the `3.17.4` cycle, so image tags from before it have nothing to
> verify.

> [!NOTE]
> `helm install --verify` and `helm verify` do **not** apply: they check a PGP
> `.prov` file, and this chart ships none. That is deliberate: a `.prov` needs a
> long-lived private key in CI, which is the exact thing this project's publishing
> lanes are built to avoid (the crates.io lane uses OIDC Trusted Publishing and
> holds no token at all). The two keyless commands above are what replace it:
> nothing to leak, and the same trust root as the images.

### Your values file is checked before anything is applied

The chart ships a `values.schema.json`, so `helm install`, `helm upgrade`, `helm
lint` and `helm template` **refuse** a values file that misspells one of the
chart's own keys, gets a type wrong, or names a value outside the permitted set,
rather than rendering and silently ignoring it:

```text
Error: values don't meet the specifications of the schema(s) in the following chart(s):
ferroehr:
- at '/image/pullPolicy': value must be one of 'Always', 'IfNotPresent', 'Never'
```

**The `config:` tree is deliberately exempt.** Those keys are the *server's*
(see the [configuration reference](configuration.md)), the binary validates them
at boot, and copying that vocabulary into the chart's schema would fork it: a
new configuration key would then be rejected by the chart until someone
remembered to widen the schema. So a mistake under `config:` is reported when the
pod starts, not when the chart renders; `--skip-schema-validation` disables the
check entirely if you ever need to bypass it.

The chart is also listed on **[Artifact
Hub](https://artifacthub.io/packages/helm/ferroehr/ferroehr)**, which renders the
chart's metadata plus a security report over the two images the chart's own
metadata lists: the server, and the optional admin console.

> [!WARNING]
> Between releases the chart's `config` defaults track development and can be
> **ahead of `appVersion`'s image**. Check the pairing before you install, with
> the image itself as the authority:
>
> ```shell
> helm template ferroehr oci://ghcr.io/rubentalstra/charts/ferroehr --version 6.0.17 \
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
> before it is published: the publish lane refuses to ship a chart whose defaults
> its own `appVersion` image rejects, and it repeats the check against every values
> overlay the chart carries, so this matters mainly when you install from a
> checkout of `develop`, or pin an image older than the chart.

That install alone **boots but answers `401` to everything**, deliberately:
`config.auth.enabled` is on and no mechanism is configured yet, and a server that
authenticates nothing is not a safe default. Add a `config.auth.oidc` issuer or
a `config.auth.basic.users` entry before expecting a request to succeed.

## Configuration: one key, one file

The chart carries the server's whole configuration under one key, **`config`**,
rendered into a `ferroehr.toml` ConfigMap mounted at
`/etc/ferroehr/ferroehr.toml`. Its keys are therefore exactly the TOML keys of
the [configuration reference](configuration.md) (`config.server.bind`,
`config.authz.rbac.enabled`, `config.spec_profile`, and so on) so anything
that reference documents can be set without waiting for a bespoke chart key.

**Secret-bearing keys are the one exception: a credential never reaches that
ConfigMap.** A ConfigMap is not a sensitive object: it is readable with namespace
read, quoted wholesale into issues and support tickets, collected by backup tooling
that skips Secrets, and not covered by Secret encryption at rest. The chart
therefore classifies every key it renders and takes one of two actions.

**A secret the chart routes:** `auth.oidc.hmac_secret`, `signing.key_passphrase`,
`multimedia.secret_access_key`, a Basic user's `password_hash`, a terminology
`client_secret`, and the four URL-shaped ones (`db.url`, `events.url`,
`fhir.outbound.url`, `audit.fhir_feed.url`) has a `secrets:` key of its own, so
a value under `config:` is a mistake and fails the render, naming the key that
belongs there:

```text
Error: execution error at (ferroehr/templates/deployment.yaml:…):
  refusing to render a secret into the ConfigMap (a ConfigMap is not a sensitive object …):
  - config.auth.oidc.hmac_secret: set `secrets.authOidcHmacSecret` instead
```

**A secret the chart cannot route** moves the **whole rendered `ferroehr.toml` into
the chart's Secret, and no ConfigMap is created at all**, the safe direction,
taken automatically. **No key reaches that branch today**: every credential the
server models now has either a `*_file` sibling or a Secret-borne environment
route. The branch exists for the next one that does not, so that a secret key
added upstream fails safe instead of landing in a ConfigMap. When it is taken, the
install notes say which object your release used, and the configuration is read
with:

```shell
kubectl -n ferroehr get secret ferroehr-config \
  -o jsonpath='{.data.ferroehr\.toml}' | base64 -d
```

Classification is by **name shape**, not by a list of today's keys: any key whose
name carries `password`, `passphrase`, `secret`, `credential`, `private_key`,
`api_key` or a trailing `token` is treated as a credential unless it ends in
`_file`, `_path` or `_dir` (those hold a path), and the four URL-shaped secrets are
matched by path, because `url` carries no shape a classifier can see. That is what
makes a secret key added to the server's configuration tree in a future release
move to the Secret rather than leak silently. `extraEnv` is the escape hatch for
anything neither `config:` nor `secrets:` surfaces.

## Database roles — who runs migrations

The chart expects a **four-role** PostgreSQL model, so the runtime pod is never
a superuser:

| Role | Purpose |
|---|---|
| owner | owns the database (provisioning only) |
| `ferroehr_migrator` | runs the append-only schema migrations |
| `ferroehr_app` | day-to-day reads and writes; **the running pod connects as this** |
| `ferroehr_reader` | read-only, for replicas and reporting |

Migrations are DDL, so whoever applies them can rewrite the schema. Two flows,
and `config.db.migrate` is where you choose:

- **(a) The pod migrates itself** (`config.db.migrate: apply`, the default).
  Simplest for single-tenant or small deployments, and the runtime DSN must
  then be a member of `ferroehr_migrator`, so the serving process holds DDL
  rights on the clinical schema for its whole life.
- **(b) A separate migration step** (`config.db.migrate: verify`) under a
  migrator DSN, with the pods on a DSN that is `ferroehr_app` **only**. The
  server issues no DDL at all and refuses to boot against a database that has
  not been migrated to its build, so the two versions cannot race the schema.
  Recommended for production.

Set `migrations.job.enabled` and the chart runs (b) for you as a
`pre-install,pre-upgrade` hook `Job` that runs `ferroehr db migrate`. Helm creates
it before the Deployment and waits for it, so a failed migration fails the release
rather than rolling pods against a schema that was never applied. The Job
authenticates from its **own** Secret (deliberately a different credential from
`database.existingSecret`) and rendering is refused if you enable it without one:

```yaml
database:
  existingSecret: ferroehr-db            # postgres://ferroehr_app:…
migrations:
  job:
    enabled: true
    existingSecret: ferroehr-db-migrator # postgres://ferroehr_migrator:…
config:
  db:
    migrate: verify
```

Give the migrator DSN a short `lock_timeout`
(`?options=-c%20lock_timeout%3D5s`) so DDL blocked behind live traffic fails
fast instead of queueing; `migrations.job.activeDeadlineSeconds` is the hard
ceiling on the step either way. `migrations.runByMigratorRole` remains an
informational marker surfaced in the install notes, which also tell you when
the Job is enabled but `config.db.migrate` is still `apply`, a combination
that buys nothing, because the server would migrate itself anyway.

You can check the posture from outside the cluster at any time: `ferroehr db
verify` exits 0 only when the database carries exactly that build's migrations,
and issues no DDL doing it.

## Secrets and mounted config

Some material is file-shaped rather than a value: ABAC policy files, ATNA TLS
certificates, terminology-server client certificates, a JWKS blob, and the PGP
signing key. Supply these under `config.files`, whose entries the chart mounts
read-only from a Secret at `/etc/ferroehr/<key>` (and which is deliberately
*not* part of the rendered TOML); point the matching in-TOML `*_file` /
`*_path` key at the mounted path. Secret-bearing scalar values go under
`secrets:`: `authOidcHmacSecret`, `signingKeyPassphrase`, `eventsUrl`,
`fhirOutboundUrl`, `auditFhirFeedUrl`, `basicUserPasswordHashes`,
`multimediaAccessKeyId`, `multimediaSecretAccessKey`,
`terminologyOauth2ClientSecrets`, and the database DSN comes from
`database.existingSecret` (key `database.existingSecretKey`, default
`FERROEHR__DB__URL`). None of these ever reach the ConfigMap, and all but two are
delivered as mounted files rather than environment values.

How a secret reaches the process differs by whether the configuration key has a
`*_file` sibling, and the difference is a security one: an environment variable
is readable through `/proc/<pid>/environ` and is inherited by every child
process, so the [OWASP Kubernetes Security Cheat
Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Kubernetes_Security_Cheat_Sheet.html)
asks for a read-only volume instead.

| Secret | How the chart delivers it |
|---|---|
| the database DSN | mounted at `/etc/ferroehr-secrets/db.url`, reached through `FERROEHR__DB__URL_FILE`, projected from `database.existingSecret` when you supply one, so **the credential that reaches patient data never enters the pod's environment** |
| `secrets.authOidcHmacSecret` | mounted at `/etc/ferroehr-secrets/auth.oidc.hmac_secret`; only the path is env |
| `secrets.signingKeyPassphrase` | mounted at `/etc/ferroehr-secrets/signing.key_passphrase` |
| `secrets.multimediaSecretAccessKey` | mounted at `/etc/ferroehr-secrets/multimedia.secret_access_key` |
| `secrets.basicUserPasswordHashes` (per username) | mounted at `/etc/ferroehr-secrets/auth.basic.users.<username>.password_hash`; the chart injects the matching `password_hash_file` |
| `secrets.terminologyOauth2ClientSecrets` (per client) | mounted at `/etc/ferroehr-secrets/terminology.external.oauth2_clients.<name>.client_secret`; the chart injects the matching `client_secret_file` |
| `secrets.eventsUrl`, `secrets.fhirOutboundUrl` | mounted at `/etc/ferroehr-secrets/events.url` and `…/fhir.outbound.url` |
| `secrets.auditFhirFeedUrl` | env; `audit.fhir_feed.url` is the only credential-bearing key with no `*_file` sibling |
| `secrets.multimediaAccessKeyId` | env; an access key *id* is not secret (it is reported unredacted by the management surface's `env` endpoint) |

The mount is read-only, `0440`, owned `root:65532` so the non-root process reads
it through the group bit, and it is deliberately **not** a `subPath` mount,
because a `subPath`-mounted Secret never receives updates and a rotation would
not propagate.

> [!NOTE]
> A Basic user's Argon2id hash is delivered as a mounted file like the others: put
> it in `secrets.basicUserPasswordHashes` under the username, and declare only the
> `username` and `roles` under `config.auth.basic.users`. Setting `password_hash`
> under `config:` is refused, and the error names the key that carries it. An
> Argon2id hash is not a plaintext password, but it is an offline cracking target,
> which is what the boot-time OWASP parameter floor exists to make expensive.

## Security posture

The chart pins the following, and its render gate holds it: the Restricted fields
are asserted per container for every workload in the render, the two isolation
settings are asserted to agree across a release's workloads, and the golden
renders pin the exact bytes so a changed default fails a diff.

| Field | Value |
|---|---|
| `runAsNonRoot` | `true` (uid/gid 65532, the distroless `nonroot` user) |
| `readOnlyRootFilesystem` | `true` (a writable `emptyDir` is mounted at `/tmp`) |
| `allowPrivilegeEscalation` | `false` |
| `capabilities.drop` | `[ALL]` |
| `seccompProfile.type` | `RuntimeDefault` (pod and container) |
| `hostUsers` | `false`; the pod gets its own user namespace |
| `supplementalGroupsPolicy` | `Strict`; only the groups the manifest names |
| ServiceAccount token | not mounted (the workload never calls the K8s API) |
| `enableServiceLinks` | `false` (see below; not a preference) |
| NetworkPolicy | ports narrowed to the API (and management) port; **sources admitted unless you narrow them** with `networkPolicy.ingressFrom`; set `networkPolicy.ingressAllowAll: false` to have the chart refuse the open state (see [§Ingress](hardening-network-policy.md#ingress-ports-are-narrowed-sources-are-yours)) |

The whole set satisfies the Pod Security **Restricted** profile, and the
[deployment probe harness](./kubernetes-hardening.md) reads it back off a running
pod rather than off the rendered manifest: the container runtime's own spec for
the security context, the API server for admission, the EndpointSlice for
readiness.

Satisfying the profile and **enforcing** it are different things, and only one of
them is yours to do. Enforcement comes from a label on the namespace, which a
chart cannot set for a namespace it does not own, so label it, or the posture
above is a convention nothing checks and nothing fails when a future change
regresses it:

```shell
kubectl label --overwrite namespace ferroehr \
  pod-security.kubernetes.io/enforce=restricted \
  pod-security.kubernetes.io/enforce-version=latest
```

With the label in place the API server refuses a non-compliant pod outright
([Pod Security
Admission](https://kubernetes.io/docs/concepts/security/pod-security-admission/)),
which is a stronger guarantee than any check the chart can make about itself.
The install notes print this as a prerequisite for the same reason.

`enableServiceLinks: false` is load-bearing, not hygiene. The kubelet injects a
[set of Service link environment
variables](https://kubernetes.io/docs/concepts/services-networking/service/#environment-variables)
for every Service in the namespace, and for a Service named `ferroehr*` those
land inside the server's reserved `FERROEHR_` namespace, whose strict boot-time
sweep rejects unknown variables and refuses to start. Leaving service links on
makes every install crash-loop.

Egress restriction is opt-in (`networkPolicy.egress.enabled`) because egress
targets (the database, broker, terminology server) are deployment-specific;
when you enable it the chart always admits DNS and you add rules for the rest.
In the default posture the server's only outbound traffic is DNS and PostgreSQL,
so those two rules suffice; each integration you switch on adds a target, and a
blocked one can fail silently. The full destination table is in
[Namespaces, network & policy](./hardening-network-policy.md#egress-deny-by-default-and-what-it-breaks).

Two limits worth stating plainly. First, with `networkPolicy.ingressFrom` empty
the rendered ingress rule carries no `from` selector, and a rule without `from`
admits **every** source, other namespaces included. Only the port list is
narrowed in that state, so set `ingressFrom` to your ingress controller for a PHI
workload, and set `networkPolicy.ingressAllowAll: false` if an open ingress rule
must never render at all; the full treatment is
[§Ingress](hardening-network-policy.md#ingress-ports-are-narrowed-sources-are-yours).
The console's own policy carries the same pair under `adminUi.networkPolicy`.
Second, a NetworkPolicy is only as real as the CNI that implements it:
on a cluster whose network plugin does not enforce NetworkPolicy the object is
documentation rather than a control, and nothing in Kubernetes reports that.
Confirm it by attempting a connection the policy should refuse.

The database-side controls (TLS with `sslmode=verify-full`, pgaudit, at-rest
encryption, WAL archiving / PITR) belong to whoever provisions PostgreSQL; the
chart references them but cannot enforce them. See
[Operations](../operations.md).

## Health probes

All three probes use the always-on, unauthenticated, PHI-free health routes on the
main HTTP port. They need no configuration at all: no management surface, no
access level, nothing to forget:

| Probe | Route | Contract |
|---|---|---|
| liveness | `/health/liveness` | 200 while the process is up; touches no dependency |
| readiness | `/health/readiness` | 200 (UP/DEGRADED) or 503 (DOWN): checks the database ping, migrations applied, the audit sender and the event outbox, each bounded at one second |
| startup | `/health/liveness` | the same constant, with a long failure threshold, so a slow first boot is not killed mid-migration |

That split is deliberate: a database outage must fail *readiness* (the pod stops
receiving traffic) and never liveness (which would restart the container in a
loop).

> [!NOTE]
> There is no `exec` probe option. One existed and was removed: it ran the
> binary's `healthcheck` subcommand, which defaults to the openEHR status
> document rather than a health route, so **readiness never touched the
> database** and a pod with a dead database reported Ready and took clinical
> traffic. `ferroehr healthcheck --url …` is still useful by hand; it is not what
> a readiness probe should run.

> [!WARNING]
> **Migrations run only at boot, so a replaced or wiped database leaves the pods
> NotReady until something migrates it.** Readiness reports
> `"migrations": {"status":"DOWN","detail":"core schema tables missing (migrations
> not applied)"}` while `"db"` reads `UP`: the pod reaches PostgreSQL, finds no
> schema, and does not migrate again, because migration is a startup step.
> Liveness keeps passing (correctly: the process is healthy), so the kubelet never
> restarts the container and the Deployment sits with no ready replicas and no
> error in the logs after the first one:
>
> ```shell
> kubectl -n ferroehr get pods                      # Running, not READY
> curl -s http://ferroehr:8080/health/readiness     # 503, "migrations" DOWN, "db" UP
> kubectl -n ferroehr rollout restart deploy/ferroehr
> ```
>
> The check re-tests the schema on every probe, so a pod recovers on its own within
> one `probes.readiness.periodSeconds` of the schema existing; an untouched pod
> returns to ready, with no restart, once anything else has migrated. What needs
> the restart is the case where the only thing that would migrate is the pod
> itself.
>
> That is what makes the interaction with flow (b) load-bearing: when migrations
> run out of band, the migration step must complete *before* the Deployment rolls,
> or the first pods sit unready waiting for a schema. `migrations.job.enabled`
> does exactly that, as a hook Helm waits on. Durable storage is the other
> half: an `emptyDir`-backed or otherwise disposable PostgreSQL puts a clinical
> repository one node eviction away from this state.
>
> One recovery path does **not** self-heal, and it is worth knowing before you try
> it: restoring or dropping *part* of the schema set. The archival tier's tables
> live in their own `cold` schema, so a `DROP SCHEMA ehr CASCADE` leaves them
> behind, and re-running the migration set then fails permanently with
> `relation "vo_version" already exists`: the pod crash-loops, and restarting it
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
regardless of all of this, and the install notes warn you if you add the scrape
annotations while the endpoint is still `off`.

Those annotations are honoured by a Prometheus that discovers targets in its own
scrape configuration. An **operator-managed** Prometheus
(kube-prometheus-stack) ignores them and selects targets through
[ServiceMonitor](https://prometheus-operator.dev/docs/developer/getting-started/)
objects instead, so set `metrics.serviceMonitor.enabled=true` there;
`metrics.serviceMonitor.labels` is where the label your Prometheus'
`serviceMonitorSelector` matches on goes. It needs the `monitoring.coreos.com`
CRDs installed first, or the install fails on an unknown kind.

## Authentication is required, and the chart says so before it installs

`config.auth.enabled` defaults to **true**, and the server requires at least one
mechanism with it: a `401` challenge has to name a scheme the server actually
implements ([RFC 9110 §11.6.1](https://www.rfc-editor.org/rfc/rfc9110#section-11.6.1)),
so it exits rather than run as an openEHR API that can only refuse every request.

The chart therefore **refuses to render** a values file that enables
authentication without configuring one, so `helm install` stops with an
actionable error instead of reporting success and crash-looping:

```text
Error: execution error at (ferroehr/templates/deployment.yaml:...):
config.auth.enabled is true but no authentication mechanism is configured...
```

Pick one:

```yaml
config:
  auth:
    oidc:
      issuer: https://keycloak.example.org/realms/ferroehr
      audiences: [ferroehr]
```

or Basic auth, whose password hashes go in `secrets.basicUserPasswordHashes`
while the username and roles stay under `config.auth.basic.users`.

> [!WARNING]
> `config.auth.enabled: false` makes the chart render, and serves **every**
> request unauthenticated. On a repository holding patient data that is a
> development-only choice; the chart makes you state it explicitly rather than
> reaching it by omission.

## Optional integrations

### Any server setting is reachable

The chart renders its `config` tree **verbatim** into `ferroehr.toml`. So
`config.<the.toml.path>` sets *any* key in the
[configuration reference](configuration.md), including keys this page and
`values.yaml` never mention. There is no allow-list to extend and no chart
release to wait for:

```yaml
# values.yaml — [query] plan_cache_capacity, which values.yaml never names
config:
  query:
    plan_cache_capacity: 512
```

```shell
helm upgrade ferroehr oci://ghcr.io/rubentalstra/charts/ferroehr \
  --version 6.0.17 -n ferroehr --reuse-values \
  --set config.query.plan_cache_capacity=512
```

Turning something off is the same edit in reverse: remove the key (or set the
integration's `enabled` back to `false`) and upgrade. The tables below are a
curated starting point for the switches most deployments want; they are not
the boundary of what the chart supports.

Two things make this safe to rely on:

- **A typo is a boot refusal, not a silent default.** The server sweeps its
  configuration strictly and rejects an unknown key with a did-you-mean, so a
  misspelled path fails loudly at startup instead of quietly doing nothing.
  Check before you deploy with the schema and boot gates described under
  [Check your values before you deploy them](#check-your-values-before-you-deploy-them).
- **Credentials do not belong in this tree.** `config` becomes a mounted
  file; secrets have their own routes (`secrets.*`, `existingSecret`, and the
  `*_file` key siblings); see
  [Secrets and mounted config](#secrets-and-mounted-config).

### The common switches

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
| Multi-tenancy | `config.tenancy.enabled` | off | Tenant from a JWT claim (`config.tenancy.claim`); never set `config.tenancy.header` in production. Pairs with PG row-level security. |
| OAuth2/OIDC auth | `config.auth.oidc.issuer` | unset | Prefer JWKS/discovery over the HS256 `secrets.authOidcHmacSecret`. |
| RBAC | `config.authz.rbac.enabled` | **on** | The coarse role gate (active while `config.auth.enabled`). |
| ABAC | `config.authz.abac.enabled` | off | Cedar (policies via a `config.files` mount) or a remote policy decision point. |
| Eventing → AMQP | `config.events.enabled` | off | Envelopes are **PHI-free** by design. Use `config.events.tls: true`; URL via `secrets.eventsUrl`. |
| FHIR inbound/façade | `config.fhir.api_enabled` | off | Read façade + inbound mapping. |
| FHIR outbound → AMQP | `config.fhir.outbound.enabled` | off | ⚠ **Carries PHI** (the mapped FHIR resource). Separate exchange; TLS broker only; URL via `secrets.fhirOutboundUrl`. |
| S3 multimedia | `config.multimedia.enabled` | off | ⚠ Offloaded blobs are PHI. Private, encrypted, HTTPS bucket; keys via `secrets.multimediaAccessKeyId` and `secrets.multimediaSecretAccessKey`. |
| External terminology | `config.terminology.external.enabled` | off | FHIR terminology server; the provider map is more `config.terminology.external.providers` keys. |
| ATNA audit trail | `config.audit.enabled` | **on** | On with the local store only; forwarding (`config.audit.syslog`, `config.audit.fhir_feed`) is opt-in per sink. |
| Version signing | `config.signing.enabled` | **on** (`config.signing.mode: digest`) | `pgp` mode needs a `config.files` key plus `secrets.signingKeyPassphrase`, and fails closed at boot without a usable key. |
| OTLP telemetry | `config.telemetry.otlp_endpoint` | unset | Setting the endpoint is all it takes; unset means the OpenTelemetry layer is not installed at all (zero overhead). With `networkPolicy.egress.enabled`, add a rule for the collector, since a blocked exporter drops spans without an error. |

Full detail on each is in [Beyond the core](../beyond-core/index.md),
[Security & multi-tenancy](../security.md), and [Operations](../operations.md).

### The admin console (a second workload, off by default)

`adminUi.enabled` renders a second Deployment and Service for the Leptos admin
console beside the CDR, from its own image
(`adminUi.image.repository`, tagged `appVersion` by default so the two move
together). It is off by default: the console is a separate product surface with
its own attack surface, and a CDR is complete without it.

Three properties are worth knowing before you switch it on:

- **It reaches the CDR strictly over the REST API**, which
  `adminUi.networkPolicy.enabled` (on by default) *enforces* rather than
  assumes: the console's egress admits the CDR Service, DNS and outbound HTTPS
  for an identity provider, and nothing else. It holds no database credential.
- **It is a human-facing web UI**, so `adminUi.ingress.enabled` is the normal way
  to reach it, and `adminUi.auth.oidc.enabled` with an issuer, client id and
  `adminUi.auth.oidc.publicBaseUrl` is how you keep a person who should not see
  PHI out of it. Its client secret comes from `adminUi.existingSecret`, mounted
  as a file exactly as the server's DSN is.
- **`adminUi.replicaCount` is 1 deliberately.** The console holds session state
  in process, so a second replica needs sticky sessions at the ingress or users
  are logged out on a reroute.

The console carries the same security context as the server, and the chart's
render gate holds it to the same Restricted profile and the same pod-isolation
settings: a release whose two workloads disagreed about `hostUsers` would be a
posture nobody could state in one sentence. The deployment probe harness then
reads that back off the *running* console container, exactly as it does for the
server: its uid, its empty capability bounding set, its read-only root and its
seccomp filter come from the container runtime's own spec, its admission from an
`enforce=restricted` namespace, and its login page from an HTTP request made
inside the cluster through the console Service, so the second workload is never
vouched for by the first. What that run does **not** establish is stated in its
own record: the console's OIDC path, the screens behind a session, and whether a
CNI enforces the console's NetworkPolicy. Its screens are documented in the
[admin console](../admin-ui/index.md) chapter.

## Staying available while things move

Four defaults keep the API serving through the events that routinely interrupt
it. None needs configuring; each is listed because the reason matters when you
tune it.

**Replicas land on different nodes.** With nothing telling the scheduler
otherwise, two replicas can share one node and one node failure is a total
outage. The chart ships a soft spread constraint (`maxSkew: 1` over
`kubernetes.io/hostname`, `whenUnsatisfiable: ScheduleAnyway`) so replicas
prefer separate nodes but a single-node or full cluster still schedules them
rather than leaving a pod `Pending`. Setting `topologySpreadConstraints`
**replaces** it wholesale, so give the complete constraint including its own
`labelSelector`; add a `topology.kubernetes.io/zone` entry if your cluster spans
zones.

**A terminating pod stops receiving requests before it shuts down.** Deleting a
pod removes it from the EndpointSlice and sends `SIGTERM` *concurrently*, and
the removal still has to propagate to every node. `preStopSleepSeconds` (default
5) holds the container for that window first. It uses the native `sleep` hook
action rather than an `exec` hook, because the image ships no shell to run one.

**A node drain does not hang on unhealthy pods.** The PodDisruptionBudget sets
`podDisruptionBudget.unhealthyPodEvictionPolicy: AlwaysAllow`, the
[documented recommendation](https://kubernetes.io/docs/concepts/workloads/pods/disruptions/).
The API default, `IfHealthyBudget`, makes a drain wait for pods to become
healthy, which never completes when they are unhealthy *because* of the drain.

**A migration interrupted by a drain is not counted as a failure.** The
migration Job carries a `podFailurePolicy` that ignores pod failures caused by
disruption, so ordinary cluster maintenance during a release cannot exhaust
`migrations.job.backoffLimit` and fail the upgrade with no migration error
anywhere in the logs.

Two more are available and off by default. `service.trafficDistribution:
PreferSameZone` keeps traffic inside the caller's zone, for lower latency and
inter-zone cost, at the price of even load, so measure before setting it. And
`autoscaling.behavior` passes scaling policies straight through; the documented
defaults already scale up immediately and wait out a five-minute stabilization
window before scaling down, so change it only to be *more* conservative.

## Upgrades

Migrations are **append-only** (a schema change is a new file, never an edit to
an applied one) so a rolling upgrade stays compatible with the previous schema
during the window where both versions run: additive DDL first, destructive
changes in a later release once all pods are on the new version. Keep
`replicaCount` at two or more (or `autoscaling.enabled`) and the default
PodDisruptionBudget so upgrades and node drains never fully interrupt the API;
the default `terminationGracePeriodSeconds` covers the binary's shutdown drain.
Roll back by re-pinning the prior image tag or digest.

A change anywhere under `config`, `config.files` or `secrets` changes the
`checksum/config` pod annotation, so `helm upgrade` rolls the pods for a
configuration-only change too, including a rotated secret or an edited ABAC
policy, both of which are read at boot and would otherwise reach the volume while
every running pod kept using the old value.

`helm uninstall` removes everything the chart created; the chart declares no
PersistentVolumeClaim, so nothing is left behind; your database, and the Secret
holding its DSN, are yours and survive.

Preview an upgrade against what you have installed with
`helm diff`, or render the new chart version and read it:

```shell
helm template ferroehr oci://ghcr.io/rubentalstra/charts/ferroehr --version 6.0.17 \
  -n ferroehr -f my-values.yaml | less
```

Working from a checkout instead, `deploy/helm/validate.sh` runs the chart's full
render gate: the helm-version pin, lint, YAML validity, the structural
Restricted-profile and selector-immutability gates, the secret-leak gate, the
values-schema probes, and the golden-render diff. It is the same gate CI runs on
every change to the chart, so a local run and a pull request agree by
construction.

### Check your values before you deploy them

A render that lints is not a deployment that boots. `validate.sh` never runs the
server, so it cannot see a configuration the server refuses: a missing
authentication mechanism, an HMAC secret under the 32-byte floor, a password
hash that is not a real Argon2id PHC string, SMART enabled without its public
base URL. Every one of those renders perfectly and crash-loops the pod. The
script prints exactly which properties it does not check, on success as well as
on failure, so a green run is not mistaken for a working deployment.

The check that closes that gap runs the image against your rendered
configuration:

```shell
FERROEHR_IMAGE=ghcr.io/rubentalstra/ferroehr:4.0.3 \
  deploy/helm/ci/boot-check.sh my-values.yaml
```

It renders the chart, mounts the ConfigMap and the Secret exactly as the
Deployment does (with their real values, not placeholders) replays the
declared environment, and runs `ferroehr config check` inside the image. Point
`FERROEHR_IMAGE` at the tag you intend to deploy: the answer is specific to that
image, since a key your values carry may be newer than the server that has to
read it.

It validates configuration only; it opens no socket, so it cannot tell you the
issuer resolves, the broker is reachable or the database accepts the DSN. CI
runs the same script over every values overlay the chart ships.
