//! `MEASUREMENT_SERVICE` — proxy access to a measurement information
//! service.
//!
//! openEHR interface: `MEASUREMENT_SERVICE`, package `rm.support`
//! (`rm.support.measurement`).
//!
//! Defines an object providing proxy access to a measurement information
//! service. The Measurement package defines a minimum of semantics
//! relating to quantitative measurement, units, and conversion, enabling
//! the Quantity package of the openEHR Data Types Information Model to be
//! correctly expressed. Note that this service as currently defined in no
//! way seeks to properly model the semantics of units, conversions etc. —
//! it provides only the minimum functions required by the openEHR
//! Reference Model.
//!
//! PORT NOTE: `MEASUREMENT_SERVICE` declares only `Functions`, no
//! `Attributes`, so per ADR-001 §1 (abstract class without attributes →
//! trait) it is transcribed as a Rust `trait`, not a struct+enum+API triple.
//! This differs in shape from `EXTERNAL_ENVIRONMENT_ACCESS`'s other parent,
//! `openehr-terminology::TerminologyService`, which is a **concrete owned
//! struct** with a bundled-asset-backed implementation (P2). That asymmetry
//! is the crux of the flagged mismatch in `external_environment_access.rs`
//! — see that file's doc comment.
pub trait MeasurementService {
    /// Spec `is_valid_units_string(units: String): Boolean` — true if the
    /// units string `units` is a valid string according to the HL7 UCUM
    /// specification.
    fn is_valid_units_string(&self, units: &str) -> bool;

    /// Spec `units_equivalent(units1: String, units2: String): Boolean` —
    /// true if two units strings correspond to the same measured property.
    fn units_equivalent(&self, units1: &str, units2: &str) -> bool;
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 support §measurement_package MEASUREMENT_SERVICE — docs/research/spec-cache/RM-1.1.0/uml_classes/measurement_service.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-measurement_package.adoc §Class Definitions / uml_classes/measurement_service.adoc §MEASUREMENT_SERVICE Class
//   confidence: high
//   todos: 0
//   note: no concrete implementor transcribed here (the spec defines only the interface); a UCUM-backed impl is a later-phase concern (P11 DV_QUANTITY validation per openehr-terminology's PropertyUnitData, per that crate's own PORT STATUS note).
// ─────────────────────────────────────────────
