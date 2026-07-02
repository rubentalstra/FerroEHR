//! `PARTICIPATION` — model of a party's participation in an activity.
//!
//! openEHR class: `PARTICIPATION` (concrete), package `common.generic`.
//!
//! Model of a participation of a Party (any Actor or Role) in an activity.
//! Used to represent any participation of a Party in some activity, which
//! is not explicitly in the model, e.g. assisting nurse. Can be used to
//! record past or future participations.
//!
//! Should not be used in place of more permanent relationships between
//! demographic entities.
//!
//! The Participation abstraction models the interaction of some Party in
//! an activity. In the openEHR reference models, participations are
//! actually modelled in two ways. In situations where the kinds of
//! participation are known and constant, they are modelled as a named
//! attribute in the relevant reference model — for example, the
//! `committer: PARTY_PROXY` attribute in `AUDIT_DETAILS` models a
//! participation in which the function is "committal". Where the kind of
//! participation is not known at design time, a descendant of the generic
//! `PARTICIPATION` class is used.

// TODO(port): `DV_TEXT`, `DV_CODED_TEXT`, `DV_INTERVAL<T>`, `DV_DATE_TIME`
// are RM 1.1.0 `data_types.text`/`data_types.quantity`/`data_types.date_time`,
// transcribed by a sibling agent in this same phase but not yet landed in
// this worktree. Forward-references to their eventual module paths.
use crate::data_types::date_time::dv_date_time::DvDateTime;
use crate::data_types::quantity::dv_interval::DvInterval;
use crate::data_types::text::dv_coded_text::DvCodedText;
use crate::data_types::text::dv_text::DvText;

use super::party_proxy::PartyProxy;

/// Canonical `_type` discriminator string for this class in serialized
/// form. Per ADR-001 refinements ("serde derives wait until P4"), a
/// `const` stands in for `#[serde(rename = ...)]` until serde lands as a
/// dependency of this crate.
pub const TYPE_NAME: &str = "PARTICIPATION";

/// `PARTICIPATION` declares no `Inherit` row in the spec table.
#[derive(Debug, Clone, PartialEq)]
pub struct Participation {
    /// `function`: `DV_TEXT`, cardinality `1..1`.
    ///
    /// The function of the Party in this participation (note that a given
    /// party might participate in more than one way in a particular
    /// activity). This attribute should be coded, but cannot be limited to
    /// the HL7v3:ParticipationFunction vocabulary, since it is too limited
    /// and hospital-oriented.
    ///
    /// Invariant `Function_valid`:
    /// `function.generating_type.is_equal("DV_CODED_TEXT") implies
    /// terminology(Terminology_id_openehr).has_code_for_group_id(
    /// Group_id_participation_function, function.defining_code)`.
    ///
    /// TODO(port): invariant requires a live `TerminologyService`; not yet
    /// enforced. See [`Participation::is_function_valid`].
    pub function: DvText,

    /// `mode`: `DV_CODED_TEXT`, cardinality `0..1`.
    ///
    /// Optional field for recording the "mode" of the performer / activity
    /// interaction, e.g. present, by telephone, by email etc.
    ///
    /// Invariant `Mode_valid`: `mode /= Void implies
    /// terminology(Terminology_id_openehr).has_code_for_group_id(
    /// Group_id_participation_mode, mode.defining_code)`.
    ///
    /// TODO(port): invariant requires a live `TerminologyService`; not yet
    /// enforced. See [`Participation::is_mode_valid`].
    pub mode: Option<DvCodedText>,

    /// `performer`: `PARTY_PROXY`, cardinality `1..1`.
    ///
    /// The id and possibly demographic system link of the party
    /// participating in the activity.
    pub performer: PartyProxy,

    /// `time`: `DV_INTERVAL<DV_DATE_TIME>`, cardinality `0..1`.
    ///
    /// The time interval during which the participation took place, if it
    /// is used in an observational context (i.e. recording facts about
    /// the past); or the intended time interval of the participation when
    /// used in future contexts, such as EHR Instructions.
    pub time: Option<DvInterval<DvDateTime>>,
}

impl Participation {
    /// Invariant `Function_valid`:
    /// `function.generating_type.is_equal("DV_CODED_TEXT") implies
    /// terminology(Terminology_id_openehr).has_code_for_group_id(
    /// Group_id_participation_function, function.defining_code)`.
    ///
    /// TODO(port): `function` is typed `DV_TEXT` (the wider supertype),
    /// but the invariant only constrains it when the runtime value happens
    /// to be a `DV_CODED_TEXT` (`generating_type.is_equal("DV_CODED_TEXT")`)
    /// — i.e. this is a conditional invariant over a value that may or may
    /// not actually carry a terminology-bound code at all. Requires both
    /// runtime-type inspection of `DV_TEXT` (a `DvText`/`DvCodedText`
    /// closed-enum discriminant, per ADR-001 §4, once `data_types.text` is
    /// transcribed) and a live `TerminologyService`; left as `todo!()`
    /// rather than a bare boolean stub since neither prerequisite exists
    /// yet.
    pub fn is_function_valid(
        &self,
        _terminology: &openehr_terminology::TerminologyService,
    ) -> bool {
        todo!(
            "Participation::is_function_valid: needs DV_TEXT runtime-type discrimination plus TerminologyService.has_code_for_group_id against Group_id_participation_function"
        )
    }

    /// Invariant `Mode_valid`: `mode /= Void implies
    /// terminology(Terminology_id_openehr).has_code_for_group_id(
    /// Group_id_participation_mode, mode.defining_code)`.
    ///
    /// TODO(port): requires a live `TerminologyService` to check
    /// `mode.defining_code` against the "participation mode" openEHR
    /// Terminology group; left as `todo!()`.
    pub fn is_mode_valid(&self, _terminology: &openehr_terminology::TerminologyService) -> bool {
        todo!(
            "Participation::is_mode_valid: needs TerminologyService.has_code_for_group_id against Group_id_participation_mode"
        )
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.generic — docs/research/spec-cache/RM-1.1.0/uml_classes/participation.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master04-generic_package.adoc §Participation / uml_classes/participation.adoc §PARTICIPATION Class
//   confidence: high
//   todos: 2
//   note: Function_valid and Mode_valid invariants left as todo!()-bodied methods (need live TerminologyService, and Function_valid additionally needs DV_TEXT runtime-type discrimination not yet available). Forward-refs DvText, DvCodedText, DvInterval<T>, DvDateTime (data_types, sibling-agent territory, not yet landed).
// ─────────────────────────────────────────────
