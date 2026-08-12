// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The FHIR R4B integration core.
//!
//! The mapping model + FLAT builder, the outbound reverse-map, the
//! feeder-audit probe, the ATNA `AuditEvent` renderer, and the
//! terminology-response decoder.
//!
//! **No openEHR spec governs FHIR resource representation — our own
//! design/extension.** The platform's service/REST glue (ingest, mapping
//! store, the outbound emitter loop, the audit sinks, the terminology HTTP
//! client) lives in the platform crate behind its `fhir` cargo feature; this
//! module owns only the pure conversion logic and the typed
//! `fhir-model` R4B resource surface — no other crate names a `fhir_model`
//! type.

pub mod audit;
pub mod feeder_audit;
pub mod mapping;
pub mod reverse;
pub mod terminology;

/// The EHR subject a mapped inbound resource addresses.
///
/// The mapping's output identity (id + namespace, a person by definition of
/// the inbound connector), which the platform resolves through its EHR index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedSubject {
    /// The subject identifier value.
    pub id: String,
    /// The identifier namespace.
    pub namespace: String,
}
