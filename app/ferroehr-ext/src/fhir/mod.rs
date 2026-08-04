//! The FHIR R4B integration core: the mapping model + FLAT builder, the
//! outbound reverse-map, and the feeder-audit probe.
//!
//! **No openEHR spec governs FHIR resource representation — our own
//! design/extension.** The platform's service/REST glue (ingest, mapping
//! store, the outbound emitter loop) lives in the platform crate behind its
//! `fhir` cargo feature; this module owns only the pure conversion logic.

pub mod feeder_audit;
pub mod mapping;
pub mod reverse;

/// The EHR subject a mapped inbound resource addresses — the mapping's
/// output identity (id + namespace, a person by definition of the inbound
/// connector), which the platform resolves through its EHR index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedSubject {
    /// The subject identifier value.
    pub id: String,
    /// The identifier namespace.
    pub namespace: String,
}
