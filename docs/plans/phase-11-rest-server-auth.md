# Phase 11 — REST server foundation + authentication

- Status: not-started (Stage-1 app build, step 3 of 13)
- Consumes: `openehr-its` (generated ITS-REST server traits + DTOs, ADR-005),
  `openehr-rm`/`openehr-its` (JSON/XML payloads)
- Compile required: yes (compiling, tested increment)
- Decisions: ADR-005 (spec-first contract), ADR-006 (modern stack + auth)

## Objectives

The `ehrbase-rest` HTTP foundation: an `axum` app that **implements the
generated ITS-REST server traits** from `openehr-its::rest::generated`, wired
with a modern middleware stack and **Basic + OAuth2/OIDC authentication**. This
is the "proper way" application entry surface — no hand-rolled routing/auth.

## Preconditions

- [ ] ITS-REST contract generated (`emit-rest`, done)
- [ ] P09/P10 available for handlers that touch storage (handlers can start as
      `NotImplemented` and fill in as P12 lands)

## Scope

**In:** `axum` 0.8 router built from the generated `ROUTES`/server traits;
`tower-http` layers (trace, cors, compression, timeout, request-id,
sensitive-headers, catch-panic, normalize-path); content negotiation (canonical
JSON + XML via `openehr-its`, `Prefer`/`Accept`/`Content-Type` handling);
`ApiError → Response` mapping (`openehr-its::rest::runtime`); **authentication**
— Basic (`argon2` verify) + OAuth2/OIDC bearer validation (`jsonwebtoken`,
`oauth2`, `openidconnect`; Keycloak-style), as an axum middleware/extractor
(`axum-login`/`tower-sessions` where useful); `utoipa` Swagger UI (drift-checked
vs the vendored OAS); `/rest/status`, `/management/*`, health.
**Out:** the business logic behind each endpoint (P12); fine-grained RBAC/authz
(Stage 2); FLAT/EhrScape endpoints (P17, in `ehrbase-compat`).

## Tasks

- [ ] `axum` app + router from the generated server traits; `AppState`
- [ ] `tower-http` middleware stack
- [ ] Content negotiation JSON/XML (`openehr-its`); `ApiError` → response
- [ ] **Basic auth** (argon2) + **OAuth2/OIDC** bearer auth middleware/extractor
- [ ] Config (`figment`): auth mode(s), issuer/JWKS, CORS, etc.
- [ ] `utoipa` OpenAPI + Swagger UI; status/management/health
- [ ] Integration tests: routing, auth (401/403 paths), negotiation

## Exit criteria

- [ ] Server boots; every ITS-REST route is mounted (stubs return typed responses)
- [ ] Basic + OAuth2/OIDC authentication enforced + tested; unauthenticated → 401
- [ ] JSON and XML request/response negotiation works end to end
- [ ] Compiles + clippy-clean

## Decisions made this phase

- Authentication (Basic + OAuth2/OIDC) is Stage-1; RBAC/authz is Stage-2 (ADR-006).
- The server implements the generated contract; it does not re-declare routes.
