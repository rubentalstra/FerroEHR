# Security & multi-tenancy

A clinical data repository holds PHI, so its access controls and audit trail
are part of the product, not an afterthought. This chapter covers the four
security surfaces you configure when you deploy FerroEHR: **authentication**
(who is calling), **authorization** (what they may do), **multi-tenancy**
(isolating independent logical systems), and the **ATNA audit trail**
(recording what happened). Each is independently configurable, and each is
described here in terms of the environment variables you actually set.

<!-- toc -->

Configuration follows the same pattern throughout: the server reads defaults,
then the single `ferroehr.toml` file, then environment variables, with `__`
separating nested keys. The security configuration groups live in
distinct sections of `ferroehr.toml` — `[auth]` (authentication), `[tenancy]`
(multi-tenancy), `[authz]` (authorization), and `[audit]`
(the ATNA audit trail) — and any key can be overridden with the matching
`FERROEHR_*` environment variable shown below.

## Authentication

Authentication is on by default (`FERROEHR__AUTH__ENABLED=true`). Setting
it to `false` lets all requests through unauthenticated — a development-only
mode.

There is no single "mode" switch. The server offers two mechanisms and enables
each by the presence of its configuration block:

- **HTTP Basic** is active when a `basic` block with a user list is
  configured. Each user has a username, an Argon2 password hash (a PHC string
  beginning `$argon2id$`), and a set of roles (default `["USER"]`). Because it
  is a list of users, the Basic block is normally supplied through the TOML
  configuration file rather than environment variables.
- **OAuth2/OIDC bearer tokens** are active when an `oidc` block is configured.
  The server validates the token's signature, issuer, and audience.

Both mechanisms are validated at startup, and a configuration the server cannot
honour **refuses to boot** rather than degrading at the first request:

| Configuration | Boot outcome |
|---|---|
| `auth.enabled = true` with no mechanism | error — a `401` challenge must name a scheme the server implements (RFC 9110 §11.6.1) |
| `[auth.oidc]` with no `audiences` | error — a server with no declared audience cannot reject another server's token (RFC 7519 §4.1.3) |
| `[auth.oidc] issuer` not `https`, or carrying a query/fragment | error — RFC 8414 §2, §6.2 (`allow_insecure_issuer = true` opts a dev issuer out of the scheme rule only) |
| `[auth.oidc] clock_skew_leeway_seconds` above `300` | error — leeway may be "no more than a few minutes" (RFC 9068 §4 step 6) |
| `[auth.oidc] hmac_secret` under 32 bytes | error — RFC 8725 §3.5 forbids memorizable passwords as keyed-MAC keys |
| `[auth.oidc] hmac_secret` set at all | boot **warning** — a symmetric key is a development posture (see below) |
| a `password_hash` below `m=19456,t=2,p=1` argon2id | error — the OWASP Argon2id floor |

Successfully verified Basic credentials are cached for
`FERROEHR__AUTH__VERIFIED_CACHE_TTL_SECONDS` (default `60`; `0` disables
the cache) so a busy client pays the deliberately-expensive Argon2
verification once per TTL instead of on every request. The cache stores only
a SHA-256 digest of the presented credential — never a plaintext password —
and an entry exists only after a successful verification; the TTL bounds how
long a revoked credential can still authenticate, exactly like a session
lifetime.

The OIDC settings:

| Environment variable | Default | Meaning |
|---|---|---|
| `FERROEHR__AUTH__OIDC__ISSUER` | — (required to enable OIDC) | expected `iss`, and the OIDC discovery base; an `https` URL with no query/fragment |
| `FERROEHR__AUTH__OIDC__AUDIENCES` | — (**required, non-empty**) | accepted `aud` values |
| `FERROEHR__AUTH__OIDC__ALGORITHMS` | `["RS256"]` | accepted signing algorithms |
| `FERROEHR__AUTH__OIDC__CLOCK_SKEW_LEEWAY_SECONDS` | `60` | leeway on `exp`/`nbf`; capped at `300` |
| `FERROEHR__AUTH__OIDC__ALLOW_INSECURE_ISSUER` | `false` | accept a non-`https` issuer (development/testing only) |
| `FERROEHR__AUTH__OIDC__HMAC_SECRET` | unset | an HS256 symmetric secret, min. 32 bytes (development/testing) |
| `FERROEHR__AUTH__OIDC__JWKS_JSON` | unset | a static JWKS document |

