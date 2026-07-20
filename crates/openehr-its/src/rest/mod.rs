//! **ITS-REST** — the openEHR REST API contract (ITS-REST 1.1.0).
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
