# SMART App Launch

EHRbase-rs can act as the **resource server** in a SMART App Launch setup: a
clinical app is launched with an OAuth2/OIDC token from your authorization
server (Keycloak or any standards-compliant IdP), and the CDR advertises that
server's endpoints, understands SMART resource scopes in the token, and binds
the launch context (the selected patient/EHR) to what the token may touch.
EHRbase-rs never issues tokens, registers clients, or serves the OAuth2
endpoints itself — those remain your authorization server's job.

Support is **off by default**. A stock server serves no discovery document and
runs no scope gate, so the wire is byte-identical to a non-SMART deployment
until you opt in.

<!-- toc -->

## Enabling it

Turn it on and tell the server where your authorization server lives:

```bash
export EHRBASE__SMART__ENABLED=true
export EHRBASE__SMART__ENDPOINTS__AUTHORIZATION_ENDPOINT=https://as.example/auth
export EHRBASE__SMART__ENDPOINTS__TOKEN_ENDPOINT=https://as.example/token
```

SMART scopes ride only **Bearer** tokens, so pair this with OIDC bearer
authentication (see [Security & multi-tenancy](security.md#authentication)).
If SMART is enabled without any bearer mechanism, the server logs a warning at
boot: it can serve discovery, but the scope gate will never see a scope.

The full key set lives in the `[smart]` section of `ehrbase.toml`; each key
can be overridden with the shown `EHRBASE__SMART__*` environment variable (`__`
separates nested fields):

| Key | Default | Meaning |
|---|---|---|
| `EHRBASE__SMART__ENABLED` | `false` | Master switch. Off = no discovery document (404) and an inert scope gate. |
| `EHRBASE__SMART__PLATFORM_BASE_URL` | unset | Base the discovery document hangs off. Unset = the REST root (`/ehrbase/rest`). A leading path is honoured (`/gateway/v1` → `/gateway/v1/.well-known/smart-configuration`). |
| `EHRBASE__SMART__EHR_ID_CLAIM` | `ehrId` | Token claim carrying the launch context's openEHR EHR id. |
| `EHRBASE__SMART__PATIENT_CLAIM` | `patient` | Fallback launch-context claim when the EHR-id claim is absent. |
| `EHRBASE__SMART__REQUIRE_SMART_SCOPES` | `false` | Fail-closed switch — see [Advisory vs required](#advisory-vs-required) below. |
| `EHRBASE__SMART__EPISODE__ENABLED` | `false` | Advertise + accept episode launch context (experimental; advisory only, no episode filtering). |
| `EHRBASE__SMART__LAUNCH_BASE64_JSON` | `false` | Advertise the base64-JSON launch-parameter capability (experimental; consumed by the app, not the CDR). |
| `EHRBASE__SMART__ENDPOINTS__ISSUER` | unset | Advertised token issuer. Unset = falls back to the configured OIDC bearer issuer. |
| `EHRBASE__SMART__ENDPOINTS__JWKS_URI` | unset | Advertised `jwks_uri`. |
| `EHRBASE__SMART__ENDPOINTS__AUTHORIZATION_ENDPOINT` | unset | Advertised OAuth2 authorization endpoint. |
| `EHRBASE__SMART__ENDPOINTS__TOKEN_ENDPOINT` | unset | Advertised OAuth2 token endpoint. |
| `EHRBASE__SMART__ENDPOINTS__REGISTRATION_ENDPOINT` | unset | Advertised dynamic-client registration endpoint. |
| `EHRBASE__SMART__ENDPOINTS__INTROSPECTION_ENDPOINT` | unset | Advertised token introspection endpoint. |
| `EHRBASE__SMART__ENDPOINTS__REVOCATION_ENDPOINT` | unset | Advertised token revocation endpoint. |
| `EHRBASE__SMART__ENDPOINTS__MANAGEMENT_ENDPOINT` | unset | Advertised user management endpoint. |
| `EHRBASE__SMART__ENDPOINTS__TOKEN_ENDPOINT_AUTH_METHODS_SUPPORTED` | `[]` | Advertised client auth methods (e.g. `client_secret_basic`, `private_key_jwt`). |
| `EHRBASE__SMART__ENDPOINTS__GRANT_TYPES_SUPPORTED` | `[]` | Advertised grant types. `implicit` and the password grant are deprecated in SMART and **rejected at boot**. |
| `EHRBASE__SMART__ENDPOINTS__RESPONSE_TYPES_SUPPORTED` | `[]` | Advertised response types (e.g. `code`). |
| `EHRBASE__SMART__ENDPOINTS__CODE_CHALLENGE_METHODS_SUPPORTED` | `[]` | Advertised PKCE methods (e.g. `S256`). |
| `EHRBASE__SMART__ENDPOINTS__SCOPES_SUPPORTED` | `[]` | Advertised scopes. Empty = a default list reflecting what the CDR enforces. |

Every unset optional endpoint is simply omitted from the discovery document —
the server advertises your values verbatim and validates none of them beyond
the deprecated-grant check.

## The discovery document

When enabled, the server serves the standard SMART configuration document,
**unauthenticated**, at:

```text
GET /ehrbase/rest/.well-known/smart-configuration
```

(relative to the REST root — the configured base path without its
`/openehr/v1` tail — or to `PLATFORM_BASE_URL` when set). A launching app
reads it to find your authorization server. It looks like:

```json
{
  "issuer": "https://as.example/realms/ehrbase",
  "authorization_endpoint": "https://as.example/auth",
  "token_endpoint": "https://as.example/token",
  "capabilities": ["context-openehr-ehr", "openehr-permission-v1"],
  "scopes_supported": [
    "openid", "profile", "offline_access",
    "launch", "launch/patient",
    "patient/composition-*.cruds", "patient/aql-*.rs",
    "user/composition-*.cruds", "user/template-*.cruds", "user/aql-*.cruds",
    "system/composition-*.cruds", "system/aql-*.cruds"
  ],
  "response_types_supported": ["code"],
  "services": [
    { "type": "org.openehr.rest", "baseUrl": "/ehrbase/rest/openehr/v1" }
  ]
}
```

- `capabilities` always contains `context-openehr-ehr` and
  `openehr-permission-v1`, plus `context-openehr-episode` and
  `launch-base64-json` when those switches are on.
- `services` names the openEHR REST service with its base URL, plus the FHIR
  façade (`org.fhir.rest`) when the FHIR routes are enabled.
- `scopes_supported` is the default list above unless you configure your own,
  which is then emitted verbatim.

With SMART disabled the path is not mounted at all (404).

## The scope grammar

SMART resource scopes have the form
**`<compartment>/<resource>.<permissions>`**:

- **Compartment** — `patient` (the launch context's EHR only), `user` (what
  the user may see), or `system` (a backend service, no user).
- **Resource** — one of three families: `composition-<template-id>`,
  `template-<template-id>`, or `aql-<stored-query-name>`.
- **Permissions** — any combination of `c` create, `r` read, `u` update,
  `d` delete, `s` search/execute (order-free, e.g. `.crud`, `.rs`).

Resource ids accept wildcards: `*` matches within one `::`-delimited namespace
segment, `**` matches across namespaces, and a bare `*` or `**` matches every
id. The permission tail is split at the *last* dot, so template ids and query
names keep their internal dots and versions.

| Scope | Grants |
|---|---|
| `patient/composition-*.crud` | Create, read, update, and delete any composition — but only in the launched patient's EHR. |
| `patient/aql-*.rs` | Read and execute any stored query, scoped to the launched patient's EHR. |
| `patient/composition-MyHospital::Template.v0.r` | Read compositions of exactly that template, in the launched patient's EHR. |
| `user/composition-MyHospital::*.r` | Read compositions of any template in the `MyHospital` namespace (not sub-namespaces). |
| `user/template-*.cruds` | Full access to template definitions. |
| `system/aql-org.openehr::bloodpressure.v1.rs` | A backend service may read and execute that one stored query, across all EHRs. |

Scopes the server does not recognise are ignored (never granted, never
fatal); the identity scopes (`openid`, `profile`, `offline_access`, …) and
the `launch`/`launch/patient` context scopes pass through untouched.

## Launch context: binding to one EHR

When an app is launched for a patient, your authorization server puts the
resolved openEHR EHR id in the token — by default in an `ehrId` claim, with
the standard SMART `patient` claim as fallback (both claim names are
configurable). Any operation permitted *only* by a `patient/…` scope is then
bound to that one EHR: requests against any other EHR are refused, and a
token holding only patient-compartment scopes but **no** launch-context claim
is refused outright. `user/` and `system/` scopes carry no such binding.

## How scopes compose with RBAC and ABAC

The SMART gate is one more layer in the authorization chain, evaluated
**after** authentication, the per-EHR `EHR_ACCESS` gate, RBAC, and ABAC (see
[Security & multi-tenancy](security.md#authorization)). Every active layer
must allow the request; SMART never overrides a denial from another layer. A
scope denial is a **403 Forbidden**.

In this release the scope gate enforces the **composition** family —
composition reads and writes are checked against `…/composition-….…` scopes
and the patient-compartment binding. The `template` and `aql` scope forms are
parsed and advertised so authorization servers can issue them; those
families, like the EHR, EHR_STATUS, CONTRIBUTION, and DIRECTORY operations
(for which SMART defines no resource type), remain governed by the RBAC/ABAC
layers.

### Advisory vs required

- **Advisory (default, `REQUIRE_SMART_SCOPES=false`)** — the gate enforces
  only when the token actually carries SMART resource scopes for the resource
  family in question. A non-SMART token (or a Basic-auth caller, which has no
  scopes) is unaffected. Once a token *does* carry, say, composition scopes,
  at least one of them must match the operation or the request is refused.
- **Required (`REQUIRE_SMART_SCOPES=true`)** — fail-closed: a Bearer token
  with no matching SMART resource scope for a scope-governed operation is
  denied, matching deployments where every app is a SMART app.

> [!NOTE]
> Episode context is experimental: enabling it advertises the capability and
> accepts `launch/episode`, but the server applies no episode-scoped
> filtering — openEHR has no first-class episode resource yet.
