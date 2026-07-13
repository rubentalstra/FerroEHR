---
paths: ["app/ehrbase-rest/**", "app/ehrbase/src/application/**"]
---

# REST server (axum) — the ITS-REST protocol adapter (shipped; rule governs maintenance/extension)

The server is `axum` 0.8 **implementing the generated ITS-REST contract** from
`openehr-its::rest::generated` (ADR-005/006). It does not re-declare routes or
DTOs — it provides `impl {Group}Api for AppState` and mounts the generated
router.

## Spec sources (the oracle)

Endpoint semantics (methods, status codes, headers, `Prefer`/`If-Match`
handling, error bodies) come from the vendored spec text — never from memory
or another server: `docs/specs/openehr/ITS-REST/` (the REST spec + API
definitions behind the generated contract) and
`docs/specs/openehr/SM/docs/openehr_platform/` (the abstract service model the
REST API realizes). The CNF chapters
`docs/specs/openehr/CNF/docs/platform_test_schedule/master06`-`master12`
(EHR / COMPOSITION / CONTRIBUTION / DIRECTORY / demographic / querying /
admin) + the Robot suites under `CNF/tests/platform/robot/<API_GROUP>/` give
the exact request/response pairs a conformant server must produce — when the
prose feels ambiguous, the CNF test case wins. Cite sections
(spec-adherence.md; `/spec-lookup`).

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
  `ehrbase-rest` as a feature-gated adapter module (`ehrscape`, P17), reusing the same service layer + `openehr-flat` (the `ehrbase-compat` crate was removed 2026-07-09 — ADR-010).

Behaviour is verified by the **ECC suite** (`tools/conformance` — the CNF
schedule text is the oracle it derives from), never by mirroring another
server. Every wire-visible change ends with an ECC run showing zero drift vs
the committed baseline. Build compiling + tested (`rust-style.md`); REST
surface changes are user-visible → same-PR website-book + changelog entries.
