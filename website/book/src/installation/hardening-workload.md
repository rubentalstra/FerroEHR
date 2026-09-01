# The workload: security context & admission

The pod-level controls: what the chart asks for, what the runtime actually
applies, and the one command that turns compliance into enforcement.

<!-- toc -->

## The security context, and what keeps it true

**Ours, and settled.** Read back from the *running container* through the
container runtime's own view of the pod, not from `values.yaml`:

```text
process.user     : {'additionalGids': [65532], 'gid': 65532, 'uid': 65532}
noNewPrivileges  : True
capabilities.bnd : None            ← an EMPTY bounding set, not "default minus ours"
root.readonly    : True
seccomp default  : SCMP_ACT_ERRNO
RW mounts        : /tmp  (plus the kernel/kubelet-managed /proc, /dev/*, /etc/hosts)
RO mounts        : /etc/ferroehr  /etc/ferroehr-secrets  /sys  /sys/fs/cgroup
```

Three things worth drawing out. The capability bounding set is **empty**, so the
drop is total at the kernel level rather than a subtraction from a runtime's
default set. `readOnlyRootFilesystem: true` needed exactly **one** writable
path (the chart's own `/tmp` emptyDir) and no per-integration surprise, which
is what makes it safe to keep rather than the setting an operator relaxes during
the first incident. And the server pod carries no init containers and no
sidecars, so the context above is the whole pod.

What keeps it true is not this page. `deploy/helm/validate.sh` parses the rendered
objects and asserts the Restricted fields **per container, for every workload in
the render** (including the optional viewer and the migration Job) and
the golden renders pin the exact bytes, so even a changed default fails a diff.
Both run in CI on any change to the chart. That structure is deliberate: the gate
this replaced grepped the rendered file for field names, so one compliant
container vouched for every other one, and a second workload could ship
non-compliant while the gate stayed green. The gate also fails when it finds
nothing to check, because a pod-less render reporting "all containers compliant"
is a false green, and it fails when a render carries no NetworkPolicy, or when a
multi-replica Deployment has neither spread constraints nor affinity.

A template edit that drops `securityContext.readOnlyRootFilesystem` or
`securityContext.allowPrivilegeEscalation` therefore fails a job, not a review.

## Beyond Restricted: the user namespace

The Restricted profile stops a container from *asking* for privilege. It does
nothing about what a container's UID means **on the node**, and under the
Kubernetes default, uid 65532 in the pod is uid 65532 on the host, so a container
escape arrives as a real host user with whatever that user can reach.

The chart closes that by default. Every pod it renders carries `hostUsers: false`,
which puts the pod in its own user namespace and maps its UID range onto an
unprivileged host range. Read off a running pod:

```text
$ cat /proc/self/uid_map
         0     838860800      65536
$ id
uid=65532 gid=65532 groups=65532
```

The process still sees uid 65532; the kernel sees an offset host UID inside that
mapped range. Root *inside* the pod (which this workload never uses, but a
compromised process might reach for) maps to the base of the range, which owns
nothing. Capabilities granted inside the namespace do not apply outside it.

This is why the chart's `kubeVersion` floor is **1.36**: that is the release where
user namespaces went stable
([KEP-127](https://kubernetes.io/docs/tasks/configure-pod-container/user-namespaces/)),
and the floor is what lets the field render unconditionally instead of being
gated and silently absent on the clusters that most needed it.

**If your nodes cannot support it**, the pod does not start, which is the failure
mode you want rather than a silent downgrade. The requirement is a Linux node
whose runtime implements idmapped mounts (containerd 2.0 or newer, CRI-O 1.25 or
newer). Set `hostUsers: true` to opt out; the chart then omits the field entirely
rather than stating the API default, so a future cluster-wide default can still
apply.

The same reasoning drives `podSecurityContext.supplementalGroupsPolicy: Strict`,
so the process gets only the groups the manifest names: a group baked into an
image cannot widen file access. And the chart's render gate asserts that this
isolation set is **identical across every workload of a release**: a viewer that
shared the host user namespace while the server did not would be a posture nobody
could state in one sentence, and that is exactly the shape of drift a second
workload introduces. That is also why `hostUsers` is a release-wide key rather
than one per workload.

## AppArmor, and why it is not on by default

`securityContext.appArmorProfile` (stable since Kubernetes 1.31) is a further
confinement layer, and it is left off deliberately, because it is not free: a node
**without** AppArmor rejects the pod outright rather than ignoring the field.
Observed on a node that does not carry it:

```text
STATUS: AppArmor
Warning  AppArmor  pod/…  Cannot enforce AppArmor: AppArmor is not enabled on the host
```

Turn it on once you know your nodes carry it: most Debian and Ubuntu nodes do,
while Docker Desktop and several minimal distributions do not:

```yaml
securityContext:
  appArmorProfile:
    type: RuntimeDefault
```

## Pod Security Admission: complying versus being refused

Meeting the Restricted profile and being *refused* when you stop meeting it are
different properties, and only the first is ours.

**Restricted compliance, field by field**, so the claim is checkable against the
[standard](https://kubernetes.io/docs/concepts/security/pod-security-standards/)
rather than asserted:

| Restricted requires | The chart sets |
|---|---|
| `hostNetwork`/`hostPID`/`hostIPC` unset | none set |
| no privileged containers | `securityContext.privileged: false` |
| `allowPrivilegeEscalation: false` | set |
| capabilities dropped to `ALL` (only `NET_BIND_SERVICE` may be added) | `drop: [ALL]`, none added |
| `runAsNonRoot: true` | set, pod and container, uid/gid 65532 |
| `seccompProfile.type` `RuntimeDefault` or `Localhost` | `RuntimeDefault`, pod and container |
| no hostPath volumes | only `emptyDir` and `projected` volumes (the latter carrying the ConfigMap and Secret sources) |
| `readOnlyRootFilesystem` (hardening beyond Restricted) | `true` |

**The enforcement half is the operator's, and it is one command:**

```shell
kubectl label --overwrite namespace ferroehr \
  pod-security.kubernetes.io/enforce=restricted \
  pod-security.kubernetes.io/audit=restricted \
  pod-security.kubernetes.io/warn=restricted
```

The install notes print the same command as a prerequisite, with
`enforce-version=latest` so the profile does not silently loosen as the cluster
moves.

**Why the chart does not do this itself**, recorded so it is not re-litigated:
Helm installs *into* a namespace that already exists (or one
`helm --create-namespace` creates, which is the CLI's action and not a template),
so a chart that declared its own release namespace would fight the tool, and
`helm uninstall` would then delete a namespace holding objects the release does
not own. More fundamentally, a PSA label is **namespace-wide policy**: it governs
every workload in that namespace, including backup jobs, sidecars and any database
an operator colocates. A single application chart is the wrong scope to claim it.

Observed on a live cluster, in a namespace labelled `enforce=restricted`, with the
database genuinely external to it:

- **the chart installs and serves unchanged:** both replicas running with no
  restarts, readiness `UP`, and a real openEHR write accepted. No admission
  warnings for its pods.
- **a regression of the chart's own pod spec is refused.** Upgrading with
  `--set securityContext.privileged=true --set securityContext.allowPrivilegeEscalation=true`
  produced a ReplicaSet that could create no pods, and:

  ```text
  Error creating: pods "ferroehr-…" is forbidden: violates PodSecurity
  "restricted:latest": privileged (container "ferroehr" must not set
  securityContext.privileged=true), allowPrivilegeEscalation != false
  ```

  while the healthy ReplicaSet kept both replicas serving, because
  `maxUnavailable: 0` means the rollout cannot retire a good pod before a
  replacement is ready. The two controls compose: the label refuses the
  regression, and the strategy means the refusal costs no availability.

**What you get by not applying the label:** the chart still complies, but nothing
*enforces* it. A future chart change, a stray `--set securityContext.privileged=true`,
or a sidecar injected by other tooling would be admitted, and the first sign would
be a running privileged container in the namespace holding your PHI.

One practical note from that test: a colocated database fixture is unlikely to be
Restricted-compliant (the upstream `postgres` entrypoint must start as root), so
labelling a namespace that contains one will refuse it. That is another reason the
production posture puts the database outside the cluster.

## Sandboxing is not tenant isolation

**Not required, and the reason matters more than the conclusion.** The cheat sheet
scopes sandboxing (Kata, gVisor, Firecracker) to clusters running *untrusted*
workloads. This is our own code, so the threat it addresses (a container escape
by hostile software you chose to run) is not the one in front of us. And the
escape it hardens against is already narrowed by the pod's own user namespace,
which is on by default.

> [!IMPORTANT]
> **A container sandbox does nothing for this server's multi-tenancy**, and that
> is the misreading worth preventing. Tenants of a single release share **one
> process and one database**; they are separated by PostgreSQL row-level security
> and a per-request session setting, *inside* the container. A sandbox draws a
> stronger boundary around the whole container, which both tenants are already on
> the same side of. Hardening the sandbox changes nothing about tenant isolation;
> only the [namespace-per-tenant
> model](hardening-network-policy.md#namespaces-and-the-two-tenant-models) moves
> that boundary.

A sandbox is worth considering in one case: a cluster where this workload runs
**beside** third-party or customer-supplied code, and you want to protect this
workload's node from *that*. Note the cost first: a sandboxed runtime changes the
syscall surface and the performance profile of a database-bound server, and the
gVisor/Kata runtimes need a `RuntimeClass` the chart does not set (add it through
your platform's pod defaults if you adopt one).

## Kernel modules: already impossible

**Satisfied by the chart's own posture**, which is unusual for a host-side control
and worth recording precisely rather than deferring.

Loading a kernel module requires **`CAP_SYS_MODULE`**. The container's capability
bounding set is **empty** (read off the running container, not off the
manifest's request) and `allowPrivilegeEscalation: false` with
`noNewPrivileges` set means no `setuid` binary can regain it. The pod's own user
namespace makes the point twice over: a capability held inside it does not apply
outside it. So this container cannot trigger a module load at all, whatever
`/etc/modprobe.d/` on the host says. An *attempt* would still be a
[runtime-detection
signal](hardening-detection-response.md#runtime-detection-on-a-shell-less-image);
it just cannot succeed.

Host-side blacklisting remains good practice **for the node**, and stays the
operator's: it constrains every other workload on that node, including ones with
capabilities this one does not have, and a privileged pod anywhere on the node can
still load modules that affect this container's kernel.
