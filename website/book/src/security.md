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
  The server validates the token's signature, issuer, and (optionally)
  audience.

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
| `FERROEHR__AUTH__OIDC__ISSUER` | — (required to enable OIDC) | expected `iss`, and the OIDC discovery base |
| `FERROEHR__AUTH__OIDC__AUDIENCES` | empty (not checked) | accepted `aud` values |
| `FERROEHR__AUTH__OIDC__ALGORITHMS` | `["RS256"]` | accepted signing algorithms |
| `FERROEHR__AUTH__OIDC__HMAC_SECRET` | unset | an HS256 symmetric secret (development/testing) |
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
| `FERROEHR__AUTHZ__RBAC__ROLE_CLAIMS` | `["realm_access.roles","scope"]` | JWT claim paths mined for roles |
| `FERROEHR__AUTHZ__RBAC__MANAGEMENT_ACCESS` | `admin_only` | management-surface access: `admin_only`, `private`, or `public` |

Roles come from the JWT claims listed in `ROLE_CLAIMS` — by default the
Keycloak `realm_access.roles` array plus the space-separated `scope` claim —
or from a Basic user's configured roles. A clinical operation needs at least one
role; an admin operation needs the admin role; the management surface follows
its tri-state setting. Disabling RBAC restores authentication-only behaviour.

A principal carrying the `readonly_role` (default `READONLY`) is refused on
every write operation — creating an EHR, committing a composition, uploading a
template, and any update/delete — even when it also holds granting roles such
as `ADMIN` (a restriction always overrides a grant). Reads and AQL queries stay
permitted, so a `READONLY` account is an authenticated, view-only principal. The
dev compose stack ships one such account (`ferroehr-readonly`, password
`ferroehr`) for evaluation.

### ABAC (attribute-based, fine-grained)

For attribute-level decisions — "may this user touch this patient's data,
under this organisation, for this template?" — enable ABAC. A policy decision
point is consulted per clinical operation with resolved attributes.

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
rendered in both official formats (FHIR R4 `AuditEvent` per IHE BALP, and
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
