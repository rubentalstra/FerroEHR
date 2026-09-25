// SPDX-FileCopyrightText: Vernum Projecten B.V.
// SPDX-FileCopyrightText: openEHR Foundation
// SPDX-License-Identifier: Apache-2.0

//! **ITS-REST** — the openEHR REST API contract (ITS-REST 1.1.0).
//!
//! The transport DTOs, the per-operation param structs, a server trait and a
//! client per API group, and a route table are **generated** by
//! `openehr-codegen`'s `emit-rest` target into [`generated`], spec-first from
//! the vendored OpenAPI (`vendor/rest-oas/*-codegen.openapi.yaml`) — both
//! halves from one read, so they cannot drift. RM payload types resolve to
//! `openehr-rm`/`openehr-base` rather than being re-emitted. The hand-written
//! parts are [`runtime`] (the `ApiError` type the server traits return) and
//! [`client`] (the engine seam, credentials, retries and errors the generated
//! clients build on). `ferroehr-rest` implements the generated traits and
//! wires axum; a consumer that calls a CDR takes the generated clients.
//! Regenerate with `cargo run -p openehr-codegen -- emit-rest`.

// The contract (DTOs, params, routes, `ApiError`) rides `rest`; the server
// traits and the axum mapping ride `rest-server`; the generated clients and
// the client runtime ride `rest-client` — see the crate docs.
#[cfg(feature = "rest-client")]
pub mod client;
#[cfg(feature = "rest")]
pub mod generated;
#[cfg(feature = "rest")]
pub mod runtime;
