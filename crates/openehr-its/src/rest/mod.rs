// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-FileCopyrightText: openEHR Foundation
// SPDX-License-Identifier: BUSL-1.1 AND Apache-2.0

//! **ITS-REST** — the openEHR REST API contract (ITS-REST 1.1.0).
//!
//! The transport DTOs, a server trait per API group, and a route table are
//! **generated** by `openehr-codegen`'s `emit-rest` target into [`generated`],
//! spec-first from the vendored OpenAPI (`vendor/rest-oas/*-codegen.openapi.yaml`).
//! RM payload types resolve to `openehr-rm`/`openehr-base` rather than being
//! re-emitted. This module is the hand-written [`runtime`] (the `ApiError`
//! response type) + re-exports. `ferroehr-rest` implements the generated traits
//! and wires axum; the handler bodies are our own service layer (`ferroehr`),
//! with the openEHR specs as the authority.
//! Regenerate with `cargo run -p openehr-codegen -- emit-rest`.

// The generated contract and its response runtime are the SERVER half of the
// contract (axum, serde, the spec crates), so they ride `rest-server`; the
// std-only `smart_scopes` grammar is always compiled — see the crate docs.
#[cfg(feature = "rest-server")]
pub mod generated;
#[cfg(feature = "rest-server")]
pub mod runtime;
pub mod smart_scopes;