There is no separate JWKS or discovery URL to set: the server discovers the
JWKS URI from the issuer's `.well-known/openid-configuration` unless you supply
a static `JWKS_JSON` (preferred when present) or an `HMAC_SECRET`.

> [!TIP]
> **Keycloak example.** Point the issuer at your realm and let discovery do the
> rest:
>
> ```bash
> export FERROEHR__AUTH__OIDC__ISSUER=https://keycloak.example/realms/ferroehr
> export FERROEHR__AUTH__OIDC__AUDIENCES=ferroehr-api
> ```
>
> The same pattern works for Active Directory or any standards-compliant
> identity provider — walkthroughs for Entra ID and AD FS (and the answer for
> plain-LDAP directories) are in
> [Enterprise identity providers](identity-providers.md). Prefer
> JWKS/discovery over a shared HS256 secret in production. User accounts,
> roles, and lifecycle are administered in the IdP — the CDR has no user API.

An unauthenticated request to a protected route is refused with `401`; an
authenticated request that lacks the required role is refused with `403`. Two
outcomes are neither: a **malformed** `Authorization` header is a `400` (the
server never read a credential), and an **unreachable token issuer** is a `503`
with `Retry-After` (no token can be validated, so the server cannot decide — it
is not a statement about the caller's credential). The per-status table for
client authors is in [Using the API](using-the-api/index.md#which-status-a-credential-problem-gets).

### Two limits worth planning around

**A symmetric `hmac_secret` is a development posture, not a production one.**
The key is shared with the authorization server, so this CDR holds everything
needed to *mint* the tokens it accepts — an asymmetric key source never gives it
that power — and it cannot be rotated without a restart. The server logs a
warning at boot whenever one is configured. Use the issuer's OIDC discovery
document (the default when no static key material is set) or `jwks_json`.

**Revocation latency equals the access-token lifetime.** Tokens are validated
offline against the issuer's published keys; the CDR does not call an
introspection endpoint (RFC 7662 defines that mechanism but does not require a
resource server to use it), so a token revoked at the identity provider stays
acceptable here until its `exp` passes. This is deliberate: introspecting on
every request would put the identity provider's availability directly in the
request path, and caching introspection results only trades the lag for a
shorter one. **The control is therefore the token lifetime, which your
authorization server owns** — keep access-token lifetimes short (minutes, not
hours) if prompt revocation matters, and use refresh tokens for session length.
`clock_skew_leeway_seconds` (default `60`, capped at `300`) adds at most its own
value on top of `exp`.

## Authorization

Authorization has three composable layers. The per-EHR `EHR_ACCESS` gate is
the openEHR-specified base and is always on; the coarse role layer is active
when authentication is enabled; the fine-grained attribute layer is opt-in.
A request must clear every active layer. Deployments serving SMART apps can
enable a fourth, token-scope layer on top — see
[SMART App Launch](smart-app-launch.md).

### Per-EHR access control (`EHR_ACCESS`)

Every EHR carries a versioned `EHR_ACCESS` object — the openEHR
access-decision authority for that record. By default it has no settings and
the EHR is open to any authenticated caller (all existing workflows keep
working). Committing settings with the `ferroehr.access_control.v1` scheme
switches that EHR to explicit policy:

```json
{
  "_type": "EHR_ACCESS",
  "name": { "_type": "DV_TEXT", "value": "access" },
  "archetype_node_id": "openEHR-EHR-EHR_ACCESS.generic.v1",
  "settings": {
    "_type": "FERROEHR_ACCESS_CONTROL_V1",
    "gate_keeper": "user:alice",
    "default_access": "restricted",
    "access_list": [
      { "principal": "user:bob",   "access": "full" },
      { "principal": "role:nurse", "access": "restricted_below", "max_level": 2 }
    ],
    "privacy": {
      "default_level": 0,
      "composition_overrides": [
        { "uid": "8849182c-82ad-4088-a07f-48ead4180515", "level": 3 }
      ]
    }
  }
}
```

- **Access list** — with `default_access: "restricted"`, only listed
  principals may touch the EHR: `user:<login or OIDC subject>` or
  `role:<role>` (matched against the caller's roles). Everyone else gets
  `403`.
- **Privacy levels** — integer sensitivity levels with meanings you define
  for your jurisdiction. A composition's level is its override entry or the
  default; a caller with `restricted_below` access may only read
  compositions strictly below their `max_level`, while `full` access has no
  ceiling.
- **Gate-keeper** — once set, only that principal may commit a new
  `EHR_ACCESS` version (via a CONTRIBUTION; there is no dedicated
  `EHR_ACCESS` endpoint in the openEHR REST API). Changes are versioned and
  audited like all record content.

The scheme is a FerroEHR extension: openEHR mandates the `EHR_ACCESS`
object and its change control but publishes no concrete access-control
scheme. Query (AQL) results are not filtered by privacy level in this
release; the per-EHR gate still applies to EHR-scoped query routes.

### RBAC (role-based, coarse)

Every operation is classified as Public, Clinical, Management, or Admin, and a
role model gates each class. Roles are plain, case-insensitive strings; the
defaults are `USER` (the baseline clinical role) and `ADMIN`.

| Environment variable | Default | Meaning |
|---|---|---|
| `FERROEHR__AUTHZ__RBAC__ENABLED` | `true` | the coarse role gate (active only when auth is enabled) |
| `FERROEHR__AUTHZ__RBAC__ADMIN_ROLE` | `ADMIN` | role required for admin operations |
| `FERROEHR__AUTHZ__RBAC__USER_ROLE` | `USER` | the baseline clinical role |
| `FERROEHR__AUTHZ__RBAC__READONLY_ROLE` | `READONLY` | role marking a principal read-only: refused on every write |
| `FERROEHR__AUTHZ__RBAC__ROLE_CLAIMS` | `["roles","groups","entitlements","realm_access.roles"]` | JWT claim paths mined for roles |
| `FERROEHR__AUTHZ__RBAC__MANAGEMENT_ACCESS` | `admin_only` | management-surface access: `admin_only`, `private`, or `public` |

Roles come from the JWT claims listed in `ROLE_CLAIMS` — by default the carriers
[RFC 9068 §2.2.3.1](https://www.rfc-editor.org/rfc/rfc9068#section-2.2.3.1) names
for conveying authorization state (`roles`, `groups`, `entitlements`, of which
`roles` and `entitlements` are [SCIM](https://www.rfc-editor.org/rfc/rfc7643#section-4.1.2)
attributes), followed by the widely deployed nested `realm_access.roles` — or from
a Basic user's configured roles. A claim path may be dotted, so an issuer that
nests them differently is configuration rather than a code change. A clinical
operation needs at least one role; an admin operation needs the admin role; the
management surface follows its tri-state setting. Disabling RBAC restores
authentication-only behaviour.

> [!IMPORTANT]
> **The OAuth2 `scope` claim does not grant roles.** A scope grants a *client*
> delegated authority ([RFC 6749 §3.3](https://www.rfc-editor.org/rfc/rfc6749#section-3.3));
> it asserts nothing about the subject's roles. Reading it as one also made the
> at-least-one-role check pass for every OIDC token, since `openid` alone
> satisfied it. If your callers rely on a scope naming a role, move that role
> into one of the role claims above. Scopes remain on the principal and still
> drive SMART scope enforcement.

> [!NOTE]
> The downloadable Compose quickstart deliberately runs with RBAC **off** and a
> single user, so every surface works out of the box (see
> [Docker Compose](installation/compose.md)). Turn the role gate on with
> `[authz.rbac] enabled = true` (or `FERROEHR__AUTHZ__RBAC__ENABLED=true`) and
> give each principal an explicit `roles` list — a Basic user without one falls
> back to the baseline `USER` role, which grants no admin operation.

A principal carrying the `readonly_role` (default `READONLY`) is refused on
every write operation — creating an EHR, committing a composition, uploading a
template, and any update/delete — even when it also holds granting roles such
as `ADMIN` (a restriction always overrides a grant). Reads and AQL queries stay
permitted, so a `READONLY` account is an authenticated, view-only principal. The
repository's from-source development stack ships one such account
(`ferroehr-readonly`, password `ferroehr`) alongside `ferroehr` and
`ferroehr-admin`, with RBAC enabled, so the separation can be tried out; the
downloadable quickstart file has neither (one user, no role gate).

### ABAC (attribute-based, fine-grained)

For attribute-level decisions — "may this user touch this patient's data,
under this organisation, for this template?" — enable ABAC. A policy decision
point is consulted per clinical operation with resolved attributes. An
enabled ABAC block that cannot be built — a missing or invalid policy
directory, an unreachable-by-construction PDP client — aborts server startup:
a configuration that promises fine-grained authorization never silently runs
without it.

| Environment variable | Default | Meaning |
|---|---|---|
| `FERROEHR__AUTHZ__ABAC__ENABLED` | `false` | master ABAC switch |
| `FERROEHR__AUTHZ__ABAC__ENGINE` | `cedar` | `cedar` (embedded) or `remote` (external PDP) |
| `FERROEHR__AUTHZ__ABAC__ORGANIZATION_CLAIM` | `organization_id` | JWT claim for the organisation attribute |
| `FERROEHR__AUTHZ__ABAC__PATIENT_CLAIM` | `patient_id` | JWT claim for the patient attribute (enables the subject gate) |
| `FERROEHR__AUTHZ__ABAC__CEDAR__POLICY_DIR` | — (required for `cedar`) | directory of `.cedar` policy files |
| `FERROEHR__AUTHZ__ABAC__CEDAR__RELOAD_SECS` | off | optional policy hot-reload interval |
| `FERROEHR__AUTHZ__ABAC__REMOTE__SERVER` | — (required for `remote`) | PDP base URL (must end with `/`) |
| `FERROEHR__AUTHZ__ABAC__REMOTE__CONNECT_TIMEOUT_MS` | `2000` | PDP connect timeout |
| `FERROEHR__AUTHZ__ABAC__REMOTE__REQUEST_TIMEOUT_MS` | `5000` | PDP request timeout |

Two engines sit behind one interface. **Cedar** is the embedded default:
policies live in `.cedar` files, are schema-validated at boot (an invalid
policy set stops the server rather than silently denying), and need no
external service. The **remote PDP** option consults an external policy server
over HTTP for deployments that already run one.

A policy sees the **caller**, not just the request: the authenticated subject,
its roles (as the role layer above resolved them), its scopes, the resolved
organization and patient, the resource's patient and template, and the operation
id. So a rule can be written about one caller, a role, a scope, or a single
operation — the [shipped example policies](https://github.com/rubentalstra/FerroEHR/tree/develop/app/ferroehr-rest/examples/policies)
show a role-keyed break-glass permit and a scope-keyed write restriction.

> [!WARNING]
> Authorization is **fail-closed in two distinct senses**, and the difference
> shows up in the status code.
>
> Nothing permits by omission: a gate reached without an authenticated caller
> refuses, an unconfigured resource kind on the remote PDP denies (and the
> missing rule is a boot error), and Cedar is deny-by-default with `forbid`
> overriding `permit`. A denied decision is a **`403`**.
>
> And a stage that cannot **decide** is never read as a decision. An
> unreachable policy engine, a policy server answering `5xx`, or a policy that
> errors during evaluation is a **`500`** — never a silent permit, and never a
> `403`, which would claim a decision was made. (Cedar skips a policy that
> errors and reports it in its diagnostics; ignoring those would let an erroring
> `forbid` quietly stop forbidding.) A `4xx` from a remote PDP *is* a decision,
> so it denies.
>
> When a patient claim is configured, a local subject gate also rejects access to
> another patient's EHR before any policy call.

## Response security headers

Three surfaces, three honest answers — the set differs because what they serve
differs, not because one was forgotten.

**The REST API** carries, on every response including the transport-layer ones
(`413` from the body limit, `408` from the timeout, `500` from the panic handler):

| Header | Value | Why |
|---|---|---|
| `Cache-Control` | `no-store` | responses carry patient data; the OWASP cheat sheet names `no-store` for exactly that. It does not affect openEHR's `ETag`/`If-Match` concurrency control, which is a precondition mechanism, not a caching one |
| `X-Content-Type-Options` | `nosniff` | stops a proxy or browser re-guessing `application/json` as something executable |
| `Referrer-Policy` | `strict-origin-when-cross-origin` | request paths carry `ehr_id` and version identifiers; this keeps them out of cross-origin `Referer` headers |
| `Cross-Origin-Resource-Policy` | `same-site` | refuses cross-site embedding of API responses |
| `X-Frame-Options` | `DENY` | for the HTML this origin can serve (Swagger UI) |
| `Content-Security-Policy` | `default-src 'none'; frame-ancestors 'none'` | the defensive minimum. The cheat sheet is explicit that CSP "might be meaningless in the response of a REST API that returns content that is not going to be rendered", so this is not a policy pretending to govern scripts — it says nothing loads and nothing frames |

**`Strict-Transport-Security` is deliberately not sent by the API server.** It is a
property of the TLS edge, and [RFC 6797 §7.2](https://www.rfc-editor.org/rfc/rfc6797#section-7.2)
requires a browser to *ignore* it over plain HTTP — which is how this server is
commonly reached behind a terminating proxy. Set it at the proxy or ingress that
owns TLS; sending it from here would be inert at best and misleading at worst.

**The admin console** additionally carries the browser set with a real CSP, because
it serves HTML and hydrates WebAssembly.

**The published documentation site cannot carry response headers at all.** GitHub
Pages serves static files, so the policy travels as a `<meta http-equiv>` element,
which is weaker by specification: `frame-ancestors`, `report-uri` and `sandbox`
are ignored in meta form, and the policy applies only once the parser reaches it.
It still blocks an injected external script or an exfiltrating connection, which
is the realistic risk for a docs site. Anyone re-hosting these docs behind a real
web server should send the headers properly instead.

## Request limits and rate limiting

Two different protections, two different statuses, and an operator should be able
to tell them apart from the status alone.

**Body size** — `[server.limits]`. Two tiers: the clinical surface, and the routes
that accept bulk by design (template upload, `/message/import`, `/message/tdd`).
Over-limit is `413`. The defaults are sized to clear the largest operational
template in the vendored corpus several times over, and a deployment whose
compositions embed large `DV_MULTIMEDIA` data raises `body_bytes` deliberately.

**Request rate** — `[server.rate_limit]`, on by default. The **address** tier sits
outside authentication so a flood is refused before the server verifies a
signature per request; the **principal** tier sits inside it, keyed on the
authenticated subject, because a hospital behind one NAT is a single address and
address-keying a clinical API would throttle a whole site for one busy client.
Refusal is `429` with `Retry-After`.

**Concurrency** — `[server].max_in_flight`, the pre-existing load shed. Refusal is
`503` with `Retry-After`.

So: `503` means the server is full right now, `429` means you are asking too
fast, `413` means your payload is too big. Full key tables are on the
[configuration page](installation/configuration.md).

**If you benchmark this server, turn the rate limiter off first**, or you will
measure the limiter. Our own measurement lanes compose an overlay that disables
it, and both instruments refuse to write a record if the server answered any
`429` — a performance number that is really a configuration key is worse than no
number.

## Operational surfaces: what is reachable, and by whom

| Surface | Default | Notes |
|---|---|---|
| `/health`, `/health/liveness`, `/health/readiness` | **always on, unauthenticated** | Deliberate: orchestrator probes must not need credentials. The payload is a status per component plus a fixed-string detail — never a driver error, a DSN, or a panic payload. Causes are logged for the operator instead. |
| `/management/*` | **not mounted at all** (`management.enabled = false`) | With the master switch off, every route is `404`. |
| `/management/{info,metrics,prometheus,env,loggers,flamegraph}` | each **`off`** individually | Even with the master switch on, each endpoint stays unmounted until given a level. The global fallback is `admin_only`, so a level set carelessly lands on the admin gate rather than open. |
| `management.port` | unset (shares the API listener) | Set it to serve ops introspection from **its own listener**, which is how to get the port/interface separation the OWASP cheat sheet asks for — bind it away from the clinical interface. |

`env` and `flamegraph` deserve their individual defaults: `env` renders the
effective configuration (redacted, but still configuration), and `flamegraph`
starts a CPU profiler on request, which is both a disclosure and a
denial-of-service lever. Both are `off` by default and `admin_only` under the
global fallback; the profiler additionally caps the window and sampling
frequency, refusing an out-of-range request rather than clamping it silently.

## Secrets: mount files, never bake values

Every secret this server reads has a `*_file` sibling, and the loader reads and
trims the file at startup: `hmac_secret_file`, `jwks_json_file`,
`key_passphrase_file`, `client_secret_file`, and the TLS key/cert paths. Exactly
one of a pair may be set — both is a boot error.

That is deliberately the shape Docker Secrets and Kubernetes Secrets deliver: a
file mounted into the container. So the recommended posture needs no extra
machinery.

```yaml
services:
  ferroehr:
    environment:
      # Point the key at the mount path; the VALUE never appears anywhere.
      FERROEHR__AUTH__OIDC__JWKS_JSON_FILE: /run/secrets/oidc_jwks
    secrets:
      - oidc_jwks

secrets:
  oidc_jwks:
    file: ./secrets/oidc_jwks.json   # or `external: true` in swarm
```

Why files rather than environment variables, concretely: an environment variable
is readable from `/proc/<pid>/environ` by anything in the container's namespace,
appears in `docker inspect` output, is inherited by every child process, and is
routinely captured whole by crash reporters and process listings. A mounted file
is none of those things, and it can be rotated without recreating the container.

Two properties worth knowing because they are not obvious:

- **Redaction is a property of the type, not a list.** Secret-bearing fields are a
  `Secret`/`SecretUrl` newtype whose `Debug` and serialization render `***`, so a
  new secret key cannot be forgotten by a per-endpoint redactor — `/management/env`
  and `ferroehr config check` show `***` because the type does, not because
  something remembered to hide it.
- **A Kubernetes `Secret` is base64, not encryption.** The cheat sheet is blunt
  about this: Secrets are stored unencrypted in etcd by default. Enable
  encryption at rest or use an external manager; the chart mounts whatever you
  give it and cannot make an unencrypted store safe.

> [!WARNING]
> The downloadable quickstart carries an inline Argon2 hash for its throwaway
> `ferroehr` user, because a self-contained demo file cannot reference a secret you
> do not have. That is the one place a credential appears in our own artifacts, and
> it is a development credential by construction — replace it before any real use.

## Verifying what you pulled

Artifacts published from `3.17.4` onward carry **signed** build provenance and an
SBOM, so you can establish that a binary or image came from this repository's
build and see what went into it. Provenance was already being generated before —
BuildKit's SLSA statement on each image index — but unsigned, which means readable
and not verifiable. It is signed through Sigstore from this release on.

> [!IMPORTANT]
> Signing landed in the publishing lanes during the `3.17.4` cycle, so **`3.17.3`
> and every earlier tag carry no attestation**: the commands below answer
> `HTTP 404: Not Found` for them. That is the honest state, not a verification
> failure — there is nothing to verify, because those artifacts were built before
> the lane signed anything. The development images
> (`ghcr.io/rubentalstra/ferroehr:develop` and its two siblings) are signed and
> verify now; the release-tag, release-binary and chart forms below start
> answering at the `3.17.4` cut.

**An image** — signed and verifying today on the development tag:

```bash
gh attestation verify oci://ghcr.io/rubentalstra/ferroehr:develop -R rubentalstra/FerroEHR
```

Add `--signer-workflow rubentalstra/FerroEHR/.github/workflows/containers.yml` to
require the image lane specifically, not merely some workflow in this repository.
On a release, substitute the `vX.Y.Z` tag for `develop`.

**A release binary:**

```bash
gh attestation verify ferroehr-v3.17.4-x86_64-unknown-linux-gnu.tar.gz   -R rubentalstra/FerroEHR
```

**A release binary, without reaching GitHub.** Each release also carries its
Sigstore bundles as assets, so verification needs nothing but the artifact and
the bundle — useful on an air-gapped host, and the only form in which the
signature travels with the download:

```bash
gh attestation verify ferroehr-v3.17.4-x86_64-unknown-linux-gnu.tar.gz \
  --bundle ferroehr-v3.17.4-x86_64-unknown-linux-gnu.tar.gz.sigstore.json \
  --repo rubentalstra/FerroEHR
```

The `*.sbom.sigstore.json` asset beside it is the same thing for the SBOM
attestation, so "which dependency graph was this binary built from" is
verifiable offline too.

Each release also attaches a **CycloneDX SBOM of the Rust dependency graph**
(`*.cdx.json`), which is a different document from the SPDX SBOM on the image
index and answers a different question. The image SBOM sees the OS layer — which
is what matters for `ferroehr-postgres`, built on the upstream `postgres` image.
The CycloneDX one enumerates the cargo graph: every component with a
`pkg:cargo/…` purl and licence, most with checksums, and the **dependency edges**,
so "is this crate a direct dependency or something four levels down" is a question
the document can answer rather than a flat list you have to guess from.

### What SLSA level each artifact reaches, and what is still not claimed

The levels are [SLSA v1.0 Build levels](https://slsa.dev/spec/v1.0/levels), and
they differ per artifact, so the table is the honest form:

| artifact | level | why |
|---|---|---|
| release binaries + their SBOM | **Build L3** | built and attested inside a *reusable* workflow, so the signing material is out of reach of any caller-defined step |
| container images | Build L2 | attested in the job that builds them |
| the Helm chart | Build L2 | same |

Build L3's distinguishing requirement is that the platform must "prevent secret
material used to sign the provenance from being accessible to the user-defined
build steps". Every step of a GitHub Actions job shares one runner VM, so
attesting inside the building job cannot satisfy it. The release lane therefore
builds and signs inside `release-build.yml`, a reusable workflow: it runs on its
own VM and a caller passes declared inputs — it cannot add steps. The caller job
has no steps at all, which is what makes the property hard to lose by accident.

The consumer-visible benefit is that you can **require** that signer:

```bash
gh attestation verify ferroehr-v3.17.4-x86_64-unknown-linux-gnu.tar.gz \
  -R rubentalstra/FerroEHR \
  --signer-workflow rubentalstra/FerroEHR/.github/workflows/release-build.yml
```

Without that flag you are trusting that *some* workflow in this repository signed
the artifact. With it, you are trusting one specific hardened lane.

What is still **not** claimed, in either lane: the isolation is GitHub's rather
than ours, and nothing here asserts a reproducible or hermetic build — those are
separate SLSA tracks this project does not address. Naming the boundary is worth
more than rounding a level up.

Where a scanner reports a finding in an inherited upstream layer that we have
argued is not reachable, the argument is published as an
[OpenVEX](https://openvex.dev) document under `security/vex/` — with the
justification and an impact statement you can check, rather than an ignore entry
that records only the verdict.

## Multi-tenancy

Multi-tenancy lets one deployment host several isolated logical openEHR
systems, each with its own `system_id`. It is off by default; when off, the
server behaves byte-for-byte as a single-tenant system.

| Environment variable | Default | Meaning |
|---|---|---|
| `FERROEHR__TENANCY__ENABLED` | `false` | enable multi-tenancy |
| `FERROEHR__TENANCY__CLAIM` | `tenant` | the JWT claim (a dotted path) carrying the tenant key |
| `FERROEHR__TENANCY__HEADER` | unset | a development header override for the tenant |

A request's tenant is resolved from the configured JWT claim (a dotted path
such as `realm_access.tenant` is walked through nested objects). Isolation is
enforced in the database with **PostgreSQL row-level security**: the resolved
tenant scopes the connection so a query can only ever see its own tenant's
rows.

> [!WARNING]
> Leave `FERROEHR__TENANCY__HEADER` unset in production — a client-supplied
> header must never be able to select a tenant; the tenant must come from the
> authenticated token. Isolation is also fail-safe by design: an absent or
> unresolvable tenant runs unscoped against a reserved default rather than
> guessing, and a cross-tenant access surfaces as an empty result set, never a
> `403` that would leak the existence of another tenant's data.

## ATNA audit trail

Separately from openEHR's own provenance, FerroEHR keeps an IHE ATNA
security audit trail of API access — **on by default**, persisted in the
local Audit Record Repository (the dedicated `audit` PostgreSQL schema),
rendered in both official formats (FHIR R4B `AuditEvent` per IHE BALP, and
the DICOM PS3.15 audit message for the classic syslog feed), retrievable via
the RESTful-ATNA **ITI-81** FHIR search, and optionally forwarded to an
external ARR over syslog and/or the ITI-20 FHIR feed. Node authentication
(ITI-19) is available as native mutual TLS on the listener.

The full chapter — record content, sinks, the ITI-81 search, fail-mode
semantics, and mTLS — is **[Audit trail (IHE ATNA)](audit.md)**; every
`[audit]` key is in the
[configuration reference](installation/configuration.md#audit).

> [!NOTE]
> The ATNA trail is orthogonal to openEHR's own `CONTRIBUTION` and
> `AUDIT_DETAILS`, which the server always writes in the same transaction as
> every change. openEHR audit records what a version says about its own
> authorship; ATNA records security surveillance of API access. Both coexist.
> Identified data never enters telemetry (metrics, traces, logs) — see
> [Operations](operations.md) — so the audit trail is the single place where
> access to identified data is recorded.
