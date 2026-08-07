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
| Container resource bounds | **chart** | [below](#resource-bounds-four-layers) |
| Namespace `ResourceQuota`/`LimitRange` | operator | [below](#resource-bounds-four-layers) |
| Egress restriction | **chart** (mechanism) + operator (destinations) | [below](#egress-deny-by-default-and-what-it-breaks) |
| Secrets encrypted at rest | operator | [below](#secrets-at-rest-and-what-ours-contain) |
| Runtime/syscall detection | operator (unusually cheap here) | [below](#runtime-detection-on-a-shell-less-image) |
| Container sandboxing | neither — recorded decision | [below](#sandboxing-is-not-tenant-isolation) |
| Kernel-module loading | **satisfied by the chart** | [below](#kernel-modules-already-impossible) |
| Replica behavioural deviation | operator (from metrics we publish) | [below](#replica-deviation-and-the-outbound-inventory) |
| Breach containment + credential rotation | operator (procedure is ours) | [below](#breach-containment-and-rotating-credentials) |
| Cluster API audit logging | operator | [below](#logging-two-streams-that-are-not-interchangeable) |
| Container logging | **chart/app** | [below](#logging-two-streams-that-are-not-interchangeable) |
| Managed control plane | provider | [below](#on-a-managed-control-plane) |
| Supply chain | **CI**, with two gaps | [below](#the-supply-chain-map) |

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
| Non-root by construction | the image declares `USER 65532:65532` (numeric, so the kubelet can verify it), and the pod pins `runAsNonRoot` + uid 65532 independently |

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

The publishing lanes attest their artifacts through **keyless Sigstore**, so a
verifier can establish that an artifact came from this repository's build — the
three published images carry a signed SLSA v1 provenance attestation today.
Nothing in a cluster *requires* that check before running one, and a signature
nobody verifies changes nothing about what actually runs. That is what the
policies below close.

### The identity to trust, read off a real artifact

Each lane signs with a short-lived Fulcio certificate whose **subject
alternative name is the workflow that issued the token**, and whose issuer is
GitHub's OIDC provider. The values below are not derived from the workflow
files — they are read out of the certificate on a published image:

```shell
gh attestation verify oci://ghcr.io/rubentalstra/ferroehr:develop \
  -R rubentalstra/FerroEHR --format json \
  | jq '.[0].verificationResult.signature.certificate
        | {subjectAlternativeName, issuer, sourceRepositoryRef, runnerEnvironment}'
```

```json
{
  "subjectAlternativeName": "https://github.com/rubentalstra/FerroEHR/.github/workflows/containers.yml@refs/heads/develop",
  "issuer": "https://token.actions.githubusercontent.com",
  "sourceRepositoryRef": "refs/heads/develop",
  "runnerEnvironment": "github-hosted"
}
```

**The SAN's ref varies with the trigger, and that is the part a policy gets
wrong.** Each lane runs on more than one ref, so each issues more than one
identity:

| Artifact | Signing workflow | SAN on a release build | SAN on a development build |
|---|---|---|---|
| the three images | `containers.yml` | `…/containers.yml@refs/tags/vX.Y.Z` | `…/containers.yml@refs/heads/develop` |
| the chart | `publish-chart.yml` | `…/publish-chart.yml@refs/tags/vX.Y.Z` | `…/publish-chart.yml@refs/heads/develop` (a `workflow_dispatch` chart-only publish) |
| the release binaries | `release-build.yml` | `…/release-build.yml@refs/tags/vX.Y.Z` | *(none — the lane only runs on a tag)* |

All three prefixed with `https://github.com/rubentalstra/FerroEHR/.github/workflows/`,
and all with issuer `https://token.actions.githubusercontent.com`.

The release binaries are signed by `release-build.yml` rather than by
`release.yml` because the build lives in a **reusable** workflow — the
certificate names the workflow that owns the build definition, which is what
makes the `--signer-workflow` pin below meaningful.

**Pick the ref set deliberately, because the choice is a refusal.** A policy
matching `refs/tags/v…` only admits released images and **refuses
`ghcr.io/rubentalstra/ferroehr:develop`** — correct for production, and the
reason a policy tested against `:develop` appears broken when it is working. A
staging cluster that runs `:develop` needs both refs. Nothing accepts an
arbitrary branch: `refs/heads/develop` is exact, not a prefix match.

### Kyverno

The engine [chosen below](#centralized-policy-and-which-engine). Two details
decide whether this policy works at all:

- **`type: SigstoreBundle`.** These attestations are GitHub Artifact
  Attestations, stored in the [Sigstore bundle
  format](https://docs.sigstore.dev/about/bundle/) as an OCI referrer. Kyverno
  reads that format only under this type; the field
  [defaults to `Cosign`](https://kyverno.io/docs/policy-types/cluster-policy/verify-images/sigstore/),
  which looks for a `sha256-<digest>.sig` tag that these images do not have (it
  returns 404 — the bundle is a referrer, not a cosign tag). Requires
  **Kyverno 1.13 or newer**.
- **`attestations:`, not `attestors:` alone.** Kyverno's own rule is that
  "each `verifyImages` rule can be used to verify signatures or attestations,
  but not both", and what the lane produces is a signed *attestation* — there
  is no detached image signature. A rule with `attestors:` at the top level
  therefore fails **closed** on a perfectly legitimate image.

```yaml
apiVersion: kyverno.io/v1
kind: ClusterPolicy
metadata:
  name: ferroehr-image-provenance
  annotations:
    pod-policies.kyverno.io/autogen-controllers: none
spec:
  background: false
  webhookTimeoutSeconds: 30
  rules:
    - name: verify-ferroehr-provenance
      match:
        any:
          - resources:
              kinds: [Pod]
              namespaces: [ferroehr]
      verifyImages:
        - imageReferences:
            - "ghcr.io/rubentalstra/ferroehr"
            - "ghcr.io/rubentalstra/ferroehr:*"
            - "ghcr.io/rubentalstra/ferroehr-admin-ui*"
            - "ghcr.io/rubentalstra/ferroehr-postgres*"
          # Sigstore bundle format — GitHub Artifact Attestations. Omitting
          # this defaults to Cosign, which looks for a signature that does not
          # exist and refuses every image.
          type: SigstoreBundle
          failureAction: Enforce
          attestations:
            - type: https://slsa.dev/provenance/v1
              attestors:
                - count: 1
                  entries:
                    - keyless:
                        issuer: https://token.actions.githubusercontent.com
                        # Released images only. For a staging cluster that runs
                        # the development tag, make the group
                        # `(heads/develop|tags/v.+)`.
                        subjectRegExp: '^https://github\.com/rubentalstra/FerroEHR/\.github/workflows/containers\.yml@refs/(tags/v.+)$'
                        rekor:
                          url: https://rekor.sigstore.dev
              conditions:
                - all:
                    - key: '{{ buildDefinition.buildType }}'
                      operator: Equals
                      value: https://actions.github.io/buildtypes/workflow/v1
```

`failureAction` sits on the `verifyImages` entry: the spec-level
`validationFailureAction` is
[deprecated in the CRD](https://github.com/kyverno/kyverno/blob/main/config/crds/kyverno/kyverno.io_clusterpolicies.yaml)
("use `validationFailureAction` under the validate rule instead"), and a
`verifyImages` rule has no validate block. `mutateDigest`, `verifyDigest` and
`required` all default to `true`, which is what you want: a tag is rewritten to
the digest that was verified, and an image with no attestation is refused
rather than passed.

### sigstore-policy-controller

If you already run it. The equivalent two details:

- **`signatureFormat: bundle`** on the authority. The default is `legacy`
  (cosign's own), which cannot read these attestations. Requires
  **policy-controller v0.13.0 or newer**.
- **an `attestations:` entry**, because in bundle format policy-controller
  supports "only attestations, not plain signatures".

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
            # Same group as the Kyverno policy: `(heads/develop|tags/v.+)` for a
            # staging cluster that runs the development tag.
            subjectRegExp: '^https://github\.com/rubentalstra/FerroEHR/\.github/workflows/containers\.yml@refs/(tags/v.+)$'
      signatureFormat: bundle
      attestations:
        - name: require-slsa-provenance
          predicateType: https://slsa.dev/provenance/v1
```

> [!NOTE]
> **What has been checked, and what has not.** The identity, the issuer and the
> predicate type above are verified first-hand against the three published
> images — `cosign verify --certificate-identity-regexp …` (cosign 3.1.3) admits
> all three and refuses both an unsigned image and, under a tags-only pattern, a
> `:develop` image, so the matcher is neither vacuous nor accidentally
> permissive. The manifests are checked field-by-field against the published
> `ClusterPolicy` and `ClusterImagePolicy` CRDs. **Neither policy has been
> exercised by a running admission controller**, and the Kyverno CLI is no
> substitute: `kyverno test` reports a `verifyImages` rule as `Excluded` and
> returns the same verdict whichever result you assert. Before enforcing, run
> the policy in `Audit` (Kyverno) or as a `warn` policy (policy-controller) long
> enough to see one real deployment pass.

**No chart policy is offered yet.** No chart version has been published, so
there is nothing to reconcile a chart identity against; the row in the table
above is what the lane *will* issue, not something read off an artifact. Verify
the chart with the manual command below once a chart version exists.

**Should the chart ship one? No, and the reason is structural:** an admission
policy is cluster-scoped and governs workloads the chart knows nothing about,
while the chart deliberately renders no cluster-scoped object at all (see
[namespaces](#namespaces-and-the-two-tenant-models)). A `ClusterPolicy` in this
chart would mean `helm uninstall` removing a control that other releases had come
to depend on. The chart's contribution is the policy *document*, here, versioned
with the lanes whose identity it encodes.

**Without an admission controller**, the manual equivalent is a release-time
check rather than a per-pod one. Add `--signer-workflow` to insist on the lane
as well as the repository — without it you are trusting that *some* workflow
here signed the image:

```shell
gh attestation verify oci://ghcr.io/rubentalstra/ferroehr:develop \
  -R rubentalstra/FerroEHR \
  --signer-workflow rubentalstra/FerroEHR/.github/workflows/containers.yml
```

Substitute a `vX.Y.Z` tag for `develop` on a release; the signer workflow is the
same. Both forms are verified working on the current `develop` images.

> [!IMPORTANT]
> Signing landed in the publishing lanes during the `3.17.4` cycle, so
> `ferroehr:3.17.3` and every earlier tag answer `HTTP 404: Not Found` — there is
> nothing to verify, not a verification failure. The chart command
> (`oci://ghcr.io/rubentalstra/charts/ferroehr:<chart-version>`) has nothing to
> answer for yet either: no chart version has been published.

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
  `--set securityContext.privileged=true --set securityContext.allowPrivilegeEscalation=true`
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

## Resource bounds: four layers

**Split exactly.** Container requests and limits are the chart's; namespace
`ResourceQuota` and `LimitRange` are cluster-admin objects, and a workload chart
that created one would be claiming the whole namespace for itself — wrong the
moment anything else shares it.

The chart's bounds, and where the numbers come from:

| | Value | Derivation |
|---|---|---|
| `requests.cpu` | `250m` | the scheduling floor: enough to boot, run migrations and serve steady traffic. Measured steady-state use on an idle-but-serving pod is **2m**, so this is deliberately generous rather than tuned to the floor — a request is what the scheduler reserves, and a too-tight one gets the pod placed on a node with nothing left for a traffic spike. |
| `requests.memory` | `256Mi` | measured steady-state **33Mi** resident, plus room for the connection pool, the template/WebTemplate cache and per-request AQL working set. |
| `limits.cpu` | `2` | AQL execution is the CPU-heavy path and is bounded per query by `query.timeout_ms`; two cores lets a query and normal traffic proceed without throttling. CPU limits throttle rather than kill, so this trades latency, not availability. |
| `limits.memory` | `1Gi` | a **hard** ceiling: exceeding it is an OOM kill, so it sits well above observed use. The application-level body and result limits below are what keep a single request from approaching it. |

Those numbers are for a modest replica. Tune them from your own metrics rather than
treating them as a recommendation — and remember that raising `replicaCount`
multiplies the request, which is what a namespace quota will notice first.

**The layering is the point**, and an operator should see it as one story — four
nested bounds, each catching what the next cannot:

1. **Per request** — `server.limits.body_bytes` (413 on an over-large body),
   `query.timeout_ms` and `query.max_result_rows` (a query cannot return an
   unbounded row set), `db.statement_timeout_ms` as the backstop the HTTP timeout
   cannot be.
2. **Per connection / per caller** — `server.connection.*` bounds a socket before
   a request exists; `server.rate_limit` refuses `429`; the in-flight shed answers
   `503` when the server is full.
3. **Per container** — the requests and limits above.
4. **Per namespace** — the operator's, and the only layer that protects *other*
   workloads from this one:

```yaml
apiVersion: v1
kind: ResourceQuota
metadata:
  name: ferroehr
  namespace: ferroehr
spec:
  hard:
    requests.cpu: "2"
    requests.memory: 4Gi
    limits.cpu: "12"
    limits.memory: 8Gi
    pods: "12"
    count/services: "4"
---
apiVersion: v1
kind: LimitRange
metadata:
  name: ferroehr-defaults
  namespace: ferroehr
spec:
  limits:
    - type: Container
      # A pod that arrives with no limits gets these, so nothing in the namespace
      # can be unbounded by omission.
      default: {cpu: "1", memory: 512Mi}
      defaultRequest: {cpu: 100m, memory: 128Mi}
      max: {cpu: "4", memory: 2Gi}
```

Size the quota above `replicaCount × requests` with headroom for a rolling
upgrade — `maxSurge: 1` means one extra pod exists mid-rollout, and a quota with
no room for it makes upgrades stall rather than fail, which looks like a hung
deployment.

## Egress: deny by default, and what it breaks

**The mechanism is the chart's; the destinations are yours** — and this is the
control where a NetworkPolicy stops being free.

Enabling `networkPolicy.egress.enabled` refuses **all** outbound traffic except
what is listed. DNS is always included, and the database is a first-class value
because it is the only destination that is never optional:

```yaml
networkPolicy:
  egress:
    enabled: true
    database:
      to:
        - ipBlock: {cidr: 10.0.12.7/32}     # a managed database
        # or, in-cluster:
        # - podSelector: {matchLabels: {app.kubernetes.io/name: postgres}}
      port: 5432
    rules: []                                # one entry per integration, below
```

> [!IMPORTANT]
> Enabling egress with **no** database destination is refused at render time
> rather than at rollout, because that mistake presents as a database fault:
> readiness reports the DB down, the log shows a connect timeout, and nothing
> mentions the network policy.

**The destination set, derived from the configuration tree rather than from what a
default install happens to use.** Every row is off unless its key is set, so add
only the rows you have switched on:

| Destination | Turned on by | Port | Typically |
|---|---|---|---|
| Cluster DNS | always | UDP+TCP 53 | in-cluster (always allowed) |
| PostgreSQL | `db.url` — **never optional** | the DSN's (5432) | off-cluster (managed) |
| OTLP collector | `telemetry.otlp_endpoint` | 4317 gRPC / 4318 HTTP | in-cluster |
| OIDC issuer (discovery + JWKS) | `auth.oidc.issuer`, **unless** `auth.oidc.jwks_json`/`jwks_json_file` is supplied | 443 | off-cluster |
| External policy decision point | `authz.abac.engine: remote` + `authz.abac.remote.server` | the URL's (3001) | in-cluster |
| FHIR terminology server(s) | `terminology.external.enabled` + `…providers.<name>.url` | 443 | off-cluster |
| Terminology token endpoint | `…oauth2_clients.<name>.token_url` | 443 | off-cluster |
| AMQP broker (events) | `events.enabled` + `secrets.eventsUrl` | 5672, or 5671 with `events.tls` | in-cluster |
| AMQP broker (FHIR outbound) | `fhir.outbound.enabled` + `secrets.fhirOutboundUrl` | 5672 / 5671 | in-cluster |
| Object store | `multimedia.enabled` + `multimedia.endpoint` (unset ⇒ AWS regional resolution) | 443, or the endpoint's | off-cluster |
| Syslog audit repository | `audit.syslog.enabled` | 514 UDP, or 6514 TCP with `transport: tls` | off-cluster |
| FHIR audit repository | `audit.fhir_feed.enabled` + `secrets.auditFhirFeedUrl` | 443 | off-cluster |
| Subject-proxy source system | `subject_proxy.systems.<name>.base_url` | 443 | off-cluster |

> [!WARNING]
> **A NetworkPolicy cannot match a DNS name.** The API selects peers by pod,
> namespace or `ipBlock` CIDR only, so every off-cluster row above needs a CIDR
> you supply and keep current. A managed database that moves IP, or a terminology
> server behind a rotating CDN address, will break under a CIDR that was correct
> when written. Where a provider publishes no stable range, the honest options are
> an egress gateway with a fixed address, or leaving egress off and accepting that
> outbound traffic is unrestricted — not a `0.0.0.0/0` rule that pretends to be a
> policy.

**Two failure modes worth knowing before you tighten this.**

**An over-tight policy silently stops observability.** A blocked OTLP exporter
does **not** fail the request that generated the span — it drops the span, with no
log line and no error. So a policy that forgets the collector produces a server
that is healthy by every check and has quietly stopped being observable. If you
enable egress and traces disappear, look at the policy before the collector.

**Tightening egress under a running pod appears to work when it has not.** A
NetworkPolicy is enforced on new connections; existing conntrack flows survive. So
a pod whose connection pool is already established keeps serving after you remove
its database rule — and fails at the next restart, which may be a node drain at
3am. Measured on this cluster:

```text
policy: DNS + database  → readiness 200 {"status":"UP", db UP, migrations UP}, POST /ehr 201
remove the database rule (DNS only), pod untouched
                        → readiness STILL 200, db STILL "UP"   ← the pool survives
delete the pods so a fresh one must connect
                        → readiness 503, both replicas unavailable
                          kubelet: Readiness probe failed: HTTP probe failed with statuscode: 503
restore the rule        → 2/2 ready again, with no restart
```

So: **verify an egress policy by restarting a pod, not by watching the one that is
already running.** (The recovery in the last line needing no restart is the
readiness check re-testing its dependencies on every probe — the same property
described for [migrations](kubernetes.md#health-probes).)

## Secrets at rest, and what ours contain

**Operator's** — encryption at rest is an API-server flag
(`--encryption-provider-config`), and **Kubernetes does not encrypt Secrets by
default**. A `Secret` is base64-encoded, which is an encoding and not a
protection: without that configuration, everything below sits readable in etcd,
which is why [the etcd section](#etcd-and-what-our-secrets-contain) is the other
half of this one.

**The useful half is ours: exactly what this deployment's Secrets contain**, so
you can judge the exposure rather than reading "secrets" generically.

| Secret content | Present when | What it gets an attacker |
|---|---|---|
| **The database DSN** | always | **direct read/write access to all patient data**, bypassing the API, its authorization and its audit trail entirely |
| The whole rendered `ferroehr.toml` | a Basic user is configured (it moves to a Secret) | the configuration, including that user's Argon2id hash |
| A Basic user's Argon2id hash | as above | an offline cracking target, not a usable password |
| The OIDC HMAC secret | `secrets.authOidcHmacSecret` (HS256 development setups) | the ability to **mint valid tokens** for any user and role |
| The version-signing passphrase (+ the PGP key via `config.files`) | `signing.mode: pgp` | the ability to forge version signatures, breaking the integrity guarantee |
| Terminology `client_secret` | `secrets.terminologyOauth2ClientSecrets.*` | access to that terminology server as this client |
| AMQP broker URLs | `secrets.eventsUrl`, `secrets.fhirOutboundUrl` | the FHIR outbound stream **carries PHI**; the events stream is PHI-free by design |
| The audit repository URL | `secrets.auditFhirFeedUrl` | the ability to read or forge audit records at the ARR |
| S3 credentials | `secrets.multimedia*` | offloaded `DV_MULTIMEDIA` blobs, which **are PHI** |

The first row is the one that matters most, and it has a property worth naming:
**the DSN is a bearer credential.** Any process that can read it can use it, from
anywhere the database is reachable — there is no binding to the workload that was
issued it. That is the same gap named under
[service mesh](#service-mesh-a-recorded-decision) (workload identity), reached
from the other direction, and the mitigation is the same: a credential that is
short-lived and issued to a workload identity rather than a long-lived password in
an object.

**Enable encryption at rest** with an
[`EncryptionConfiguration`](https://kubernetes.io/docs/tasks/administer-cluster/encrypt-data/)
on every API server. Prefer a KMS provider over `aescbc`/`secretbox` with a local
key: a key sitting in a file on the control-plane node is protected by the same
boundary as the etcd data it encrypts. On a managed cluster this is usually one
setting (envelope encryption with the provider's KMS) — check whether it is on,
because it generally is not by default. Existing Secrets are only re-encrypted
when rewritten, so follow the docs' `kubectl get secrets --all-namespaces -o json |
kubectl replace -f -` step, or the encryption applies to new writes only.

**Or remove them from etcd entirely.** Every secret this chart carries has a
`*_file` route or an `existingSecret` route, so no code change is needed to source
them from a secret manager:

- **A CSI driver** ([Secrets Store CSI
  Driver](https://secrets-store-csi-driver.sigs.k8s.io/) with the Vault, AWS,
  Azure or GCP provider) mounts the value as a file. Point `extraVolumes` /
  `extraVolumeMounts` at it and set the matching `*_file` configuration key —
  `auth.oidc.hmac_secret_file`, `signing.key_passphrase_file`,
  `multimedia.secret_access_key_file`, a terminology client's
  `client_secret_file`. Nothing reaches a Kubernetes Secret at all.
- **An operator that syncs into a Secret** (External Secrets Operator, Vault
  Agent Injector) still lands in etcd, so it buys rotation rather than removal —
  worth having, but pair it with encryption at rest.
- **The DSN** is mounted too, from `database.existingSecret` via `db.url_file`, so
  a CSI-provided Secret carries it like any other. What a mount does **not** fix is
  that the DSN remains a *bearer* credential: cloud IAM database authentication is
  the route that removes the standing password (the DSN then carries a short-lived
  token), and `serviceAccount.annotations` exists for the IRSA/Workload-Identity
  binding that needs.
- **`audit.fhir_feed.url`** is now the only credential-bearing key with no `*_file`
  sibling, so it is the one value still passed as environment.

**Finding exposed secrets** (§4.10.4) is already covered in CI rather than left to
an operator: Trivy's `secret` scanner runs over the whole tree and over every
published image, so a credential committed to the repository or baked into a layer
fails a job. The deliberate development credentials were checked against it and are
not flagged, so nothing is exempted to make it pass.

The volume-versus-environment half of this control — mounting secrets as read-only
files rather than passing them as environment variables — is
[covered on the Kubernetes page](kubernetes.md#secrets-and-mounted-config) and is
already the chart's behaviour for every secret whose configuration key has a
`*_file` sibling.

## Runtime detection on a shell-less image

**Operator's tooling** (Falco, Tetragon, or a managed equivalent), but the signals
are unusually high-confidence here, and that is what makes it worth more than a
recommendation.

The runtime image is **distroless and shell-less**. There is no `sh`, no `bash`,
no `curl`, no package manager, and the container runs one process. So the usual
heuristics stop being heuristics:

| Signal | Why it is unambiguous for this image |
|---|---|
| **Any `execve` of anything other than `/usr/local/bin/ferroehr`** | the image contains no other executable to run — a second process means one arrived from outside |
| **A shell process in the container** | impossible under normal operation; there is no shell in the filesystem |
| **A write outside `/tmp`** | the root filesystem is read-only and `/tmp` is the only declared writable mount — verified on the running container |
| **An outbound connection to anything not in the egress table** | the [inventory](#egress-deny-by-default-and-what-it-breaks) is complete and small |
| **Any attempt to load a kernel module, mount, or change namespaces** | the capability bounding set is empty, so these cannot succeed — an *attempt* is still a signal |

A starter Falco rule set exploiting exactly that:

```yaml
- macro: ferroehr_container
  condition: container.image.repository endswith "/ferroehr"

- rule: FerroEHR unexpected process
  desc: Any process other than the server binary in a shell-less image
  condition: spawned_process and ferroehr_container and proc.exepath != "/usr/local/bin/ferroehr"
  output: Unexpected process in FerroEHR container (proc=%proc.exepath parent=%proc.pname container=%container.id)
  priority: CRITICAL

- rule: FerroEHR write outside tmp
  desc: The root filesystem is read-only; /tmp is the only writable mount
  condition: open_write and ferroehr_container and not fd.name startswith "/tmp/"
  output: Write outside /tmp in FerroEHR container (file=%fd.name proc=%proc.exepath container=%container.id)
  priority: CRITICAL

- rule: FerroEHR unexpected outbound connection
  desc: Outbound to a destination outside the configured inventory
  condition: >
    outbound and ferroehr_container and
    not fd.sport in (53) and not fd.sport in (5432, 4317, 4318, 443, 5671, 5672, 6514)
  output: Unexpected egress from FerroEHR container (dest=%fd.rip:%fd.rport proc=%proc.exepath)
  priority: WARNING
```

Tune the port list to the destinations you actually enabled; the point of the
third rule is that the list is short enough to be worth writing.

**What each layer sees, and cannot.** The syscall layer sees process execution,
file writes and raw connections — a compromise of the *container* — but it has no
idea which patient's record was read, because to it every request is bytes on an
established socket. The **ATNA audit trail** sees exactly that: which subject
accessed which EHR, through which operation, under which authenticated identity —
but it is emitted *by* the application, so a compromise deep enough to control the
process can stop or falsify it. They are complementary and neither substitutes:
runtime detection is how you learn the process is not itself any more; the audit
trail is how you answer what was accessed while it still was. Forwarding audit
records **off-box** to an external repository (`audit.syslog`, `audit.fhir_feed`)
is what keeps the second answer available after the first alarm.

## Sandboxing is not tenant isolation

**Not required, and the reason matters more than the conclusion.** The cheat sheet
scopes sandboxing (Kata, gVisor, Firecracker) to clusters running *untrusted*
workloads. This is our own code, so the threat it addresses — a container escape
by hostile software you chose to run — is not the one in front of us.

> [!IMPORTANT]
> **A container sandbox does nothing for this server's multi-tenancy**, and that
> is the misreading worth preventing. Tenants of a single release share **one
> process and one database**; they are separated by PostgreSQL row-level security
> and a per-request session GUC — *inside* the container. A sandbox draws a
> stronger boundary around the whole container, which both tenants are already on
> the same side of. Hardening the sandbox changes nothing about tenant isolation;
> only the [namespace-per-tenant model](#namespaces-and-the-two-tenant-models)
> moves that boundary.

A sandbox is worth considering in one case: a cluster where this workload runs
**beside** third-party or customer-supplied code, and you want to protect this
workload's node from *that*. Note the cost first — a sandboxed runtime changes the
syscall surface and the performance profile of a database-bound server, and the
gVisor/Kata runtimes need a `RuntimeClass` the chart does not set (add it through
your platform's pod defaults if you adopt one).

## Kernel modules: already impossible

**Satisfied by the chart's own posture**, which is unusual for a host-side control
and worth recording precisely rather than deferring.

Loading a kernel module requires **`CAP_SYS_MODULE`**. The container's capability
bounding set is **empty** — verified on the running container via `crictl`
(`capabilities.bnd: None`), not merely requested in the manifest — and
`allowPrivilegeEscalation: false` with `noNewPrivileges` set means no
`setuid` binary can regain it. So this container cannot trigger a module load at
all, whatever `/etc/modprobe.d/` on the host says. An *attempt* would still be a
[runtime-detection signal](#runtime-detection-on-a-shell-less-image); it just
cannot succeed.

Host-side blacklisting remains good practice **for the node**, and stays the
operator's: it constrains every other workload on that node, including ones with
capabilities this one does not have, and a privileged pod anywhere on the node can
still load modules that affect this container's kernel.

## Replica deviation and the outbound inventory

**Operator's practice, from material the server already publishes** — which makes
it more actionable here than the generic advice.

Replicas of this Deployment are interchangeable: same image, same configuration,
traffic distributed by the Service. So a metric that differs *per pod* is a signal,
and the Prometheus surface is already per-pod (each pod is its own scrape target).
Worth alerting on a divergence between pods rather than on an absolute value:

| Comparison across pods | What a deviation suggests |
|---|---|
| request rate per pod | a load-balancing fault, or one pod being addressed directly, bypassing the Service |
| error ratio (5xx / total) per pod | a pod-local fault: a broken database connection, an exhausted pool, a failing dependency one pod reaches and others do not |
| p99 latency per pod | a throttled pod (CPU limit), a noisy neighbour, or a degraded node |
| authentication-failure rate per pod | credential stuffing aimed at one endpoint — or a token-validation path failing on one pod (an unreachable JWKS endpoint) |
| resident memory slope per pod | a leak or an unbounded working set on one replica only |
| database pool acquire-wait per pod | that pod's pool is starved while others are not |

The `/management/prometheus` endpoint (opened with
`config.management.endpoints.prometheus`) is the source, and
`metrics.serviceMonitor.enabled` is how an operator-managed Prometheus discovers
it. Two things worth knowing: the ATNA audit trail gives a second, independent
view — an access pattern that deviates per pod is visible there at
patient-and-operation granularity — and a pod that fails readiness leaves the
Service, so "one pod has zero traffic" can mean it is unready rather than
unreachable.

**The outbound inventory** the traffic half of this control asks for is the
[egress table above](#egress-deny-by-default-and-what-it-breaks), derived from the
configuration tree. In the chart's default posture, measured from node conntrack on
a running pod, the complete set is **two destinations**: TCP 5432 to the database
and UDP 53 to cluster DNS. Nothing else. That is what makes a deny-by-default
egress policy tractable — the base allowance is two rules, and every addition is a
named, configured endpoint rather than an open range. Compare live traffic against
the policy periodically: a connection the policy permits and nothing makes is a
rule to remove.

## Breach containment and rotating credentials

**Scaling to zero is a clinical-safety decision, and it should be made before you
need it.** `kubectl scale deploy/ferroehr --replicas=0` is the Kubernetes-native
containment action, and for this workload it means **clinical access stops
immediately** — no reads, no commits, for everyone. That is the point during a
breach, and it is also an outage of a system clinicians may be depending on at
that moment. Decide in advance who is authorized to make that call.

What scaling to zero **does**:

- stops all new requests, including whatever the attacker is doing through the API;
- leaves the database untouched — it is external, so its data, its contents and
  its own access controls are unaffected;
- preserves the pod's evidence if you scale rather than delete... **only
  partially**: scaling to zero terminates the pods, so anything in memory is gone.
  To preserve a pod for forensics, cordon its node and remove the pod from the
  Service by editing its labels instead — the ReplicaSet then creates a
  replacement while the original keeps running, detached from traffic.

What it does **not** do:

- **it does not undo committed data.** openEHR change control is append-only, so a
  malicious commit is a new version, not an overwrite. The prior version is still
  there and still retrievable.
- **it does not stop an attacker who has the DSN.** The database is reachable
  independently of these pods; a leaked DSN is used from anywhere the database
  admits, which is why rotating it (below) — not scaling — is the containment
  action for that particular compromise.
- **it does not truncate the audit trail.** Records already written to the local
  store, or already forwarded to an external repository, survive. Records still in
  the outbox at termination are drained during the grace period, so a scale-to-zero
  loses less than a `kill -9`; forwarding to an external repository
  (`audit.syslog`, `audit.fhir_feed`) is what makes the trail survive the pods
  entirely.

### Rotating each credential

Every secret except two is a mounted file, so rotation is: update the Secret, then
**restart the pods**. The restart is not optional — configuration is read at boot,
and Kubernetes propagating a new Secret into the volume does not make a running
process re-read it:

```shell
kubectl -n ferroehr create secret generic ferroehr-db \
  --from-literal=FERROEHR__DB__URL='postgres://…new…' --dry-run=client -o yaml \
  | kubectl apply -f -
kubectl -n ferroehr rollout restart deploy/ferroehr
```

| Credential | Rotation | Notes |
|---|---|---|
| Database DSN | update the Secret → `rollout restart` | rotate the **database** password too, or the old one still works; `maxUnavailable: 0` keeps the old pods serving on the old credential until the new ones are ready, so grant both briefly or accept a gap |
| OIDC HMAC secret | update → restart | invalidates every token signed with the old secret; prefer JWKS/discovery, where the issuer rotates for you and no secret lives here |
| Terminology `client_secret` | update → restart | rotate at the IdP in the same window |
| AMQP broker URLs | update → restart | the credential is inside the URL |
| S3 credentials | update → restart | or remove them entirely with IRSA/Workload Identity |
| Audit repository URL | update → restart | still an environment value (no `*_file` sibling) |
| Basic user password hash | update → restart | rotate the password, re-hash at the OWASP floor |
| **Version signing key** | **read the next section first** | not a restart-and-forget operation |

### The signing key is the one that does not simply rotate

In the default `digest` mode there is no key: `VERSION.signature` is
`sha256:` + the hash of the canonical form, recomputed at read time. Nothing to
rotate, and nothing breaks.

In `pgp` mode, rotation has a consequence the other credentials do not:

> [!WARNING]
> **Replacing the PGP key makes every previously-signed version fail
> verification.** The stored signature carries no key identifier — the schema
> holds `signature text` and nothing else — so read-time verification checks an
> armored PGP signature against *the currently configured key*, and there is only
> one (`signing.key_path`; the chart mounts a single key, and there is no
> keyring). After a rotation, versions signed by the old key verify as
> `pgp_invalid`, which with the default `verify_on_read: strict` is an **integrity
> failure served as a 5xx** on reading historical data.

The signatures themselves cannot be re-issued: a `VERSION`'s signature is an
immutable, committed fact — re-signing would mean rewriting change-controlled
history, which openEHR's append-only model does not permit and which would destroy
the property the signature exists to provide.

So the options, none of which is "rotate and move on":

1. **Treat the PGP signing key as long-lived**, protected accordingly (an HSM or a
   secret manager rather than a chart value), and rotate it only when it is
   actually compromised.
2. **If you must rotate**, set `signing.verify_on_read: warn` before doing so.
   Verification failures are then logged and metered
   (`version_signature_invalid_total{verdict="pgp_invalid"}`) rather than
   returned as 5xx, so historical reads keep working while remaining visible. This
   is a deliberate, recorded reduction in an integrity guarantee — not a setting
   to leave on by default and forget.
3. **Use `digest` mode** where the requirement is tamper-evidence rather than
   attributable authorship. It detects modification of stored content, needs no
   key, and has no rotation problem.

There is no multi-key or keyring support, so "verify old signatures with the old
public key while signing new ones with the new key" is not currently expressible.
If your governance requires periodic signing-key rotation with historical
verifiability, that is a gap to raise before choosing `pgp` mode.

## Logging: two streams that are not interchangeable

**Container logging is ours, and already the right shape.** The server writes to
stdout/stderr and never to a file inside the container — which is both the
[Kubernetes logging
architecture](https://kubernetes.io/docs/concepts/cluster-administration/logging/)'s
expectation and a requirement of `readOnlyRootFilesystem: true`: there is nowhere
to write a log file, and a build that tried would fail rather than silently filling
a writable layer. Set `config.log.format: json` (the chart's default) for a
collector; `pretty` is for a terminal.

**The distinction that matters, because getting it wrong loses the accountability
record:**

| | Application log | ATNA audit trail |
|---|---|---|
| Purpose | diagnostics: what the process is doing | **accountability**: who accessed which patient's record |
| Destination | stdout/stderr → node → your collector | its own store in the database (`audit` schema), plus optional forwarding to an external repository |
| Format | JSON lines, ours | DICOM PS3.15 + FHIR `AuditEvent` (IHE BALP), standardised |
| Retrieval | your log tool | the ITI-81 FHIR `AuditEvent` search endpoint |
| Retention | your collector's policy | `audit.store.retention_days` (`0` = keep forever) |
| May it be sampled or dropped? | **yes** — it is diagnostics | **no** |

> [!IMPORTANT]
> **Do not treat the audit trail as "logs".** A collector configured to sample a
> noisy stream, or to drop under volume, is a reasonable policy for diagnostics and
> a compliance failure for the audit trail — it silently discards the record of who
> read which patient's data. The two travel by different paths precisely so that
> one can be lossy: the audit trail does not go through stdout at all. If you also
> ship audit records to your log platform for convenience, that copy is a
> convenience, not the record.

The audit trail's own failure behaviour is configurable, and the default is worth
knowing rather than inheriting: **`audit.fail_mode` defaults to `open`**, so an
operation whose audit record cannot be written still proceeds, and the failure is
metered rather than refused. `closed` answers `503` instead — the stronger
compliance posture, and one that turns an audit outage into a clinical outage. Which
is correct depends on whether your regulatory position can tolerate an unaudited
access more or less than a refused one; it is a policy choice either way, and the
shipped default chooses availability.

**Cluster API audit logging is the operator's.** Enable it on the API server with
an audit policy (`None` / `Metadata` / `Request` / `RequestResponse` per rule).
`Metadata` for most resources with `Request`-level detail for Secret and RBAC
changes is a reasonable starting shape. Two things worth alerting on specifically:
**authorization failures** (`Forbidden` responses — a principal probing what it can
reach), and any read of Secrets in this namespace by a principal that is not the
kubelet, since that is what reading the DSN looks like from the API side.

**Kubernetes `Events` are a third source**, and distinct from both: they are the
cluster's account of what happened to your objects, they expire (typically an hour),
and they are where this chart's failures show up first —
`Readiness probe failed: HTTP probe failed with statuscode: 503` when a dependency
is down, `FailedCreate … violates PodSecurity "restricted:latest"` when a pod spec
regresses under enforcement, `Unhealthy` and `Killing` during a rollout. Check
`kubectl get events --sort-by=.lastTimestamp` before the application log when a pod
will not start: the reason is usually there, and it is usually not in the log,
because the container never ran.

## On a managed control plane

On EKS, GKE, AKS or an equivalent, **several controls in this audit stop being
yours** — you cannot set API-server flags, reach etcd, or configure kubelet
authentication. In exchange you inherit the provider's defaults, which may be
stronger or weaker than this sheet assumes, and which you should verify rather
than assume.

| Control | On a managed cluster |
|---|---|
| Host hardening, node OS patching | provider's, though **node images and upgrades are usually still yours to trigger** |
| API-server flags, authorization mode | provider's (RBAC is on by default everywhere mainstream) |
| etcd access + encryption at rest | provider's — but **envelope encryption with your own KMS key is usually opt-in**, and it is the control [our Secrets need](#secrets-at-rest-and-what-ours-contain) |
| Kubelet authentication | provider's |
| Control-plane audit logging | provider's to enable, **often off or short-retention by default**, and usually billed |
| Pod Security Admission | yours (namespace labels) |
| NetworkPolicy | yours to write — **enforcement depends on the CNI** |
| Everything the chart sets | unchanged: it is a workload |

**Check the CNI before relying on the shipped NetworkPolicy.** This is the item
that varies most and fails most silently: a NetworkPolicy on a cluster whose
network plugin does not enforce it is an object the API accepts, stores and
displays, with no effect and no warning. Provider defaults differ, versions change
them, and some require enabling enforcement at cluster creation — which cannot be
changed afterwards on some platforms. Do not read your provider's documentation
and conclude; **test it**, the way this audit did:

```shell
kubectl -n ferroehr-probe run probe --image=busybox:1.37 --restart=Never --command -- sleep 600
kubectl -n ferroehr-probe exec probe -- nc -w3 -z <ferroehr-pod-ip> 5432   # must fail
kubectl -n ferroehr-probe exec probe -- nc -w3 -z <ferroehr-pod-ip> 8080   # must succeed
```

If the first command succeeds, the policy is decoration and every claim in this
chapter that rests on it is void for your cluster.

Provider audit tooling exists — for EKS, [hardeneks](https://github.com/aws-samples/hardeneks)
and MKAD are the commonly cited ones. **We have not run them**, so they are named
as a starting point rather than a recommendation: treat their output as input to
the same ownership question this page asks, not as a verdict.

## The supply-chain map

Each cheat-sheet supply-chain control, and the artifact that satisfies it — so a
reader can check rather than trust:

| Control | Satisfied by | Check it yourself |
|---|---|---|
| Trusted, minimal base images | `gcr.io/distroless/cc-debian13:nonroot`, **digest-pinned**; build stages pinned by digest too | `grep FROM docker/Dockerfile` |
| Vulnerability scanning in CI | Trivy on every published image, HIGH/CRITICAL with a fix | the `containers.yml` run log |
| Scanning after release | scheduled weekly scan of the published tags | `.github/workflows/image-scan.yml` |
| Dockerfile linting | hadolint, with adjudicated exceptions in `.hadolint.yaml` | the `Dockerfile lint` job |
| Secret + misconfiguration scanning | Trivy `secret` and `misconfig` over the tree | the `tree-scan` job |
| Dependency advisories | `cargo deny` on every change; a scheduled latest-deps lane | `cargo deny check` |
| Signed images | Sigstore keyless SLSA provenance + SBOM attestations | `gh attestation verify oci://ghcr.io/rubentalstra/ferroehr:<tag> -R rubentalstra/FerroEHR` |
| Signed chart | the same, on the chart artifact | `gh attestation verify oci://ghcr.io/rubentalstra/charts/ferroehr:<version> -R rubentalstra/FerroEHR` |
| SBOM | SPDX on the image index; a CycloneDX cargo-graph SBOM attached per release | `docker buildx imagetools inspect <image> --format '{{json .SBOM}}'` |
| Adjudicated findings carry their argument | OpenVEX under `security/vex/`, applied by the scheduled scan | read the `impact_statement` in the document |
| Secured CI/CD | every `uses:` digest-pinned, `permissions: {}` by default, no context interpolated into shells, zizmor + CodeQL over the workflows | the `zizmor` job |
| No long-lived registry token | crates.io Trusted Publishing (OIDC); GHCR uses the ephemeral workflow token | `.github/workflows/publish-crates.yml` |
| Independent grade | OpenSSF Scorecard, computed by someone other than us | the Scorecard badge |

**Two gaps remain, stated here rather than left out of a page that otherwise reads
as complete:**

1. **Nothing verifies the signatures at admission.** We sign; no cluster is
   required to check before running an image. The policy to close it is
   [above](#image-provenance-at-admission) — and it is not yet verified against a
   live attestation, because none is published yet (issues #2085, #2120).
2. **Provenance exists only from this release onward.** Images published before
   the signing lane landed — `3.17.3` and earlier — carry no attestation and never
   will. `gh attestation verify` on those returns a 404, which is the correct
   answer and not a verification failure to work around.

## Final thoughts: the three practices, checked

The cheat sheet closes with three practices rather than controls. Checking them
against this repository rather than claiming them:

**"Embed security into the container lifecycle as early as possible" — evidenced.**
Security here is CI, not a review checklist: the golden-render gate asserts every
Restricted field on every chart change, the secret-leak gate refuses a credential
that would reach a ConfigMap, the image scanners run at build and weekly after
release, `cargo deny` runs on every change, zizmor and CodeQL read the workflows,
and the guards refuse a merge rather than filing a comment. The test of this claim
is whether any of them has been *watched* to fail, and each of the chart-side ones
has, deliberately, with the failure recorded.

**"Use Kubernetes-native controls to reduce operational risk" — evidenced.** The
chart's controls are the platform's own: a NetworkPolicy rather than an in-app
firewall, a security context and the Restricted profile rather than a hardening
sidecar, resource limits rather than in-process throttling alone, probes rather
than an external watchdog, and [no service mesh](#service-mesh-a-recorded-decision)
because what one would add is either already provided or not needed at this shape.
The one place we did *not* take a native control is Pod Security Admission, and
that is because [labelling a namespace is not a chart's
call](#pod-security-admission-complying-versus-being-refused) — stated as the
operator's rather than quietly skipped.

**"Leverage the context Kubernetes provides to prioritize remediation" — partly,
and here is the honest version.** The generic form of this practice is to rank
findings by whether the affected code is reachable in your deployment. This
project's answer is the **OpenVEX documents**: when a scanner reports fifteen Go
advisories in a privilege-dropping helper that opens no socket and parses no
untrusted input, the response is a machine-readable `not_affected` statement with a
controlled-vocabulary justification and an `impact_statement` a reader can check —
not a silenced ignore list, and not a rebuild that fixes nothing.

That mechanism has a cost, and stating it is what makes the claim honest: **a VEX
statement is a claim about today's binary, and it must be re-checked on every
base-image bump.** When upstream rebuilds that helper, statements about it become
obsolete, and a stale `not_affected` is worse than no VEX at all — it is an
argument that has quietly stopped being true while still suppressing its finding.
The obligation is written into `security/vex/README.md`, and the scheduled scan is
what surfaces a finding whose statement no longer matches. The prioritisation
practice is only as good as that re-check, and it is a human obligation, not an
automated one.
