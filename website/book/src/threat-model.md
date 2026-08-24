# Threat model

This chapter says what FerroEHR defends against, how, and (more usefully)
**what remains true after the control has done its job**. It exists because the
parts of this system that actually protect patient data are the parts no
external specification governs: openEHR's REST specification makes
authentication a `SHOULD` and mandates no scheme, and the Service Model places
authorization out of band, so only the 401-versus-403 split is
specification-grounded. Everything finer is this project's own design, and a
design nobody has written down asks every deployer to reconstruct it privately.

<!-- toc -->

## What this is, and what it is not

This is an **attack-surface and trust-boundary analysis with named residual
risk**. Boundaries are enumerated, the control at each is stated, and the risk
that survives the control is stated beside it.

It is **not** a certification, an audit report, or an assurance case in the
formal sense, and it does not make FerroEHR compliant with anything. It is also
not a substitute for your own analysis: your deployment introduces boundaries
this document cannot see: your identity provider's token policy, your
network, your operators, your backups. The value here is that you do not have
to reverse-engineer *our* half of it from configuration prose.

Read it alongside [Security & multi-tenancy](security.md) (how each control is
configured), [Audit trail](audit.md) (what is recorded),
[Verifying releases](verifying-releases.md) (establishing what you are running),
[Operations](operations.md) (the deployment surface) and
[Cluster hardening](installation/kubernetes-hardening.md) (what the platform
owes).

**Where this document and the code disagree, the code is right and this
document is a defect.** Report it.

## Assets

What an attacker wants, in the order the loss hurts:

| Asset | Where it lives | Why it matters |
|---|---|---|
| **Clinical payload:** compositions, EHR status, folders, demographics | the `node` and `vo_version` tables, and the cold-archive mirrors | this is PHI; disclosure is the primary harm and it is not undoable |
| **The audit trail** | the `audit` schema, plus any configured forwarding sink | it is the evidence that everything else happened; an attacker who can edit it can make an access disappear |
| **Version history and its integrity** | `vo_version`, `contribution`, attestations | an openEHR record's value is that it is *append-only and attributable*; a silently rewritten prior version is worse than a deleted one |
| **Signing keys** | the configured signing key material for commit attestation | forging an attestation forges provenance of clinical content |
| **Bearer credentials and password hashes** | tokens in flight; Argon2id PHC hashes at rest | a stolen token is an authenticated clinical caller until it expires |
| **Tenant separation** | the `ferroehr.tenant_id` session setting and the row-level-security policies | in a multi-tenant deployment, one tenant reading another is a breach of two organisations at once |
| **Availability** | the whole stack | a CDR that is down is a clinical system that is down; denial of service is a patient-safety issue here, not an inconvenience |

## Actors

| Actor | Reaches | Assumed capability |
|---|---|---|
| **Unauthenticated caller** | the listening port | can send arbitrary bytes, replay anything observed, and probe every route |
| **Authenticated clinical caller** | the openEHR API within their grants | holds a valid token; may be a legitimate user acting outside their remit, or a compromised client |
| **Admin / management caller** | the admin, management and messaging surfaces | can delete physically, archive, dump and load; the most dangerous *authorized* actor |
| **Tenant peer** | their own tenant's API | a legitimate tenant of a shared deployment, trying to reach another's rows |
| **Database** | everything stored | trusted for confidentiality, but assumed to be a place where a mistake is permanent |
| **Terminology server (FHIR R4B)** | called out to at validation and commit time | operator-configured; semi-trusted: it answers our questions and can lie |
| **Object store (S3)** | called out to for multimedia | operator-configured; semi-trusted, and its responses are parsed |
| **Message broker (AMQP)** | receives change events | operator-configured; a sink for data, so a confidentiality boundary |
| **Identity provider** | issues the tokens everything else believes | fully trusted by construction; if it is compromised, no control below it holds |
| **Cluster / host operator** | the process, its memory, its filesystem, its secrets | fully trusted; outside the software's reach |

