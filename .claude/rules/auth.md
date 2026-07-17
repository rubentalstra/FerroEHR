---
paths: ["app/ehrbase-rest/**", "app/ehrbase/src/config/**", "app/ehrbase/src/**/security/**"]
---

# Authentication & authorization (rewritten 2026-07-13 — authn + access module are SHIPPED)

**State:** Basic + OAuth2/OIDC authentication is shipped and live.
Authorization lives in the **`ehrbase-rest::access` module** (the RBAC/ABAC
policy-enforcement point; the old `ehrbase-authz` crate was dissolved into
it), with **SMART resource-scope enforcement** (`ehrbase-rest::smart`)
AND-composed onto that PEP — config-gated, off by default. This rule governs
maintenance and extension, not initial build.

## Spec authority

- ITS-REST §Authentication + SM `master02` place authorization largely **out
  of band** — fine-grained authz is our own extension where the spec is
  silent; flag it as such in comments ("no openEHR spec governs this"),
  never cite an internal doc (spec-adherence.md).
- **401 vs 403 discipline:** unauthenticated → 401, authenticated-but-
  unauthorized → 403, per the ITS-REST text — verified by the ECC SEC cases
  (`ECC-SEC-*` in `tools/conformance`), not by comparison with any other
  server. Public endpoints (`/rest/status`, health, discovery documents
  incl. `/.well-known/smart-configuration`) stay outside the auth layer.
- SMART scope grammar/enforcement: the vendored
  `docs/specs/openehr/ITS-REST/docs/smart_app_launch/` text (master08 is the
  load-bearing scope grammar). EHR-level access: the
  `ehrbase.access_control.v1` scheme implemented in
  `app/ehrbase/src/service/ehr/access_types.rs` (no openEHR spec defines a
  concrete ACCESS_CONTROL_SETTINGS scheme — our own design).

## Rules

- **Use the pinned crates, never hand-roll crypto or token parsing:**
  `argon2` (+`password-hash`) for Basic-auth password verification,
  `jsonwebtoken` for JWT validation, `oauth2` + `openidconnect` for
  OAuth2/OIDC + JWKS/issuer discovery (Keycloak-style), `tower-sessions`
  where session state helps, `secrecy`/`zeroize` for secret material.
- **Auth is a `tower`/axum middleware + extractor** on the generated router
  — one place, never per-handler. The authenticated principal goes into
  request extensions; the `access` PEP consumes it.
- **Config-driven** (one TOML file, `ehrbase.toml`, with environment-variable
  overrides): enabled modes, OIDC issuer/JWKS, audience, the Basic user store.
  Config changes are user-visible → same-PR website-book + changelog entries.
- **Layer order is load-bearing:** authn → `access` (RBAC/ABAC) → SMART
  scopes (AND-composed — SMART can only narrow, never widen). Disabled
  SMART must produce zero wire drift.
- Errors map through `openehr-its::rest::runtime::ApiError`; never invent
  an error body shape.
- Every auth change lands with tests (401/403 paths, valid/expired/wrong-
  audience tokens, Basic + bearer, scope-narrowing cases) and an ECC run
  showing zero drift — the SEC area covers this surface.
