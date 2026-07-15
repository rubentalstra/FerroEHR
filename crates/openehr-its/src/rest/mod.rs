//! **ITS-REST** — the openEHR REST API contract (ITS-REST 1.0.3+).
//!
//! The transport DTOs, a server trait per API group, and a route table are
//! **generated** by `openehr-codegen`'s `emit-rest` target into [`generated`],
//! spec-first from the vendored OpenAPI (`vendor/rest-oas/*-codegen.openapi.yaml`).
//! RM payload types resolve to `openehr-rm`/`openehr-base` rather than being
//! re-emitted. This module is the hand-written [`runtime`] (the `ApiError`
//! response type) + re-exports. `ehrbase-rest` implements the generated traits
//! and wires axum; the handler bodies are our own service layer (`ehrbase`,
//! the openEHR specs are the authority, EHRbase is prior art).
//! Regenerate with `cargo run -p openehr-codegen -- emit-rest`.

pub mod generated;
pub mod runtime;

pub use runtime::ApiError;

/// The vendored ITS-REST OpenAPI bundles (the `-html` documentation variant
/// of the same pinned tree `emit-rest` generates from — provenance in
/// `vendor/rest-oas/PROVENANCE.md`), embedded verbatim so a server can serve
/// the authoritative contract for discoverability (e.g. Swagger UI). One
/// `(api_group, yaml)` entry per API group.
pub const VENDORED_OAS: &[(&str, &str)] = &[
    (
        "ehr",
        include_str!("../../vendor/rest-oas/ehr-html.openapi.yaml"),
    ),
    (
        "query",
        include_str!("../../vendor/rest-oas/query-html.openapi.yaml"),
    ),
    (
        "definition",
        include_str!("../../vendor/rest-oas/definition-html.openapi.yaml"),
    ),
    (
        "demographic",
        include_str!("../../vendor/rest-oas/demographic-html.openapi.yaml"),
    ),
    (
        "admin",
        include_str!("../../vendor/rest-oas/admin-html.openapi.yaml"),
    ),
    (
        "system",
        include_str!("../../vendor/rest-oas/system-html.openapi.yaml"),
    ),
    (
        "overview",
        include_str!("../../vendor/rest-oas/overview-html.openapi.yaml"),
    ),
];
