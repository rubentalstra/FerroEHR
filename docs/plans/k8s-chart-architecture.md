# Kubernetes deployment architecture — derived from the official docs

Working plan for the chart rewrite (#2194). **Deleted in the PR that implements
it**, per `docs/plans/README.md`; the durable record is the PR description and
the chart itself.

Every rule below is taken from the official Kubernetes or Helm documentation and
cited. Where our current chart contradicts one, the contradiction is named with
what it costs — the point of this document is the delta, not a restatement of
the docs.

## 1. Probes — three roles, and ours conflates two

**The rule.** From [Liveness, Readiness and Startup
Probes](https://kubernetes.io/docs/concepts/workloads/pods/probes/):

> Liveness probes can be a powerful way to recover from application failures,
> but they should be used with caution. […] Incorrect implementation of liveness
> probes can lead to cascading failures. This results in restarting of container
> under high load; failed client requests as your application became less
> scalable; and increased workload on remaining pods due to some failed pods.

and, on the division of labour:

> The liveness probe passes when the app itself is healthy, but the readiness
> probe additionally checks that each required back-end service is available.

The documented pattern when one endpoint serves both is *the same low-cost
endpoint with a higher `failureThreshold` on liveness*, so a pod is observed
not-ready for a period before being hard-killed.

**What we do.** The server's liveness and readiness probes run the **identical**
command — `ferroehr healthcheck` — differing only in the timing block. If that
command consults the database (and readiness must, or readiness is meaningless),
then a database blip restarts every pod instead of removing them from the
Service. That is precisely the cascading failure the caution describes: the
CDR's dependency going soft would turn into a restart storm across the fleet.

**The architecture.**

| probe | question | may touch dependencies |
|---|---|---|
| startup | has the process finished booting? | no |
| liveness | is the process wedged beyond recovery? | **never** |
| readiness | can this pod serve a request right now? | **yes — that is its job** |

- liveness → a process-local check that cannot fail because Postgres is slow
- readiness → `/health/readiness`, which already reports dependency state
- startup → protects a cold start (migrations, template cache) from a liveness
  probe that would otherwise kill it mid-boot; the docs recommend it precisely
  when start time can exceed `initialDelaySeconds + failureThreshold ×
  periodSeconds`

The console gets the same treatment: its liveness must not fail because the CDR
is down, or a CDR outage becomes a console restart loop on top of it.

## 2. PodDisruptionBudget — we omit the documented recommendation

**The rule.** From
[Disruptions](https://kubernetes.io/docs/concepts/workloads/pods/disruptions/):

> It is recommended to set `AlwaysAllow` Unhealthy Pod Eviction Policy to your
> PodDisruptionBudgets to support eviction of misbehaving applications during a
> node drain. The default behavior is to wait for the application pods to become
> healthy before the drain can proceed.

**What we do.** Our PDB sets `minAvailable`/`maxUnavailable` and never sets
`unhealthyPodEvictionPolicy`. With the default policy plus our `maxUnavailable:
0` posture, a node drain waits for pods that are already unhealthy to become
healthy — which, if they are unhealthy *because* of the node being drained, is a
wait that never ends. A cluster operator sees a drain hang with no explanation.

**The architecture.** `unhealthyPodEvictionPolicy: AlwaysAllow` by default,
exposed as a value for the deployment that wants the stricter default, with the
trade stated where it is set.

## 3. Pod Security Standards — measure against `restricted`, not against taste

**The rule.** The [restricted
profile](https://kubernetes.io/docs/concepts/security/pod-security-standards/)
requires: `allowPrivilegeEscalation: false`; `runAsNonRoot: true` **explicitly
set**; `seccompProfile.type` `RuntimeDefault` or `Localhost` and never undefined;
`capabilities.drop` must include `ALL` with nothing added; and the volume set
restricted to `configMap`, `csi`, `downwardAPI`, `emptyDir`, `ephemeral`,
`persistentVolumeClaim`, `projected`, `secret`.

**What we do.** The server complies. What is missing is the *namespace* half:
the profile is enforced by Pod Security Admission via namespace labels
(`pod-security.kubernetes.io/enforce: restricted`), and a chart that produces
compliant pods into an unlabelled namespace is compliant by luck. Nothing in the
chart or its documentation tells an operator to label the namespace, and nothing
fails if they do not.

**The architecture.** The chart cannot label a namespace it does not own, so:
document the label as a prerequisite, render it in the docs' namespace example,
and add a chart-level guard that the rendered pods would pass `restricted` —
so the claim is machine-checked rather than asserted.

## 4. Labels — the standard set, and one immutability trap

**The rule.** Helm's [conventions](https://helm.sh/docs/chart_best_practices/)
and the Kubernetes [recommended
labels](https://kubernetes.io/docs/concepts/overview/working-with-objects/common-labels/)
define `app.kubernetes.io/{name,instance,version,component,part-of,managed-by}`.

**The trap, found while writing the console.** A Deployment's
`spec.selector.matchLabels` is **immutable**
([Deployment](https://kubernetes.io/docs/concepts/workloads/controllers/deployment/)).
Adding `component` to the existing `selectorLabels` helper — the natural way to
distinguish server pods from console pods — breaks `helm upgrade` on every
existing release with an error most operators will read as a chart bug.

**The architecture.** `component` goes on the **pod template and object labels
only**, never inside `selectorLabels`. New workloads get their own selector
built from name+instance+component *from the start*, so they are separable
without ever mutating the server's.

## 5. Configuration hygiene

From [Configuration Best
Practices](https://kubernetes.io/docs/concepts/configuration/overview/):

- *"Use the latest stable API version."* — audit every `apiVersion` we emit.
- *"Don't specify default values unnecessarily: simple, minimal configuration
  will make errors less likely."* — our templates set several fields to their
  API defaults, which is noise in every rendered manifest and in every golden
  diff.
- *"Group related objects into a single file"* — the console lands as one file
  rather than five, which this rewrite already does.
- *"`kubernetes.io/description`"* is called out as the most useful annotation
  and we use it nowhere.
- Booleans are `true`/`false` only, and anything Boolean-looking is quoted.

## 6. What the rewrite delivers

1. Probes split by role, liveness process-local, startup probe added.
2. `unhealthyPodEvictionPolicy: AlwaysAllow` with the trade documented.
3. A restricted-profile assertion over every rendered pod, in `validate.sh`.
4. `component` labels on objects and pod templates, never in a selector.
5. Default-valued fields removed from the templates.
6. `kubernetes.io/description` on the objects an operator meets first.
7. The console as a first-class optional workload (#2193), built to all of the
   above rather than retrofitted.

## 7. Explicitly out of scope

Anything that changes an existing Deployment's `selector` — see §4. If a future
change needs it, that is a major chart version and a documented
delete-and-reinstall, not a silent upgrade failure.
