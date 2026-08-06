# Cluster hardening: what the chart does, and what you must

The [Kubernetes chapter](kubernetes.md) documents the chart. This page is the
other half: an audit of this deployment against the [OWASP Kubernetes Security
Cheat
Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Kubernetes_Security_Cheat_Sheet.html),
split by **who can actually apply each control**.

That split is the point of this page. A workload chart controls its pod security
context, its resource bounds, its NetworkPolicy, its ServiceAccount and how it
consumes secrets. It cannot patch a node, set an API-server flag, configure etcd,
authenticate a kubelet, or install an admission controller. For those, the honest
deliverable is not a setting but a statement of **what you owe and what happens if
you do not** — because a deployment that inherits an unchecked assumption is not
secured by the chart's own hardening.

> [!IMPORTANT]
> Several controls here are ones **no application-level hardening can compensate
> for**. If the kubelet accepts anonymous requests, or anything untrusted can read
> etcd, then every control this project ships — non-root, read-only rootfs, dropped
> capabilities, RBAC, ABAC, the audit trail — is bypassable, because the attacker
> is beneath the layer they operate in. Those are marked below.

Claims on this page about the running deployment come from `kubectl`, `crictl` or
`/proc` on a live cluster, not from reading `values.yaml`.

## The ownership map

| Cheat-sheet control | Owner | Where |
|---|---|---|
| Host hardening, OS patching, node firewall | operator | [below](#host-hardening-and-the-version-window) |
| Supported Kubernetes version window | operator (chart states the floor) | [below](#host-hardening-and-the-version-window) |
| Rolling upgrades rather than mutating containers | **chart** | `strategy` in values; [below](#upgrades-roll-they-do-not-replace) |
| Kubernetes security advisories | operator | [below](#security-advisories) |
| Kubernetes Dashboard | operator (we ship none) | [below](#the-dashboard-we-do-not-ship) |
| etcd access + encryption at rest | operator | [below](#etcd-and-what-our-secrets-contain) |
| Control-plane and kubelet ports | operator | [below](#ports-theirs-and-ours) |
| The workload's own port surface | **chart** | [below](#ports-theirs-and-ours) |
| Cluster API access control, MFA | operator | [below](#cluster-api-access) |
| Cluster RBAC (`Node,RBAC`, `NodeRestriction`) | operator | [below](#cluster-rbac-and-why-this-chart-needs-none) |
| The workload's own RBAC | **chart** — deliberately none | [below](#cluster-rbac-and-why-this-chart-needs-none) |
| Kubelet authentication/authorization | operator | [below](#kubelet-access) |
| Minimal, current, authorized images | **chart/CI** | [below](#the-build-phase-and-what-distroless-costs) |
| Namespace isolation | **chart** (namespace-scoped by construction) | [below](#namespaces-and-the-two-tenant-models) |
| Image provenance at admission | operator (we publish the attestations) | [below](#image-provenance-at-admission) |
| Continuous scanning of published images | **CI** | [below](#continuous-scanning-of-published-images) |
| Pod/container security context | **chart** | [below](#the-security-context-and-what-keeps-it-true) |
| Pod Security Admission enforcement | operator (one `kubectl label`) | [below](#pod-security-admission-complying-versus-being-refused) |
| Service mesh | neither — recorded decision | [below](#service-mesh-a-recorded-decision) |
| Centralized policy management | operator, for admission only | [below](#centralized-policy-and-which-engine) |

## Host hardening and the version window

**Operator's.** Keep the node OS patched, hardened and firewalled; a workload chart
cannot reach any of it. The cheat sheet's list applies unchanged.

The part worth stating precisely is the **version window**. Upstream Kubernetes
maintains release branches for the **three most recent minor releases**, each
receiving roughly a year of patch support
([kubernetes.io/releases](https://kubernetes.io/releases/)). As of this writing
that is **1.34, 1.35 and 1.36**. A cluster below that window receives **no
security backports at all** — a published CVE in the API server or kubelet simply
stays open on it.

The chart's `kubeVersion: ">=1.25.0-0"` is **not** a statement that 1.25 is a
supported platform. It is a *compatibility* floor: the newest API the chart uses
is HPA `autoscaling/v2` (GA in 1.23), so it genuinely renders and runs on 1.25,
and refusing to install there would be a false claim in the other direction. The
two facts are separate, and this is the one place they are stated together:

- **Will the chart work?** Anything from 1.25.
- **Will your cluster receive security fixes?** Only inside the three-release
  window, which is a moving target you must track.

If you run outside that window, you have accepted that the platform beneath this
CDR is unpatched, and no setting in `values.yaml` changes that.

## Upgrades roll, they do not replace

**Chart's.** The cheat sheet asks that a new version arrive by rolling update
rather than by mutating a running container. The chart sets the strategy
explicitly rather than inheriting the API server's percentages:

```yaml
strategy:
  type: RollingUpdate
  rollingUpdate:
    maxSurge: 1
    maxUnavailable: 0        # served capacity never drops
```

`maxUnavailable: 0` means a replacement pod must pass its readiness probe before
an old one is removed. At the default two replicas this is identical to what
Kubernetes computed from `25%` (it rounds down to zero); above two replicas the
percentage default would have taken a pod out of service, and this does not. The
trade-off is real and fails in the safe direction: on a cluster with no spare
scheduling capacity a `maxUnavailable: 0` rollout **stalls visibly** instead of
proceeding at reduced capacity.

Verified on a live cluster rather than argued: a probe pod issued **400
sequential requests** to the Service through a full `helm upgrade` — with
`Killing`/`Started` events for both replicas inside the window — and recorded
**zero failures**, while the ReplicaSet history showed revision 1 scaled to zero
as revision 2 scaled up (a roll, not a recreate) and the Deployment reported
`Available=True (MinimumReplicasAvailable)` throughout.

## Security advisories

**Operator's, with a decision recorded on our side.**

You must follow
[`kubernetes-announce`](https://groups.google.com/g/kubernetes-announce) and the
[official advisory
feed](https://kubernetes.io/docs/reference/issues-security/official-cve-feed/).
Kubernetes CVEs are announced there, and nothing in this repository will tell you
about them.

**This project does not track Kubernetes platform advisories, deliberately.** The
decision, so it is not mistaken for an oversight: we run no cluster and cannot act
on a node or control-plane CVE, and a watcher that opened issues we could only
close as "the operator's" would be noise that trains people to ignore it. What we
*do* track is what we ship — dependency advisories (`cargo deny`, on every
change), our own container images (scheduled scans), and the openEHR
specifications (release watchers).

A vulnerability in Kubernetes itself is reported to
[Kubernetes](https://kubernetes.io/docs/reference/issues-security/security/), not
to this project. A vulnerability in FerroEHR — including in this chart — comes to
us, through `SECURITY.md`.

## The dashboard we do not ship

**Operator's.** This chart installs no Kubernetes Dashboard, and nothing in it
depends on one. If you install one, the cheat sheet's conditions apply: never
expose it publicly, give it a limited-privilege ServiceAccount, and put an
authenticating reverse proxy in front of it if it must be reachable at all.

The reason this section is not simply "not applicable" is that **the same
reasoning applies to two surfaces that _are_ ours**, and an operator hardening
"the dashboard" should find them here:

- **`/management/*`** — the ops-introspection surface (`info`, `prometheus`,
  `metrics`, `env`, `loggers`, `flamegraph`). It is a privileged read onto the
  deployment: `env` renders the effective configuration and `flamegraph` profiles
  the live process. The chart ships the master switch on and **every endpoint
  `off`**, so nothing is exposed until you opt in, and `access_default` is
  `admin_only`. Set `config.management.port` to move the whole surface onto its own
  listener so it is never reachable on the clinical API port.
- **The admin console** — a separate image and deployment, not installed by this
  chart. It consumes the CDR strictly over ITS-REST and holds no database
  credential, but it is a privileged UI and belongs behind the same
  authenticating edge you would put in front of a dashboard.

## etcd and what our secrets contain

**Operator's**, and this is one of the controls that cannot be compensated for.

The cheat sheet's requirements stand: mutual TLS between the API server and etcd,
etcd reachable from nothing else, and separate instances or ACLs to bound what a
component can read.

What makes it concrete for this deployment: **anything that can read etcd can read
every Secret in the cluster**, and this release's Secrets are not incidental. They
hold the **database DSN** — the credential that reaches patient data — plus the
OIDC HMAC secret, the version-signing passphrase, any terminology-server client
secret, and (when a Basic user is configured) the whole rendered `ferroehr.toml`.
So "etcd is a cluster concern" is true and insufficient: for this workload etcd is
the confidentiality boundary of the credentials that reach PHI.

Two mitigations you can apply without touching etcd's network posture:

1. **Encryption at rest for Secrets** — not on by default in Kubernetes. See
   Kubernetes'
   [encryption-at-rest
   configuration](https://kubernetes.io/docs/tasks/administer-cluster/encrypt-data/),
   which is a cluster-level setting an operator applies once.
2. **An external secret manager**, which removes the credential from etcd
   entirely: every secret this chart carries has a `*_file` route or an
   `existingSecret` route, so the value can arrive from a CSI driver or an
   operator-synced Secret rather than from chart values.

## Ports, theirs and ours

**Operator's, for the cluster's ports.** Block untrusted access to the
control-plane ports — `6443` (API server), `2379-2380` (etcd), `10250-10257`
(kubelet and controller/scheduler) — and the worker ports `10248-10250`. An
exposed `10250` is the kubelet case below.

**Ours, for the workload's ports**, and the surface is one port:

| Port | Serves | Who should reach it |
|---|---|---|
| `8080` (`service.port`) | the openEHR REST API, the always-on `/health` family, and `/management/*` when `config.management.port` is unset | your ingress controller or gateway — not the internet directly |
| `config.management.port` (unset by default) | `/management/*` on its own listener when set | operators and your Prometheus, never clinical clients |

Measured on the running pod rather than read from the template — the listening
sockets in the container's network namespace, via `/proc/<pid>/net/tcp` on the
node:

```text
LISTENING TCP ports in the container netns: [8080]
```

One port. Nothing else is bound, in the default posture.

The shipped NetworkPolicy narrows inbound traffic to that port list, and the
narrowing is **enforced**, not decorative. From a pod in a different namespace:

```text
nc -z <pod-ip> 8080   → exit 0     (admitted)
nc -z <pod-ip> 9100   → exit 1
nc -z <pod-ip> 5432   → exit 1
```

and with the policy's port list patched to a port the app does *not* serve, while
the process kept listening on 8080:

```text
nc -z <pod-ip> 8080   → exit 1     (refused by policy, not by the process)
```

Two limits stated in full on the [Kubernetes page](kubernetes.md#security-posture)
and repeated because they matter here: with `networkPolicy.ingressFrom` empty the
rule carries no `from` and therefore admits **every source**, including other
namespaces (only the port list is narrowed — the test above reached `8080` from a
different namespace); and a NetworkPolicy is only as real as the CNI that
implements it.

## Cluster API access

**Operator's.** Control access to the Kubernetes API: authenticate, then
authorize, and deny by default.

- **Recommended routes:** OIDC, a managed-IAM integration, or user
  impersonation — with **MFA** on the identities that can reach the cluster API.
- **Not suitable for production:** static token files, long-lived X.509 client
  certificates, and service-account tokens used as human credentials. They cannot
  be revoked individually, they do not expire usefully, and they carry no second
  factor.

Worth drawing explicitly, because it saves a translation: **this server implements
the same shape.** Authentication then authorization, deny by default, OIDC
preferred over long-lived credentials, and a missing credential distinguished from
a refused one (`401` with a challenge versus `403`). The
[Security chapter](../security.md) is the detail. An operator who understands why
a static token file is a poor cluster credential already understands why
`auth.basic` is a development mechanism here and `auth.oidc` is the production
one.

## Cluster RBAC, and why this chart needs none

**Operator's, for the cluster:** run the API server with
`--authorization-mode=Node,RBAC` and enable the `NodeRestriction` admission
plugin, so a compromised kubelet cannot edit objects belonging to other nodes.

**Ours, and it is an absence on purpose.** The chart creates a ServiceAccount and
**no Role, RoleBinding, ClusterRole or ClusterRoleBinding at all**, with
`automountServiceAccountToken: false`. That is not an omission to be tidied up
later: the workload never calls the Kubernetes API, so it needs no permissions,
and it is not given a token with which to try. Verified on the live release:

```text
$ helm get manifest ferroehr | grep -cE '^kind: (Role|RoleBinding|ClusterRole|ClusterRoleBinding)'
0
$ kubectl -n ferroehr get role,rolebinding
No resources found in ferroehr namespace.
$ find /var/lib/kubelet/pods/<uid>/volumes -name '*token*' -o -name '*serviceaccount*'
(nothing)
```

**If you are reviewing this chart and reaching for a Role to add: don't.** The
correct fix for a future feature that genuinely needs the Kubernetes API is a
Role enumerating exactly the verbs and resources it needs, plus turning the token
mount back on for that ServiceAccount alone — not a broad grant added
speculatively.

## Kubelet access

**Operator's**, and the second control no application hardening can compensate
for.

Run every kubelet with `--anonymous-auth=false` and
`--authorization-mode=Webhook` so its HTTPS endpoint is not open. Left open, that
endpoint permits **arbitrary command execution in any container on the node**.

For this deployment, spelled out: an attacker reaching an unauthenticated kubelet
gets a process-level foothold in a running CDR — the ability to read the
database DSN out of the process environment or its mounted secret files, to read
patient data straight from memory, and to do so **beneath** the layer where
authentication, RBAC, ABAC and the ATNA audit trail operate, so none of them see
it and none of them can stop it. Non-root, a read-only root filesystem and an
empty capability set raise the cost of what happens next; they do not prevent the
entry.

This is a cluster-configuration control, and it is worth confirming rather than
assuming — an exposed `10250` is a routine finding in real clusters.

## The build phase, and what distroless costs

**Ours, and this is the section with the most evidence behind it.** Each
build-phase control and what actually satisfies it:

| Control | Satisfied by |
|---|---|
| Minimal image (§3.4, §3.4.1 distroless) | `gcr.io/distroless/cc-debian13:nonroot` — no shell, no package manager, no libc tooling |
| Image currency (§3.1.1) | base images and CI job containers pinned **by digest**, not tag, so a rebuild cannot silently change bytes |
| Vulnerability identification in CI (§3.3) | Trivy on every published image, hadolint on every Dockerfile, plus secret and misconfiguration scanning over the tree |
| Continuous scanning after release (§4.3) | a scheduled scan of the *published* tags — [below](#continuous-scanning-of-published-images) |
| Authorized images only (§3.2) | signed provenance published; **enforcement is the operator's** — [below](#image-provenance-at-admission) |
| Non-root by construction | the image declares `USER nonroot`, and the pod pins `runAsNonRoot` + uid 65532 independently |

**What distroless costs, stated before an incident rather than during one: there
is no shell in the image, so `kubectl exec … -- sh` does not work.** That is the
security property working as intended — an attacker who achieves command
execution finds no interpreter, no `curl`, no package manager — but it changes how
you debug. Use instead:

- `kubectl logs` (the server logs JSON by default, for a collector),
- the always-on `/health/readiness` body, which names the failing dependency,
- `/management/*` for the effective configuration, live log filters and an
  on-demand CPU flamegraph,
- `kubectl debug -it <pod> --image=busybox:1.37 --target=ferroehr` — an ephemeral
  container shares the target's namespaces without adding a shell to the image
  that ships.

The registry posture: the images are **public** on GHCR, so a pull needs no
credential and there is nothing to leak. Public does not mean trusted, which is
the point of the next section — nothing about a public registry stops a cluster
pulling a *different* image with the same name from somewhere else.

## Namespaces and the two tenant models

**Ours, and satisfied by construction.** Every object the chart renders is
namespace-scoped — Deployment, Service, ConfigMap, Secret, ServiceAccount,
NetworkPolicy, PodDisruptionBudget, HorizontalPodAutoscaler, Ingress,
ServiceMonitor. There is **no ClusterRole, no ClusterRoleBinding, no
CustomResourceDefinition, no cluster-scoped object of any kind**, and no template
hard-codes a namespace: every reference is `.Release.Namespace` or a bare name
resolved within the release's own namespace. So two releases in two namespaces
cannot collide, and neither can reach the other's Secrets.

Two ways to isolate tenants, with genuinely different blast radii — choose
deliberately:

| | Namespace per tenant | In-process multi-tenancy (`config.tenancy`) |
|---|---|---|
| Isolation boundary | Kubernetes: separate Secrets, NetworkPolicies, quotas, RBAC | one process, one database; tenant from a JWT claim, enforced by PostgreSQL row-level security |
| Blast radius of an application-level bug | one tenant | potentially all tenants in the release |
| Blast radius of a compromised database credential | one tenant's database | every tenant in that database |
| Cost | one Deployment, one connection pool, one image pull per tenant | one deployment for all |

The stronger boundary is a namespace and a database per tenant; the cheaper one is
the tenancy feature. A deployment holding data for organizations that must not be
able to reach each other under any single failure should prefer the first, and
should not treat the second as equivalent because the wire behaviour looks the
same.

## Image provenance at admission

**The operator's — and this is provenance nobody currently checks.**

The publishing lanes attest both the images and the chart through **keyless
Sigstore**, which means a verifier can establish that an artifact came from this
repository's build. Nothing in a cluster *requires* that check before running one,
and a signature nobody verifies changes nothing about what actually runs.

> [!WARNING]
> **No published artifact carries an attestation yet, so the policies below are
> written from the workflow definitions and have not been verified against a live
> attestation.** The signing step was added after the most recent release, and no
> publishing run has executed since: `gh attestation verify` on the current
> `ferroehr:3.17.3` and `ferroehr:develop` images both return HTTP 404, and the
> repository's attestation list is empty. Before relying on either policy, confirm
> the identity it trusts against a real artifact:
>
> ```shell
> gh attestation verify oci://ghcr.io/rubentalstra/ferroehr:<tag> \
>   -R rubentalstra/FerroEHR --format json | jq '.[0].verificationResult.signature.certificate'
> ```
>
> and reconcile `certificateIdentity` / `certificateOidcIssuer` with the values
> below. A `--certificate-identity` that does not match what the lane issues fails
> **closed** — it blocks a legitimate image — which is worse than no policy.

The identity to trust, derived from the lanes themselves: GitHub's OIDC issuer,
and a subject that is the publishing workflow's own ref.

| Artifact | Built by | Certificate identity (SAN) |
|---|---|---|
| the three images | `.github/workflows/containers.yml` | `https://github.com/rubentalstra/FerroEHR/.github/workflows/containers.yml@refs/tags/vX.Y.Z` |
| the chart | `.github/workflows/publish-chart.yml` | `https://github.com/rubentalstra/FerroEHR/.github/workflows/publish-chart.yml@refs/tags/vX.Y.Z` |

with issuer `https://token.actions.githubusercontent.com` in both cases.

**Kyverno** (`verifyImages`, keyless) — the engine [chosen
below](#centralized-policy-and-which-engine):

```yaml
apiVersion: kyverno.io/v1
kind: ClusterPolicy
metadata:
  name: ferroehr-image-provenance
spec:
  validationFailureAction: Enforce
  rules:
    - name: verify-ferroehr-images
      match:
        any:
          - resources:
              kinds: [Pod]
              namespaces: [ferroehr]
      verifyImages:
        - imageReferences:
            - "ghcr.io/rubentalstra/ferroehr*"
          attestors:
            - entries:
                - keyless:
                    subject: "https://github.com/rubentalstra/FerroEHR/.github/workflows/containers.yml@refs/tags/*"
                    issuer: "https://token.actions.githubusercontent.com"
                    rekor:
                      url: https://rekor.sigstore.dev
```

**sigstore-policy-controller**, if you already run it:

```yaml
apiVersion: policy.sigstore.dev/v1beta1
kind: ClusterImagePolicy
metadata:
  name: ferroehr-image-provenance
spec:
  images:
    - glob: "ghcr.io/rubentalstra/ferroehr**"
  authorities:
    - keyless:
        url: https://fulcio.sigstore.dev
        identities:
          - issuer: https://token.actions.githubusercontent.com
            subjectRegExp: "^https://github\\.com/rubentalstra/FerroEHR/\\.github/workflows/containers\\.yml@refs/tags/v.*$"
```

**Should the chart ship one? No, and the reason is structural:** an admission
policy is cluster-scoped and governs workloads the chart knows nothing about,
while the chart deliberately renders no cluster-scoped object at all (see
[namespaces](#namespaces-and-the-two-tenant-models)). A `ClusterPolicy` in this
chart would mean `helm uninstall` removing a control that other releases had come
to depend on. The chart's contribution is the policy *document*, here, versioned
with the lanes whose identity it encodes.

**Without an admission controller**, the manual equivalent is a release-time
check rather than a per-pod one:

```shell
gh attestation verify oci://ghcr.io/rubentalstra/ferroehr:3.17.3 -R rubentalstra/FerroEHR
gh attestation verify oci://ghcr.io/rubentalstra/charts/ferroehr:4.0.0 -R rubentalstra/FerroEHR
```

Then deploy **by digest** (`image.digest`), so what you verified is what runs — a
tag can be moved afterwards, a digest cannot.

## Continuous scanning of published images

**Ours.** CI scans images at build time, which catches what was known when they
were built and nothing after. A CVE published the week after a release applies to
the image people are running, so the published tags are re-scanned on a schedule
(`.github/workflows/image-scan.yml`, Mondays): all three images at the tag a user
pulls, with the same `trivy.yaml` floor and the same adjudicated exceptions, and
the OpenVEX documents under `security/vex/` applied so an accepted finding stays
accepted with its argument attached.

A finding does two things, because either alone fails quietly: it opens (or
comments on) a tracking issue, **and** it fails the run. A red scheduled run
nobody looks at is not a control, and an issue with no failing check can be closed
without the finding being addressed.

`ferroehr-postgres` is the image this exists for — it is built on the upstream
`postgres` image, so its OS package set is not ours and its CVEs arrive on
someone else's schedule. Measured when the lane was written: **0** fixable
HIGH/CRITICAL findings across all three published images.

## The security context, and what keeps it true

**Ours, and settled.** Read from the *running container* via `crictl`, not from
`values.yaml`:

```text
process.user     : {'additionalGids': [65532], 'gid': 65532, 'uid': 65532}
noNewPrivileges  : True
capabilities.bnd : None            ← an EMPTY bounding set, not "default minus ours"
root.readonly    : True
seccomp default  : SCMP_ACT_ERRNO  (13 rule groups)
RW mounts        : /tmp  (plus the kernel/kubelet-managed /proc, /dev/*, /etc/hosts)
RO mounts        : /etc/ferroehr  /etc/ferroehr-secrets  /sys  /sys/fs/cgroup
```

Three things worth drawing out. The capability bounding set is **empty**, so the
drop is total at the kernel level rather than a subtraction from a runtime's
default set. `readOnlyRootFilesystem: true` needed exactly **one** writable
path — the chart's own `/tmp` emptyDir — and no per-integration surprise, which
is what makes it safe to keep rather than the setting an operator relaxes during
the first incident. And there are no init containers and no sidecars, so the
context above is the whole pod.

What keeps it true is not this page: `deploy/helm/validate.sh` asserts every one
of these fields on every render and the golden files pin the exact bytes, and both
run in CI on any change to the chart. A template edit that drops
`readOnlyRootFilesystem` or `allowPrivilegeEscalation: false` fails a job, not a
review.

## Pod Security Admission: complying versus being refused

Meeting the Restricted profile and being *refused* when you stop meeting it are
different properties, and only the first is ours.

**Restricted compliance, field by field**, so the claim is checkable against the
[standard](https://kubernetes.io/docs/concepts/security/pod-security-standards/)
rather than asserted:

| Restricted requires | The chart sets |
|---|---|
| `hostNetwork`/`hostPID`/`hostIPC` unset | none set |
| no privileged containers | `privileged: false` |
| `allowPrivilegeEscalation: false` | set |
| capabilities dropped to `ALL` (only `NET_BIND_SERVICE` may be added) | `drop: [ALL]`, none added |
| `runAsNonRoot: true` | set, pod and container, uid/gid 65532 |
| `seccompProfile.type` `RuntimeDefault` or `Localhost` | `RuntimeDefault`, pod and container |
| no hostPath volumes | only `emptyDir`, `projected`, `secret` |
| `readOnlyRootFilesystem` (hardening beyond Restricted) | `true` |

**The enforcement half is the operator's, and it is one command:**

```shell
kubectl label namespace ferroehr \
  pod-security.kubernetes.io/enforce=restricted \
  pod-security.kubernetes.io/audit=restricted \
  pod-security.kubernetes.io/warn=restricted
```

**Why the chart does not do this itself**, recorded so it is not re-litigated:
Helm installs *into* a namespace that already exists (or one `helm
--create-namespace` creates, which is the CLI's action and not a template), so a
chart that declared its own release namespace would fight the tool — and
`helm uninstall` would then delete a namespace holding objects the release does
not own. More fundamentally, a PSA label is **namespace-wide policy**: it governs
every workload in that namespace, including backup jobs, sidecars and any database
an operator colocates. A single application chart is the wrong scope to claim it.

Verified on a live cluster, in a namespace labelled `enforce=restricted`, with the
database genuinely external to it:

- **the chart installs and serves unchanged** — both replicas `1/1 Running`, zero
  restarts, readiness `UP`, `POST /ehr` → `201`. No warnings for its pods.
- **a regression of the chart's own pod spec is refused.** Upgrading with
  `--set securityContext.privileged=true --set allowPrivilegeEscalation=true`
  produced a ReplicaSet with `DESIRED 1, CURRENT 0` and:

  ```text
  Error creating: pods "ferroehr-…" is forbidden: violates PodSecurity
  "restricted:latest": privileged (container "ferroehr" must not set
  securityContext.privileged=true), allowPrivilegeEscalation != false
  ```

  while the healthy ReplicaSet stayed at **2/2** — because `maxUnavailable: 0`
  means the rollout cannot retire a good pod before a replacement is ready. The
  two controls compose: the label refuses the regression, and the strategy means
  the refusal costs no availability.

**What you get by not applying the label:** the chart still complies, but nothing
*enforces* it. A future chart change, a stray `--set securityContext.privileged=true`,
or a sidecar injected by other tooling would be admitted, and the first sign would
be a running privileged container in the namespace holding your PHI.

One practical note from that test: a colocated database fixture is unlikely to be
Restricted-compliant (the upstream `postgres` entrypoint must start as root), so
labelling a namespace that contains one will refuse it. That is another reason the
production posture puts the database outside the cluster.

## Service mesh: a recorded decision

**Not adopted, deliberately.** The cheat sheet presents a mesh as a trade-off, not
a requirement, and for this workload the trade lands clearly.

What a mesh would provide, and what already provides it:

| Mesh benefit | Already covered by |
|---|---|
| mTLS between services | the server terminates TLS natively (`[server.tls]`), including mutual TLS for the IHE ATNA ITI-19 posture; the database connection uses `sslmode=verify-full` |
| East-west traffic restriction | the shipped NetworkPolicy, proven enforcing on this cluster |
| Request-level observability | OTLP traces and Prometheus metrics from the application, which sees openEHR operations rather than an L7 proxy's view of opaque HTTP |
| An audit trail of access | the ATNA/BALP audit trail, which records *who read which patient's record* — a property no proxy can reconstruct |

Against that: a mesh is a second control plane, a sidecar in every pod (which the
Restricted profile and the empty capability set then have to accommodate), and an
opinionated platform to upgrade in lockstep. For **one workload plus an external
database**, the lateral-movement problem a mesh exists to solve barely exists.

**A genuine gap, named rather than glossed:** a mesh would give *workload
identity* — SPIFFE-style cryptographic identity per pod, so the database could
authenticate the client workload rather than a shared password held in a Secret.
Nothing in this deployment provides that; the DSN is a bearer credential, and any
process that can read the Secret can use it. The mitigations available without a
mesh are an external secret manager plus short-lived credentials (for example
cloud IAM database authentication, where the DSN carries a rotating token rather
than a password). If you run a mesh anyway, expect the overlaps above rather than
double-implementing them — and note that its sidecar will need its own PSA
accommodation.

## Centralized policy, and which engine

**Operator's, and only for one of OPA's three use cases.**

- **Application authorization — already solved, do not add a second engine.** This
  server ships a policy-driven authorization layer: RBAC, plus ABAC with an
  embedded Cedar engine or an external PDP. Adding OPA for application decisions
  would mean two policy engines disagreeing about one question, and the one that
  loses is whichever is consulted second.
- **Service-mesh authorization — moot**, no mesh ([above](#service-mesh-a-recorded-decision)).
- **Admission control — applies**, and it is the same lever image provenance and
  Pod Security enforcement need.

**Decision: Kyverno**, chosen for what it must enforce rather than on general
merit. It is the only one of the three candidates that covers **both** levers with
one controller: `verifyImages` does keyless signature and attestation
verification, and its policies can enforce pod-security constraints beyond what
namespace labels express. `sigstore-policy-controller` does provenance only, so it
would have to be paired with something else. Kubernetes' built-in
**ValidatingAdmissionPolicy** (CEL, no external controller) is attractive for pod
shape and is the right tool for cheap structural rules — but it **cannot verify
signatures at all**, because CEL evaluation makes no network calls and cannot
reach a transparency log, so it cannot be the answer to the control that matters
most here.

If you already run OPA/Gatekeeper for other reasons, keep it and add
`sigstore-policy-controller` alongside for provenance; do not run two general
admission engines. The copyable provenance policy is
[above](#image-provenance-at-admission), and Pod Security enforcement needs no
engine at all — it is the namespace label in the previous section.
