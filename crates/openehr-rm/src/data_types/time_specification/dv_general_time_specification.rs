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
use crate::data_types::time_specification::hl7v3_syntax::{self, TimeSpecSyntax};
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `DV_GENERAL_TIME_SPECIFICATION`.
///
/// openEHR class: `DV_GENERAL_TIME_SPECIFICATION`.
///
/// Declares no attributes of its own beyond the inherited `value:
/// DV_PARSABLE` from `DV_TIME_SPECIFICATION`, held directly here per the
/// same reasoning documented on `DvPeriodicTimeSpecification`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvGeneralTimeSpecification {
    /// Canonical `_type` discriminator (`"DV_GENERAL_TIME_SPECIFICATION"`),
    /// always serialized first; tolerated-absent and validated-if-present on
    /// input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `value`: `DV_PARSABLE` (`1..1`), inherited from
    /// `DV_TIME_SPECIFICATION`.
    ///
    /// The specification, in the general HL7v3 GTS syntax (see module doc).
    pub value: DvParsable,
}

pub const TYPE_NAME: &str = "DV_GENERAL_TIME_SPECIFICATION";

impl TypeName for DvGeneralTimeSpecification {
    const NAME: &'static str = TYPE_NAME;
}

impl DvGeneralTimeSpecification {
    /// Attempts to read `value.value` as a *single* GTS `factor` — a bare
    /// phase-linked or event-linked specification, optionally wrapped in
    /// one level of `( … )` per the `factor` production.
    ///
    /// PORT NOTE: helper beyond the spec's own function list. A GTS value
    /// that is a genuine union/intersection/exclusion of several factors
    /// returns `None` here — see the TODO(port) on `calendar_alignment`
    /// for why the three extraction functions cannot be defined over such
    /// compounds without inventing semantics.
    #[must_use]
    fn single_factor(&self) -> Option<TimeSpecSyntax> {
        let text = self.value.value.trim();
        let text = text
            .strip_prefix('(')
            .and_then(|rest| rest.strip_suffix(')'))
            .unwrap_or(text);
        hl7v3_syntax::parse_time_spec(text).ok()
    }
}

impl DvTimeSpecification for DvGeneralTimeSpecification {
    fn value(&self) -> &DvParsable {
        &self.value
    }

    /// `calendar_alignment` `(): String` (effected).
    ///
    /// Calendar alignment extracted from value. Implemented for the
    /// degenerate-but-common GTS case where the value is a single
    /// phase-linked/event-linked `factor`; "Empty if not aligned" (parent
    /// class description) covers the rest.
    ///
    /// TODO(port): published-spec gap — for a compound GTS value (a
    /// `union`/`intersection`/`exclusion` of several factors, each
    /// potentially carrying its *own* alignment) the table gives no rule
    /// for combining multiple alignments into this function's single
    /// `String` return. Rather than invent one, compound values yield the
    /// "not aligned" empty string; revisit if a later spec release defines
    /// the combination.
    fn calendar_alignment(&self) -> String {
        match self.single_factor() {
            Some(TimeSpecSyntax::PhaseLinked(p)) => p.alignment.unwrap_or_default(),
            _ => String::new(),
        }
    }

    /// `event_alignment` `(): String` (effected).
    ///
    /// Event alignment extracted from value — single-factor case only.
    ///
    /// TODO(port): same compound-GTS combination gap as
    /// `calendar_alignment` above.
    fn event_alignment(&self) -> String {
        match self.single_factor() {
            Some(TimeSpecSyntax::EventLinked(e)) => e.event,
            _ => String::new(),
        }
    }

    /// `institution_specified` `(): Boolean` (effected).
    ///
    /// Extracted from value — the phase-linked grammar's `IST` token, over
    /// the single-factor case (see the PORT NOTE on the periodic sibling's
    /// identically-derived function).
    ///
    /// TODO(port): same compound-GTS combination gap as
    /// `calendar_alignment` above (does one `IST` factor make the whole
    /// union institution-specified? — undefined); compound values yield
    /// `false`.
    fn institution_specified(&self) -> bool {
        match self.single_factor() {
            Some(TimeSpecSyntax::PhaseLinked(p)) => p.institution_specified,
            _ => false,
        }
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.time_specification — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_general_time_specification.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-time_specification_package.adoc §Class Descriptions / dv_general_time_specification.adoc §DV_GENERAL_TIME_SPECIFICATION Class
//   confidence: medium
//   todos: 3
//   note: structurally identical to DV_TIME_SPECIFICATION per the package overview's own wording (no added attributes, no own invariant); all three effected functions require the (harder, recursive) general GTS syntax parser, not yet designed; same "extracted from value" underspecification flagged on institution_specified as its periodic sibling. P4: Serialize/Deserialize added; `value` is mandatory, no skip needed; ADR-002 self-tagging applied (TypeTag<Self> first field + TypeName from TYPE_NAME) — the tag is the sole wire-level discriminator vs the structure-identical DV_PERIODIC_TIME_SPECIFICATION.
// ─────────────────────────────────────────────
