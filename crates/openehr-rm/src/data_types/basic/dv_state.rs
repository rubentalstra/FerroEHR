//! `DV_STATE` — state values driven by an archetype-defined state machine.
//!
//! openEHR class: `DV_STATE`, package `rm.data_types.basic`.
//! Inherits: `DATA_VALUE`.
//!
//! For representing state values which obey a defined state machine, such
//! as a variable representing the states of an instruction or care
//! process.
//!
//! `DV_STATE` is expressed as a `String` but its values are driven by
//! archetype-defined state machines. This provides a powerful way of
//! capturing stateful complex processes in simple data.
use crate::data_types::data_value::DataValueApi;
use crate::data_types::text::dv_coded_text::DvCodedText;

/// Canonical `_type` discriminator string for this class in serialized
/// form (ADR-001 Refinements: serde derives wait until P4).
pub const TYPE_NAME: &str = "DV_STATE";

/// `DV_STATE` is a leaf, non-abstract class with two attributes.
///
/// PORT NOTE: the spec's `value` attribute is declared of type
/// `DV_CODED_TEXT`, not the plain `String` the class overview prose
/// ("`DV_STATE` is expressed as a `String`...") might suggest — the prose is
/// describing the *underlying representation* of state names in general
/// terms, while the Attributes table is the binding signature. Transcribed
/// literally from the table: `value: DvCodedText`, embedding the sibling
/// `text` package class directly rather than widening to a bare `String`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DvState {
    /// `value`: `DV_CODED_TEXT` (`1..1`).
    ///
    /// The state name. State names are determined by a state/event table
    /// defined in archetypes, and coded using openEHR Terminology or local
    /// archetype terms, as specified by the archetype.
    pub value: DvCodedText,

    /// `is_terminal`: `Boolean` (`1..1`).
    ///
    /// Indicates whether this state is a terminal state, such as "aborted",
    /// "completed" etc. from which no further transitions are possible.
    pub is_terminal: bool,
}

impl DataValueApi for DvState {
    fn type_name(&self) -> &'static str {
        TYPE_NAME
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.basic — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_state.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-basic_package.adoc §Class Descriptions / dv_state.adoc §DV_STATE Class
//   confidence: high
//   todos: 0
//   note: `value` transcribed literally as DV_CODED_TEXT per the Attributes table, not widened to String despite the class overview's looser "expressed as a String" prose (flagged as a documentation-vs-table wording gap, not a defect); no invariants published.
// ─────────────────────────────────────────────
