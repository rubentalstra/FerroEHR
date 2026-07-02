//! `DV_GENERAL_TIME_SPECIFICATION` — general-syntax timing specification.
//!
//! openEHR class: `DV_GENERAL_TIME_SPECIFICATION`, package
//! `rm.data_types.time_specification`.
//! Inherits: `DV_TIME_SPECIFICATION`.
//!
//! Specifies points in time in a general syntax. Based on the HL7v3 GTS
//! (General Timing Specification) data type.
//!
//! The published package overview states: "The class is the same
//! structurally as the `DV_TIME_SPECIFICATION` parent" — this class's own
//! per-class table indeed declares no attributes beyond the inherited
//! `value: DV_PARSABLE`, and no invariant of its own (contrast
//! `DV_PERIODIC_TIME_SPECIFICATION`'s `Value_valid`).
//!
//! # General Time Specification syntax
//!
//! ```text
//! general_time_spec = symbol | union | exclusion ;
//! union = intersection [";" union] ;
//! exclusion = exclusion "\" intersection ;
//! intersection = factor intersection | factor ;
//! hull = factor [".." hull] ;
//! factor = interval | phase_linked_time_spec | event_linked_time_spec | "(" general_time_spec ")" ;
//! ```
//!
//! `phase_linked_time_spec` and `event_linked_time_spec` are the same two
//! grammars documented on `dv_periodic_time_specification.rs`.
use crate::data_types::encapsulated::dv_parsable::DvParsable;
use crate::data_types::time_specification::dv_time_specification::DvTimeSpecification;

/// `DV_GENERAL_TIME_SPECIFICATION`.
///
/// openEHR class: `DV_GENERAL_TIME_SPECIFICATION`.
///
/// Declares no attributes of its own beyond the inherited `value:
/// DV_PARSABLE` from `DV_TIME_SPECIFICATION`, held directly here per the
/// same reasoning documented on `DvPeriodicTimeSpecification`.
#[derive(Debug, Clone, PartialEq)]
pub struct DvGeneralTimeSpecification {
    /// `value`: `DV_PARSABLE` (`1..1`), inherited from
    /// `DV_TIME_SPECIFICATION`.
    ///
    /// The specification, in the general HL7v3 GTS syntax (see module doc).
    pub value: DvParsable,
}

pub const TYPE_NAME: &str = "DV_GENERAL_TIME_SPECIFICATION";

impl DvTimeSpecification for DvGeneralTimeSpecification {
    fn value(&self) -> &DvParsable {
        &self.value
    }

    /// `calendar_alignment` `(): String` (effected).
    ///
    /// Calendar alignment extracted from value.
    ///
    /// TODO(port): requires a general HL7v3 GTS syntax parser (see module
    /// doc grammar); no parser exists yet, and unlike
    /// `DV_PERIODIC_TIME_SPECIFICATION`'s narrower `PIVL`/`EIVL` case, this
    /// class's syntax additionally permits full recursive
    /// union/intersection/exclusion/hull composition of nested time
    /// specifications, which is the harder of the two grammars in this
    /// package.
    fn calendar_alignment(&self) -> String {
        todo!(
            "DV_GENERAL_TIME_SPECIFICATION.calendar_alignment: requires a general HL7v3 GTS syntax parser, not yet designed"
        )
    }

    /// `event_alignment` `(): String` (effected).
    ///
    /// Event alignment extracted from value.
    ///
    /// TODO(port): see `calendar_alignment` above — same GTS syntax parsing
    /// gap.
    fn event_alignment(&self) -> String {
        todo!(
            "DV_GENERAL_TIME_SPECIFICATION.event_alignment: requires a general HL7v3 GTS syntax parser, not yet designed"
        )
    }

    /// `institution_specified` `(): Boolean` (effected).
    ///
    /// Extracted from value.
    ///
    /// TODO(port): see `calendar_alignment` above — same GTS syntax parsing
    /// gap; also flagged as underspecified in the source table beyond
    /// "extracted from value", same as the periodic sibling class.
    fn institution_specified(&self) -> bool {
        todo!(
            "DV_GENERAL_TIME_SPECIFICATION.institution_specified: extraction rule underspecified in the source table beyond \"extracted from value\"; requires a general HL7v3 GTS syntax parser, not yet designed"
        )
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.time_specification — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_general_time_specification.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-time_specification_package.adoc §Class Descriptions / dv_general_time_specification.adoc §DV_GENERAL_TIME_SPECIFICATION Class
//   confidence: medium
//   todos: 3
//   note: structurally identical to DV_TIME_SPECIFICATION per the package overview's own wording (no added attributes, no own invariant); all three effected functions require the (harder, recursive) general GTS syntax parser, not yet designed; same "extracted from value" underspecification flagged on institution_specified as its periodic sibling.
// ─────────────────────────────────────────────
