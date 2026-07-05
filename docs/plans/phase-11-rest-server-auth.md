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

- [x] ITS-REST contract generated (`emit-rest`, done)
- [x] P09/P10 available for handlers that touch storage (handlers start as
      `NotImplemented`; the service layer fills them in P12)

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

- [x] `axum` app + router from the generated server traits; `AppState` — a
      generic HTTP dispatcher built from each group's `ROUTES` table
      (`dispatch/`) rebuilds each `*Params` via a type-directed deserializer
      (`params.rs`) and calls the trait method; the 5 `*Api` traits are
      implemented on `AppState` (`api/`, `NotImplemented` stubs via a macro).
- [x] `tower-http` middleware stack — trace, cors, compression, timeout,
      request-id (+propagate), sensitive-headers (Authorization), catch-panic,
      body-limit; normalize-path applied at serve time (wraps the router).
- [x] Content negotiation JSON/XML (`openehr-its`); `ApiError` → response —
      `negotiate.rs`; JSON wired end to end, XML request bodies decoded to
      canonical JSON for the RM write paths (composition/ehr_status/directory),
      typed XML responses ready for P12; `RestError` renders the openEHR JSON
      error body. Extended `openehr-its` `ApiError` with
      `Unauthorized`/`Forbidden`/`UnsupportedMediaType`/`NotAcceptable`.
- [x] **Basic auth** (argon2) + **OAuth2/OIDC** bearer (jsonwebtoken over a JWKS;
      HMAC / static-JWKS / discovered-via-`openidconnect` key sources) as one
      axum middleware + an `AuthenticatedUser` extractor (`auth/`).
- [x] Config (`figment`): `RestConfig` + `AuthConfig` (bind, base path, CORS,
      swagger toggle, auth modes, issuer/JWKS/audience, admin-scope gate).
- [x] `utoipa` OpenAPI + Swagger UI; `/rest/status`, `/health`,
      `/management/info` (public). Binary (`ehrbase`) wired to boot & serve.
- [x] Integration tests: routing, auth (401/403 paths), negotiation
      (`tests/http.rs`, 16 tests) + 32 unit tests.

## Exit criteria

- [x] Server boots; every ITS-REST route is mounted (stubs return typed
      responses) — verified by booting the `ehrbase` binary and by the
      integration suite across all five groups (ehr/demographic/definition/
      query/admin).
- [x] Basic + OAuth2/OIDC authentication enforced + tested; unauthenticated → 401
      (with `WWW-Authenticate`); authenticated-but-unauthorized admin → 403.
- [x] JSON and XML request/response negotiation works end to end — JSON fully at
      the HTTP layer; canonical XML request decode proven end to end (RM types
      via `openehr-its`); typed XML responses land with P12's typed payloads.
- [x] Compiles + clippy-clean (workspace) + tested (`cargo nextest`).

## Decisions made this phase

- Authentication (Basic + OAuth2/OIDC) is Stage-1; RBAC/authz is Stage-2 (ADR-006).
  A coarse **admin-scope gate** is the Stage-1 seam (off by default).
- The server implements the generated contract; it does not re-declare routes.
  A **generic dispatcher** over the generated `ROUTES` tables + a type-directed
  params deserializer avoids ~96 bespoke handlers while staying type-correct.
- **Resource-server scope:** the CDR validates bearer JWTs (jsonwebtoken over a
  JWKS, OIDC discovery via `openidconnect`); the `oauth2` authorization-code
  *client* flow and `axum-login`/`tower-sessions` are not pulled in (a CDR is a
  stateless resource server, not an OAuth2 client) — recorded as a PORT NOTE.
- `jsonwebtoken` 10 uses the **aws-lc-rs** crypto provider (matches the rustls
  stack); pin updated in the workspace manifest.
- Design docs authored this session (Rust-native, Stage-2/observability):
  `docs/enterprise/atna-audit.md` (ATNA audit trail) and `docs/observability.md`
  (status/health/metrics on OpenTelemetry + Prometheus).

## Handoff for next session (P12)

P11 is complete: the `ehrbase-rest` axum app implements all five generated
ITS-REST traits, boots via the `ehrbase` binary, enforces Basic + OAuth2/OIDC
auth, and negotiates JSON/XML — handlers return `NotImplemented`. **P12** fills
the handler bodies with the service layer: EHR / EHR_STATUS / COMPOSITION /
CONTRIBUTION create/get/update/delete on the P10 node-codec storage, with
versioning + contribution/audit, **end-to-end tested against a real PG 18.4
testcontainer** (per the owner's directive to make the REST API genuinely
e2e-testable). The dispatcher already decodes RM-typed bodies (incl. canonical
XML) into the `serde_json::Value` the traits receive, so P12 works against typed
values; wire typed XML *responses* via `negotiate::respond_negotiated` as typed
payloads land.
