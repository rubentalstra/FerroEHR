//! **ITS-REST** — the openEHR REST API contract (ITS-REST 1.0.3+).
//!
//! The machine-readable OpenAPI specs are vendored at `vendor/rest-oas/`
//! (`*-codegen.openapi.yaml` per API group: ehr, definition, query, admin,
//! demographic, system, overview) from `specifications-ITS-REST`. They are the
//! authoritative contract; the axum server that implements them is
//! `ehrbase-rest` (P6), code-first via `utoipa` and cross-checked against these.
//!
//! `// TODO(port):` shared request/response DTOs and the endpoint contract
//! derived from the vendored OAS.
