# Security & multi-tenancy

A clinical data repository holds PHI, so its access controls and audit trail
are part of the product, not an afterthought. This chapter covers the four
security surfaces you configure when you deploy EHRbase-rs: **authentication**
(who is calling), **authorization** (what they may do), **multi-tenancy**
(isolating independent logical systems), and the **ATNA audit trail**
(recording what happened). Each is independently configurable, and each is
described here in terms of the environment variables you actually set.

<!-- toc -->

Configuration follows the same pattern throughout: the server reads defaults,
then an optional TOML file, then environment variables, with `__` separating
nested keys. The three security configuration groups use distinct prefixes —
`EHRBASE_REST_` (authentication and tenancy), `EHRBASE_AUTHZ_` (authorization),
and `EHRBASE_ATNA_` (audit) — and each also accepts a TOML file path
(`EHRBASE_REST_CONFIG`, `EHRBASE_AUTHZ_CONFIG`, `EHRBASE_ATNA_CONFIG`).

## Authentication

Authentication is on by default (`EHRBASE_REST_AUTH__ENABLED=true`). Setting
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
  The server validates the token's signature, issuer, and (optionally)
  audience.

Successfully verified Basic credentials are cached for
`EHRBASE_REST_AUTH__VERIFIED_CACHE_TTL_SECONDS` (default `60`; `0` disables
the cache) so a busy client pays the deliberately-expensive Argon2
verification once per TTL instead of on every request. The cache stores only
a SHA-256 digest of the presented credential — never a plaintext password —
and an entry exists only after a successful verification; the TTL bounds how
long a revoked credential can still authenticate, exactly like a session
lifetime.

The OIDC settings:

| Environment variable | Default | Meaning |
|---|---|---|
| `EHRBASE_REST_AUTH__OIDC__ISSUER` | — (required to enable OIDC) | expected `iss`, and the OIDC discovery base |
| `EHRBASE_REST_AUTH__OIDC__AUDIENCES` | empty (not checked) | accepted `aud` values |
| `EHRBASE_REST_AUTH__OIDC__ALGORITHMS` | `["RS256"]` | accepted signing algorithms |
| `EHRBASE_REST_AUTH__OIDC__HMAC_SECRET` | unset | an HS256 symmetric secret (development/testing) |
| `EHRBASE_REST_AUTH__OIDC__JWKS_JSON` | unset | a static JWKS document |

There is no separate JWKS or discovery URL to set: the server discovers the
JWKS URI from the issuer's `.well-known/openid-configuration` unless you supply
a static `JWKS_JSON` (preferred when present) or an `HMAC_SECRET`.

> [!TIP]
> **Keycloak example.** Point the issuer at your realm and let discovery do the
> rest:
>
> ```bash
> export EHRBASE_REST_AUTH__OIDC__ISSUER=https://keycloak.example/realms/ehrbase
> export EHRBASE_REST_AUTH__OIDC__AUDIENCES=ehrbase-api
> ```
>
> The same pattern works for Active Directory or any standards-compliant
> identity provider. Prefer JWKS/discovery over a shared HS256 secret in
> production.

An unauthenticated request to a protected route is refused with `401`; an
authenticated request that lacks the required role is refused with `403`.

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
working). Committing settings with the `ehrbase.access_control.v1` scheme
switches that EHR to explicit policy:

