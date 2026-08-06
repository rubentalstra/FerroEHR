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
   [Secrets at rest](#secrets-are-not-encrypted-at-rest-by-default).
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
