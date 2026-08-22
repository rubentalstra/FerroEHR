// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The FHIR integration core.
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
//! `fhir-model` resource surface — no other crate names a `fhir_model`
//! type.
//!
//! # Release identity
//!
//! The **connector speaks FHIR R4** — the release its wire advertises
//! (`/fhir/r4`). Every resource it builds (`AuditEvent`, `Bundle`,
//! `OperationOutcome`) and every data type they carry (`CodeableConcept`,
//! `Coding`, `Identifier`, `Meta`, `Reference`) is outside R4B's changed set,
//! so the documents are equally valid under either release: R4B changed only
//! the medication-definition, clinical-reasoning and subscription families and
//! added `CodeableReference`/`RatioRange`, and "implementers that do not use
//! the specific portions where changes have been made can continue to use
//! either R4 or R4B without any functional difference"
//! (<https://hl7.org/fhir/R4B/r4b-explanation.html>). [`mapping`] and
//! [`reverse`] fix no resource at all — the resource type is mapping data. The
//! typed model is the `fhir-model` crate's `r4b` generation, which is an
//! implementation detail of the dependency, not the wire's release.
//!
//! [`terminology`] is the exception and stays **R4B** by its own design: it
//! decodes responses from external terminology servers, whose release is the
//! remote's property, not ours.

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
