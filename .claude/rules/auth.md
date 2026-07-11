---
paths: ["app/ehrbase-rest/**", "app/ehrbase/src/config/**", "app/ehrbase/src/**/security/**"]
---

# Authentication — Basic + OAuth2/OIDC (Stage 1)

EHRbase Java supports **Basic auth** and **OAuth2/OIDC** (Keycloak-style)
authentication; we do the same in **Stage 1** (ADR-006). Fine-grained
**RBAC / attribute authorization is Stage 2** — do not build it now.

## Rules

- **Use the pinned crates, don't hand-roll crypto or token parsing:**
  `argon2` (+ `password-hash`) for password verification (Basic auth),
  `jsonwebtoken` for JWT validation, `oauth2` + `openidconnect` for the
  OAuth2/OIDC flow + JWKS/issuer discovery (Keycloak), `axum-login` /
  `tower-sessions` where session state helps. `secrecy`/`zeroize` for secrets.
- **Authentication is a `tower`/axum middleware + extractor**, applied to the
  generated router (see `rest-axum.md`) — one place, not per-handler. An
  authenticated principal is put in request extensions for handlers/service.
- **Config-driven** (`figment`/`config`): which auth modes are enabled, the
  OIDC issuer/JWKS URL, audience, Basic-auth user store. Mirror EHRbase's
  `application.yml` security options behaviourally.
- **Unauthenticated → 401; authenticated-but-unauthorized → 403** — match
  EHRbase's status/behaviour at the REST surface (parity-verified). Public
  endpoints (`/rest/status`, health, Swagger) are exempt as EHRbase exempts them.
- **No RBAC/permission checks in Stage 1** beyond "is authenticated". Row-level
  / AQL-result filtering by permission is the Stage-2 restoration (now shipped
  as the `ehrbase-rest::access` module, ADR-015/E-arc); historically a
  `// TODO(port): Stage 2 RBAC` seam, not an implementation.

The CNF security suites (`docs/specs/openehr/CNF/tests/platform/robot/SECURITY_TESTS/`,
incl. the Keycloak OAuth2 setup) are the conformance reference for the
401/403 behaviour and the Basic + bearer flows.

Errors map through `openehr-its::rest::runtime::ApiError`. Build compiling +
tested (auth middleware needs unit + integration tests: 401/403 paths, valid/
invalid tokens, Basic + bearer).
