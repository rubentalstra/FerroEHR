---
paths: ["crates/ehrbase-rest/**", "crates/ehrbase-compat/**", "crates/ehrbase/src/application/**"]
---

# REST server (axum) — P11, P17

The server is `axum` 0.8 **implementing the generated ITS-REST contract** from
`openehr-its::rest::generated` (ADR-005/006). It does not re-declare routes or
DTOs — it provides `impl {Group}Api for AppState` and mounts the generated
router.

## Rules

- **Implement the generated server traits.** One `impl` per API group
  (ehr/composition/directory/contribution/query/definition/admin); the handler
  bodies are our service layer (P12). RM payloads
  are `openehr-rm` types; transport DTOs come from `openehr-its`.
- **Middleware = `tower-http`, not hand-rolled:** trace, cors, compression,
  timeout, request-id, sensitive-headers, catch-panic, normalize-path. Auth is a
  `tower`/extractor layer (see `auth.md`).
- **Content negotiation** (canonical JSON + XML) goes through `openehr-its`
  (`to_canonical_json`/`from_canonical_json`, `to_canonical_xml`/
  `from_canonical_xml`); honour `Accept`/`Content-Type`/`Prefer` exactly as
  the ITS-REST spec requires.
- **Errors → responses** via `openehr-its::rest::runtime::ApiError` (it carries
  the ITS-REST status codes). Map service/domain errors into it; return the
  openEHR error body shape the spec defines.
- **OpenAPI:** `utoipa` may emit our OAS as a CI **drift-check** against the
  vendored upstream OAS — a drift signal, never the source of truth (the
  vendored OAS is authoritative, ADR-005). Serve Swagger UI.
- **EhrScape / admin** (`/rest/ecis/v1/*`, `/rest/admin`) live in
  `ehrbase-compat` (P17), reusing the same service layer + `openehr-flat`.

Behaviour is verified by the CNF conformance suite (P19), not by mirroring another server's
Java controller structure. Build compiling + tested (`rust-style.md`).
