//! `DV_TIME_SPECIFICATION` — abstract ancestor of all timing specifications.
//!
//! openEHR class: `DV_TIME_SPECIFICATION` (abstract), package
//! `rm.data_types.time_specification`.
//! Inherits: `DATA_VALUE`.
//!
//! This is an abstract class of which all timing specifications are
//! specialisations. Specifies points in time, possibly linked to the
//! calendar, or a real world repeating event, such as "breakfast".
//!
//! The `data_types.time_specification` package provides types for
//! expressing *potential* time (as opposed to actual time), based on the
//! HL7v3 types of the same names (`PIVL<T:TS>`, `EIVL<T:TS>`, GTS). See
//! `docs/research/spec-cache/RM-1.1.0/data_types/master08-time_specification_package.adoc`
//! for the HL7 syntax background transcribed into the two concrete
//! subclasses' doc comments.
//!
//! # Forward references
//!
//! `DATA_VALUE` (the RM-wide closed abstract root, ADR-001 §4) is
//! transcribed in the `data_types` root cluster, concurrent with this
//! transcription pass and not yet landed — this abstract class's own state
//! (none beyond the inherited `DATA_VALUE` marker) is therefore not
//! embedded via a shared struct here; only its function contract is
//! transcribed as a trait. `DV_PARSABLE` is transcribed in this same
//! session's `encapsulated` package (`dv_parsable.rs`), imported directly.
use crate::data_types::encapsulated::dv_parsable::DvParsable;

/// `DV_TIME_SPECIFICATION` is modelled as a Rust trait: it is abstract, its
/// own per-class table declares no attributes beyond the required `value:
/// DV_PARSABLE` (below) plus three abstract functions every concrete
/// descendant redefines with a concrete ("effected") body.
///
/// Per ADR-001 §4, `DV_TIME_SPECIFICATION`'s two concrete descendants
/// (`DV_PERIODIC_TIME_SPECIFICATION`, `DV_GENERAL_TIME_SPECIFICATION`) form
/// a small closed subtype set; a `TimeSpecification` enum wrapping both
/// could be added once both concrete types exist, but is not written in
/// this file — that decision belongs beside whichever RM-wide
/// `DATA_VALUE`/`DvOrdered` enum eventually needs to hold a
/// `DV_TIME_SPECIFICATION` variant, since this package alone has no caller
/// requiring the closed-enum shape yet.
pub trait DvTimeSpecification {
    /// `value`: `DV_PARSABLE` (`1..1`).
    ///
    /// The specification, in the HL7v3 syntax for `PIVL` or `EIVL` types.
    fn value(&self) -> &DvParsable;

    /// `calendar_alignment` `(): String` (abstract).
    ///
    /// Indicates what prototypical point in the calendar the specification
    /// is aligned to, e.g. "5th of the month". Empty if not aligned.
    /// Extracted from the `value` attribute.
    ///
    /// TODO(port): requires parsing `value`'s HL7v3 `PIVL`/`EIVL`/GTS syntax
    /// (see the phase-linked/event-linked/general syntax EBNF in the
    /// package overview adoc); no parsing engine exists yet for this
    /// HL7-derived syntax family (distinct from the ISO 8601 date/time
    /// parsing deferred to jiff at P17 — this is its own grammar, not
    /// covered by that bridging plan).
    fn calendar_alignment(&self) -> String;

    /// `event_alignment` `(): String` (abstract).
    ///
    /// Indicates what real-world event the specification is aligned to if
    /// any. Extracted from the `value` attribute.
    ///
    /// TODO(port): see `calendar_alignment` above — same HL7v3 syntax
    /// parsing gap.
    fn event_alignment(&self) -> String;

    /// `institution_specified` `(): Boolean` (abstract).
    ///
    /// Indicates if the specification is aligned with institution
    /// schedules, e.g. a hospital nursing changeover or meal serving times.
    /// Extracted from the `value` attribute.
    ///
    /// TODO(port): see `calendar_alignment` above — same HL7v3 syntax
    /// parsing gap.
    fn institution_specified(&self) -> bool;
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.time_specification — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_time_specification.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-time_specification_package.adoc §Class Descriptions / dv_time_specification.adoc §DV_TIME_SPECIFICATION Class
//   confidence: medium
//   todos: 3
//   note: abstract class, no attributes beyond value: DV_PARSABLE and three abstract functions -> pure trait (ADR-001 §1/§3 boundary case: no shared state struct needed since DATA_VALUE's own state is not yet landed and this class adds no further attributes). All three functions require an HL7v3 PIVL/EIVL/GTS syntax parser not yet designed (distinct from the ISO-8601-specific jiff bridging plan). Closed-enum decision for the two concrete descendants deferred to whichever call site needs it.
// ─────────────────────────────────────────────