```json
{
  "_type": "EHR_ACCESS",
  "name": { "_type": "DV_TEXT", "value": "access" },
  "archetype_node_id": "openEHR-EHR-EHR_ACCESS.generic.v1",
  "settings": {
    "_type": "EHRBASE_ACCESS_CONTROL_V1",
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

The scheme is an EHRbase-rs extension: openEHR mandates the `EHR_ACCESS`
object and its change control but publishes no concrete access-control
scheme. Query (AQL) results are not filtered by privacy level in this
release; the per-EHR gate still applies to EHR-scoped query routes.

### RBAC (role-based, coarse)

Every operation is classified as Public, Clinical, Management, or Admin, and a
role model gates each class. Roles are plain, case-insensitive strings; the
defaults are `USER` (the baseline clinical role) and `ADMIN`.

| Environment variable | Default | Meaning |
|---|---|---|
| `EHRBASE_AUTHZ_RBAC__ENABLED` | `true` | the coarse role gate (active only when auth is enabled) |
| `EHRBASE_AUTHZ_RBAC__ADMIN_ROLE` | `ADMIN` | role required for admin operations |
| `EHRBASE_AUTHZ_RBAC__USER_ROLE` | `USER` | the baseline clinical role |
| `EHRBASE_AUTHZ_RBAC__ROLE_CLAIMS` | `["realm_access.roles","scope"]` | JWT claim paths mined for roles |
| `EHRBASE_AUTHZ_RBAC__MANAGEMENT_ACCESS` | `admin_only` | management-surface access: `admin_only`, `private`, or `public` |

Roles come from the JWT claims listed in `ROLE_CLAIMS` — by default the
Keycloak `realm_access.roles` array plus the space-separated `scope` claim —
or from a Basic user's configured roles. A clinical operation needs at least one
role; an admin operation needs the admin role; the management surface follows
its tri-state setting. Disabling RBAC restores authentication-only behaviour.

### ABAC (attribute-based, fine-grained)

For attribute-level decisions — "may this user touch this patient's data,
under this organisation, for this template?" — enable ABAC. A policy decision
point is consulted per clinical operation with resolved attributes.

| Environment variable | Default | Meaning |
|---|---|---|
| `EHRBASE_AUTHZ_ABAC__ENABLED` | `false` | master ABAC switch |
| `EHRBASE_AUTHZ_ABAC__ENGINE` | `cedar` | `cedar` (embedded) or `remote` (external PDP) |
| `EHRBASE_AUTHZ_ABAC__ORGANIZATION_CLAIM` | `organization_id` | JWT claim for the organisation attribute |
| `EHRBASE_AUTHZ_ABAC__PATIENT_CLAIM` | `patient_id` | JWT claim for the patient attribute (enables the subject gate) |
| `EHRBASE_AUTHZ_ABAC__CEDAR__POLICY_DIR` | — (required for `cedar`) | directory of `.cedar` policy files |
| `EHRBASE_AUTHZ_ABAC__CEDAR__RELOAD_SECS` | off | optional policy hot-reload interval |
| `EHRBASE_AUTHZ_ABAC__REMOTE__SERVER` | — (required for `remote`) | PDP base URL (must end with `/`) |
| `EHRBASE_AUTHZ_ABAC__REMOTE__CONNECT_TIMEOUT_MS` | `2000` | PDP connect timeout |
| `EHRBASE_AUTHZ_ABAC__REMOTE__REQUEST_TIMEOUT_MS` | `5000` | PDP request timeout |

Two engines sit behind one interface. **Cedar** is the embedded default:
policies live in `.cedar` files, are schema-validated at boot (an invalid
policy set stops the server rather than silently denying), and need no
external service. The **remote PDP** option consults an external policy server
over HTTP for deployments that already run one.

> [!WARNING]
> Authorization is **fail-closed**: if the policy engine is unreachable or a
> policy cannot be evaluated, the request is refused (mapped to `500`), never
> permitted. When a patient claim is configured, a local subject gate also
> rejects access to another patient's EHR before any policy call. A denied
> decision is a `403`.

## Multi-tenancy

Multi-tenancy lets one deployment host several isolated logical openEHR
systems, each with its own `system_id`. It is off by default; when off, the
server behaves byte-for-byte as a single-tenant system.

| Environment variable | Default | Meaning |
|---|---|---|
| `EHRBASE_REST_TENANCY__ENABLED` | `false` | enable multi-tenancy |
| `EHRBASE_REST_TENANCY__CLAIM` | `tenant` | the JWT claim (a dotted path) carrying the tenant key |
| `EHRBASE_REST_TENANCY__HEADER` | unset | a development header override for the tenant |

A request's tenant is resolved from the configured JWT claim (a dotted path
such as `realm_access.tenant` is walked through nested objects). Isolation is
enforced in the database with **PostgreSQL row-level security**: the resolved
tenant scopes the connection so a query can only ever see its own tenant's
rows.

> [!WARNING]
> Leave `EHRBASE_REST_TENANCY__HEADER` unset in production — a client-supplied
> header must never be able to select a tenant; the tenant must come from the
> authenticated token. Isolation is also fail-safe by design: an absent or
> unresolvable tenant runs unscoped against a reserved default rather than
> guessing, and a cross-tenant access surfaces as an empty result set, never a
> `403` that would leak the existence of another tenant's data.

## ATNA audit trail

Separately from openEHR's own provenance, EHRbase-rs can emit an IHE ATNA
security audit trail: one DICOM Audit Message (DICOM PS3.15 §A.5) per audited
operation, describing _who_ did _what_ to _which_ resource, with _what
outcome_, from _where_, and _when_. Records are shipped to an Audit Record
Repository over syslog (RFC 5424 framing), transported over UDP (RFC 5426) or
TLS (RFC 5425). Every server operation is audited, and authentication failures
(`401`/`403`) are always recorded. The audited operations include the
native-API EHR-Extract export and import (recorded with the ATNA `Extract`
object class), so moving a patient's record between systems leaves an audit
trail alongside the REST activity.

| Environment variable | Default | Meaning |
|---|---|---|
| `EHRBASE_ATNA_ENABLED` | `false` | master switch |
| `EHRBASE_ATNA_REPOSITORY_HOST` | `localhost` | audit repository host |
| `EHRBASE_ATNA_REPOSITORY_PORT` | `514` | audit repository port |
| `EHRBASE_ATNA_TRANSPORT` | `udp` | `udp` or `tls` |
| `EHRBASE_ATNA_ENTERPRISE_SITE_ID` | unset | enterprise/site identifier |
| `EHRBASE_ATNA_SOURCE_ID` | `ehrbase` | audit source identifier |
| `EHRBASE_ATNA_VALUE_IF_MISSING` | `UNKNOWN` | fill for an empty mandatory field |
| `EHRBASE_ATNA_SUPPRESS_LOGIN_EVENTS` | `true` | skip the successful-login records |
| `EHRBASE_ATNA_FAIL_MODE` | `open` | `open` (drop and continue) or `closed` (reject auditable ops if undeliverable) |
| `EHRBASE_ATNA_RESOLVE_SUBJECT` | `false` | enrich the patient identifier from stored data |
| `EHRBASE_ATNA_QUEUE_CAPACITY` | `1024` | bounded in-memory audit queue |
| `EHRBASE_ATNA_SERVER_HOST` | unset | this node's advertised network address |
| `EHRBASE_ATNA_TLS_CA_PATH` | unset | PEM of the repository CA (TLS) |
| `EHRBASE_ATNA_TLS_IDENTITY_CERT_PATH` | unset | client certificate PEM (mutual TLS) |
| `EHRBASE_ATNA_TLS_IDENTITY_KEY_PATH` | unset | client key PEM (mutual TLS) |

For PHI-adjacent audit, use `EHRBASE_ATNA_TRANSPORT=tls` with a CA (and, where
the repository requires it, a client certificate and key for mutual TLS). The
`fail_mode` choice is a policy decision: `open` never blocks a clinical
request when the repository is down (records are dropped and metered), while
`closed` refuses auditable operations with `503` rather than proceed
unaudited.

> [!NOTE]
> The ATNA trail is orthogonal to openEHR's own `CONTRIBUTION` and
> `AUDIT_DETAILS`, which the server always writes in the same transaction as
> every change. openEHR audit records what a version says about its own
> authorship; ATNA records security surveillance of API access. Both coexist.
> Identified data never enters telemetry (metrics, traces, logs) — see
> [Operations](operations.md) — so the audit trail is the single place where
> access to identified data is recorded.