## Trust boundaries

```mermaid
flowchart LR
  subgraph Untrusted
    U[Unauthenticated caller]
    C[Clinical client]
    A[Admin client]
  end
  subgraph Deployment["Deployment boundary (operator-controlled)"]
    subgraph Server["FerroEHR process"]
      B1["B1 network + protocol"]
      B2["B2 authentication"]
      B3["B3 authorization<br/>EHR_ACCESS - RBAC - ABAC - SMART"]
      B4["B4 content validation"]
      B5["B5 tenant scoping"]
    end
    DB[("B6 PostgreSQL")]
    AUD[("B7 audit store")]
  end
  subgraph External["Semi-trusted peers"]
    IDP[Identity provider]
    TS[Terminology server]
    S3[Object store]
    MQ[Message broker]
  end
  U --> B1
  C --> B1
  A --> B1
  B1 --> B2
  B2 --> B3
  B3 --> B4
  B4 --> B5
  B5 --> DB
  B5 --> AUD
  B2 -.verifies tokens from.-> IDP
  B4 -.asks.-> TS
  B4 -.stores blobs.-> S3
  B5 -.publishes.-> MQ
```

Each boundary below names the control that holds and the risk that remains.

### B1 — Network and protocol

**Control.** TLS terminates either at the server (`rustls`, TLS 1.3 by default)
or in front of it. Connection-level bounds apply before a request even exists (an
HTTP/1 header-read timeout, an HTTP/2 stream cap with keep-alive PINGs); request
bodies are size-limited, requests are timed out, concurrency is capped by an
admission limit, and a two-tier `tower_governor` rate limit is **on by
default**. Panics are caught and become a clean `500` rather than a dropped
connection, and release builds run with overflow checks on so an arithmetic
mistake is a loud panic rather than a silently wrong number. Response security
headers are set. Every parser that reads attacker-controlled bytes (canonical
JSON, canonical XML, AQL, ADL, OPT 1.4 templates, the simplified formats, and
the identifier types) has a libFuzzer harness, built on the pull-request path
and fuzzed on a nightly campaign, and a crash is fixed in the crate, never in
the harness.

**Residual risk.**

- **Fuzzing finds crashes; it does not prove their absence.** The harnesses
  cover the parsers we identified as reachable. A parser reached by a path we
  did not enumerate is not covered by anything.
- **Rate limiting is per-process.** A multi-replica deployment behind a load
  balancer divides the effective limit by the replica count; it is not a
  distributed limiter. The shipped defaults are also deliberately generous:
  they sit above this implementation's own measured whole-server ceiling, so the
  limiter refuses abuse rather than shaping normal load.
- **Algorithmic complexity in query execution is not bounded by the limiter.**
  A syntactically small AQL query can be an expensive one. Statement timeouts at
  the database are the backstop, and they are the operator's to set.
- **TLS termination in front of the server is the operator's**; nothing in the
  software can tell whether the hop behind the proxy is encrypted.

### B2 — Authentication

**Control.** Two mechanisms: HTTP Basic against Argon2id PHC hashes with a
verified-credential cache, and OAuth2/OIDC bearer tokens validated against the
issuer. Configuration is validated **at boot**, not on the first request: a
mandatory audience list, an `https` issuer, an algorithm set bound to its key
source (with `none` refused outright and two competing key sources refused as a
contradiction), an HMAC entropy floor, and the OWASP Argon2id parameter floor. A
malformed `Authorization` header is a `400`, not a `401`: the server never read
a credential. An unreachable issuer is a `503`, never a silent pass. A `401` body
names no reason, so it is not an oracle for expired-versus-forged.

**Residual risk.**

- **A valid token is a valid caller.** FerroEHR verifies signature, audience,
  issuer and expiry; it does not and cannot know whether the human behind the
  token is who the token says. Token theft is defended against by your identity
  provider's lifetimes and binding, not by us.
- **There is no token binding, no proof-of-possession and no replay window.** A
  bearer token replayed from another network location is accepted.
- **The identity provider is unconditionally trusted.** A compromised issuer,
  or an attacker who can mint tokens with the right audience, defeats every
  authorization stage below, all of which read their inputs from claims.
- **Basic authentication has no session, no lockout and no second factor.** It
  is intended for machine callers and small deployments; the Argon2id cost is
  the only brake on offline cracking if a hash leaks.

### B3 — Authorization

**Control.** Five stages, each of which can only *narrow* what the previous one
allowed, which is what makes an optional stage safe to leave off:

1. **Authentication** produces a principal or a typed refusal.
2. **`EHR_ACCESS`** is always on and unconditional, from the openEHR Reference
   Model's own gateway clause.
3. **RBAC** checks the coarse operation class (public, clinical, or admin)
   against roles taken from the RFC 9068 claim carriers. An OAuth2
   scope is deliberately not treated as a role. The read-only restriction
   overrides any grant. (The `/management/*` surface is *not* in this
   classification: it is governed endpoint-by-endpoint by its own configuration,
   and an endpoint you do not name is not mounted at all.)
4. **ABAC** (off by default) evaluates subject and resource attributes through
   embedded Cedar or an external policy decision point, fanning multi-valued
   attributes out as a cartesian product with all-must-permit and short-circuit
   deny.
5. **SMART scopes** (off by default) are AND-composed onto the ABAC decision.

Deny-by-default throughout, and **fail-closed in both senses**: no stage permits
by omission (an unconfigured resource kind on the external PDP denies, and the
missing rule is a boot error), and a stage that cannot *decide* is never read as
a decision: an unreachable issuer is `503`, a policy server answering `5xx` or a
Cedar policy that errors during evaluation is a fail-closed `500`. Silence is
never consent, and a broken control never looks like a policy outcome.

**Residual risk.**

- **Coarse by default.** With ABAC off (the default) authorization is
  role-level and `EHR_ACCESS`-level. Any authenticated clinical caller can reach
  any EHR that `EHR_ACCESS` does not restrict. If your model is "a clinician may
  read only their own patients", that is ABAC, and you must turn it on and write
  the policy.
- **Policy is only as good as the attributes.** Every decision is made from
  claims the identity provider asserted. A wrong `organization` claim produces a
  correct decision on a false premise.
- **The external PDP is a network dependency in the request path.** Fail-closed
  means an outage is an outage: correct, and an availability risk you should
  size.
- **Authorization is enforced at the API.** Anything with a direct database
  connection is past it entirely; see B6.
- **A legitimate caller acting outside their remit is not prevented, only
  recorded.** That is what the audit trail is for, and detection is not
  prevention.

### B4 — Content validation and outbound calls

**Control.** Committed content is validated against the operational template
(the WebTemplate walker), the Reference Model's own invariants (machine-derived
from the specification's meta-model rather than hand-written) and terminology
bindings. Canonical JSON is read by a **strict** reader that refuses undeclared
and duplicate keys. Outbound terminology, object-store and broker calls go only
to operator-configured endpoints.

**Residual risk.**

- **The terminology server is trusted to answer honestly.** A compromised or
  misconfigured one can permit a code that should have been refused. It is a
  correctness boundary, and a clinical-safety one.
- **A response parsed from a semi-trusted peer is still parsed.** A terminology
  answer, an object-store response and a broker message are all parsed by the
  server, so a compromised endpoint reaches a parser rather than only a
  validator. The object-store path once carried an XML parser version with
  denial-of-service advisories the client-facing canonical-XML codec did not
  share; both are now on the patched version, and the only pre-fix copy left in
  the build reaches it through the flamegraph renderer, which writes SVG and
  parses nothing. That argument is published as a machine-readable
  [VEX document](https://github.com/rubentalstra/FerroEHR/tree/develop/security/vex)
  rather than left in a comment. Read the current documents for the current
  position: they name the versions, and they are regenerated from the gate.
- **Multimedia blobs are stored, not inspected.** FerroEHR is not an antivirus
  and does not pretend to be; a malicious blob is delivered faithfully to
  whatever opens it.
- **Validation does not make content true.** A well-formed composition asserting
  a clinically wrong fact is stored, correctly.

### B5 — Tenant scoping

**Control.** Off by default; when off the server is byte-for-byte
single-tenant. When on, the tenant is resolved from a configured JWT claim and
applied as a PostgreSQL session setting that drives **`FORCE ROW LEVEL
SECURITY`** policies on every tenant-scoped table, including the cold-archive
mirrors. Isolation is fail-safe: an absent or unresolvable tenant runs against a
reserved default rather than guessing, and a cross-tenant access is an empty
result set rather than a `403` that would leak the existence of another tenant's
data.

A tenant *registry* that cannot be reached is a separate case and is not read as
"no tenant": it answers `503`, like any other dependency failure, rather than
falling through to the default.

**Residual risk.**

- **`FERROEHR__TENANCY__HEADER` is a tenant selector a client controls**, and
  when set it **wins over the JWT claim**. It is a development affordance and
  there is no way to make it safe in production. If it is set, tenancy is not a
  boundary. Leave it unset.
- **A missing or unknown claim degrades quietly to the default tenant.** That is
  the fail-safe choice, and its cost is that a misconfigured claim path looks
  like "tenancy is doing nothing" rather than failing loudly.
- **Row-level security binds a connection, not a request.** The scoping is
  applied per connection from a shared pool; a defect in that plumbing is a
  cross-tenant read, which is why it is enforced by the database rather than by
  application `WHERE` clauses: the database refuses even if the query forgets.
- **Tenancy separates rows, not resources.** One tenant's expensive query
  competes with another's for the same CPU, connections and disk. It is not a
  noisy-neighbour control.

### B6 — The database

**Control.** The application connects with a least-privilege role; migrations
are applied by a separate, more privileged one. `FORCE ROW LEVEL SECURITY`
means even a table owner is subject to the policies. Version history is
temporal (`PRIMARY KEY … WITHOUT OVERLAPS`) rather than overwritten, so a prior
version is a row that exists, not one that was replaced. Every write emits a
contribution and an audit record in the same transaction.

**Residual risk.**

- **Anyone with a database connection is past every control in B1–B5.** That is
  the single largest residual risk in this system, and it is inherent: FerroEHR
  enforces authorization at the API. Protect the credentials and the network
  path accordingly.
- **Encryption at rest is the operator's.** FerroEHR does not encrypt clinical
  payload before it reaches PostgreSQL; the payload is queryable JSONB, which is
  what makes AQL possible and is incompatible with application-level encryption
  of the same fields.
- **A superuser can rewrite history.** The temporal design makes accidental
  overwrite structurally hard; it does not defend against someone with `UPDATE`
  on the tables. The audit chain (B7) is the detection layer.
- **Backups carry PHI with none of these controls.** A restored dump is a
  complete copy of everything.

### B7 — The audit trail

**Control.** Every access is recorded in both official renderings (DICOM PS3.15
and FHIR R4 `AuditEvent`/BALP) into a local Audit Record Repository, on by
default, with optional syslog and ATX:FHIR-Feed forwarding. Refusals (`401`,
`403`, and the `400` a malformed credential header earns) are always recorded,
and an unattributable denial is recorded as unattributed rather than under a
fabricated subject. Records are **hash-chained**, so
`SELECT * FROM audit.verify_audit_chain()` names any record that was altered
or removed. ITI-81 is the retrieval side; ITI-19 mutual TLS is available for
node authentication.

**Residual risk.**

- **Tamper *evidence*, not tamper *proof*.** The chain proves alteration
  happened; it does not prevent it. An attacker with write access to the audit
  schema can rewrite the chain from the point of compromise forward, which is
  precisely why forwarding to an off-box sink matters, and why the sink should
  not be writable by the same identity.
- **A local repository shares the blast radius of the thing it audits.** If the
  database is lost, so is the local audit trail.
- **The trail records what the server saw.** A read performed directly against
  the database appears nowhere in it.
- **Audit records are themselves sensitive.** They name patients, subjects and
  actions; ITI-81 retrieval is a PHI-disclosure surface and is authorized like
  any other.

### B8 — The supply chain

**Control.** Releases are built by a reusable workflow that satisfies SLSA v1.0
Build L3, signed through Sigstore with a signer identity a consumer can pin,
carrying a CycloneDX dependency SBOM, a SPDX repository SBOM, in-toto
provenance and a plain `sha256sum`; the verification procedure is
[Verifying releases](verifying-releases.md). The release is created as a draft
and only published once its full asset set is present, because a published
release is immutable. Container base images are digest-pinned, and the published
images are re-scanned weekly at the tag an operator actually pulls, with a
fixable high-or-critical finding both failing the run and filing an issue. Every
action is pinned to a commit SHA, workflows are audited by `zizmor` on every pull
request, and accepted advisories carry published VEX justifications. Commits and
tags are signed and the tags are protected by a ruleset.

**Residual risk.**

- **Provenance proves *where* an artifact was built, not that the source was
  good.** Build L3 says these bytes came from this workflow at this commit. It
  says nothing about what that commit contained.
- **The build is not hermetic and not reproducible.** Those are separate SLSA
  tracks this project does not address. A dependency resolved at build time is
  trusted to be what the lock file names.
- **Review capacity is one person.** No second human is structurally required
  between an idea and a released binary; the mitigation is machine enforcement
  and it is honestly recorded in
  [GOVERNANCE.md](https://github.com/rubentalstra/FerroEHR/blob/develop/GOVERNANCE.md).
- **Only the newest release receives fixes.** There is no maintenance branch to
  backport to; see [SECURITY.md](https://github.com/rubentalstra/FerroEHR/blob/develop/SECURITY.md).

## What is explicitly NOT defended against

Silence is never coverage, so these are stated rather than left to inference.

- **A compromised identity provider.** Everything downstream reads claims it
  asserted.
- **A compromised host, container runtime, or cluster operator.** Process memory
  holds decrypted PHI and key material by necessity.
- **Anyone with direct database credentials.**
- **A malicious maintainer, or a compromise of the maintainer's account or
  signing key.** The bus factor is one; see
  [MAINTAINERS.md](https://github.com/rubentalstra/FerroEHR/blob/develop/MAINTAINERS.md).
- **Physical access, side channels, and speculative-execution attacks.**
- **Traffic analysis.** Request sizes, timings and error-code patterns can leak
  the existence of records; nothing pads or delays.
- **A legitimate user's authorized misuse**, beyond recording it.
- **Denial of service by a resource-holding authorized caller:** an expensive
  query, a large commit, a slow client. The rate limiter is on but coarse and
  per-process; database statement timeouts and container limits are the
  operator's.
- **Data remanence.** Physical deletion removes rows; it does not scrub
  filesystem blocks, WAL segments, replicas or backups.
- **Clinical correctness.** FerroEHR validates structure, invariants and
  terminology bindings. It does not know whether a recorded fact is true, and
  no control here is a substitute for clinical governance.

## Reporting a finding

If you believe a control is weaker than stated here, or a residual risk is
missing, that is a valid security report, including when the defect is in this
document rather than in the code. Follow
[SECURITY.md](https://github.com/rubentalstra/FerroEHR/blob/develop/SECURITY.md):
report privately, never as a public issue.
