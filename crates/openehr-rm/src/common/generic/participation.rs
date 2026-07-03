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
use openehr_foundation::serde_support::{TypeName, TypeTag};
use openehr_terminology::{
    OpenehrTerminologyGroupIdentifiers, TerminologyAccess, TerminologyCode, TerminologyService,
};
use serde::{Deserialize, Serialize};

use super::party_proxy::PartyProxy;

/// Canonical `_type` discriminator string for this class in serialized
/// form. Per ADR-001 refinements ("serde derives wait until P4"), a
/// `const` stands in for `#[serde(rename = ...)]` until serde lands as a
/// dependency of this crate.
pub const TYPE_NAME: &str = "PARTICIPATION";

/// `PARTICIPATION` declares no `Inherit` row in the spec table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Participation {
    /// Canonical `_type` discriminator (`"PARTICIPATION"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

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
    /// Checked by [`Participation::is_function_valid`] (ADR-003 d.8), which
    /// discriminates the [`DvText::Coded`] runtime case; P11 Validate-
    /// framework wiring is pending.
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
    /// Checked by [`Participation::is_mode_valid`] (ADR-003 d.8); P11
    /// Validate-framework wiring is pending.
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<DvInterval<DvDateTime>>,
}

impl TypeName for Participation {
    const NAME: &'static str = TYPE_NAME;
}

impl Participation {
    /// Invariant `Function_valid`:
    /// `function.generating_type.is_equal("DV_CODED_TEXT") implies
    /// terminology(Terminology_id_openehr).has_code_for_group_id(
    /// Group_id_participation_function, function.defining_code)`.
    ///
    /// Working method per ADR-003 decision 8. `function` is typed `DV_TEXT`
    /// (the wider supertype), but the invariant is an *implication* whose
    /// antecedent is `function.generating_type.is_equal("DV_CODED_TEXT")` —
    /// so a plain (non-coded) `DV_TEXT` function satisfies it vacuously. The
    /// runtime-type test is the [`DvText::Coded`] discriminant of the closed
    /// enum (ADR-001 §4); a coded function's `defining_code` is then checked
    /// against the openEHR "participation function" group.
    pub fn is_function_valid(&self, terminology: &TerminologyService) -> bool {
        match &self.function {
            DvText::Coded(coded) => {
                let defining_code = &coded.defining_code;
                terminology
                    .terminology(OpenehrTerminologyGroupIdentifiers::TERMINOLOGY_ID_OPENEHR)
                    .is_some_and(|access| {
                        access.has_code_for_group_id(
                            OpenehrTerminologyGroupIdentifiers::GROUP_ID_PARTICIPATION_FUNCTION,
                            &TerminologyCode::new(
                                defining_code.terminology_id.value(),
                                defining_code.code_string.clone(),
                            ),
                        )
                    })
            }
            DvText::Text { .. } => true,
        }
    }

    /// Invariant `Mode_valid`: `mode /= Void implies
    /// terminology(Terminology_id_openehr).has_code_for_group_id(
    /// Group_id_participation_mode, mode.defining_code)`.
    ///
    /// Working method per ADR-003 decision 8. `mode` is a `DV_CODED_TEXT`
    /// already, so no runtime-type test is needed; the antecedent is `mode
    /// /= Void`, so an absent `mode` is vacuously valid. A present `mode`'s
    /// `defining_code` is checked against the openEHR "participation mode"
    /// group.
    pub fn is_mode_valid(&self, terminology: &TerminologyService) -> bool {
        match &self.mode {
            Some(mode) => {
                let defining_code = &mode.defining_code;
                terminology
                    .terminology(OpenehrTerminologyGroupIdentifiers::TERMINOLOGY_ID_OPENEHR)
                    .is_some_and(|access| {
                        access.has_code_for_group_id(
                            OpenehrTerminologyGroupIdentifiers::GROUP_ID_PARTICIPATION_MODE,
                            &TerminologyCode::new(
                                defining_code.terminology_id.value(),
                                defining_code.code_string.clone(),
                            ),
                        )
                    })
            }
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::change_control::versioned_object::test_support::{coded, party_self};
    use crate::data_types::text::dv_text::DvTextData;

    fn participation(function: DvText, mode: Option<DvCodedText>) -> Participation {
        Participation {
            type_tag: TypeTag::new(),
            function,
            mode,
            performer: party_self(),
            time: None,
        }
    }

    fn plain_text(value: &str) -> DvText {
        DvText::Text {
            type_tag: TypeTag::new(),
            data: DvTextData {
                value: value.to_string(),
                hyperlink: None,
                formatting: None,
                mappings: None,
                language: None,
                encoding: None,
            },
        }
    }

    #[test]
    fn function_valid_checks_the_participation_function_group_when_coded() {
        let service = TerminologyService::bundled().expect("bundled terminology parses");
        // 253 = "unknown" in the openEHR "participation function" group.
        let ok = participation(DvText::Coded(coded("253", "unknown")), None);
        assert!(ok.is_function_valid(service));

        let bad = participation(DvText::Coded(coded("999999", "nope")), None);
        assert!(!bad.is_function_valid(service));
    }

    #[test]
    fn function_valid_is_vacuous_for_plain_text() {
        let service = TerminologyService::bundled().expect("bundled terminology parses");
        let p = participation(plain_text("assisting nurse"), None);
        assert!(p.is_function_valid(service));
    }

    #[test]
    fn mode_valid_checks_the_participation_mode_group_and_is_vacuous_when_absent() {
        let service = TerminologyService::bundled().expect("bundled terminology parses");

        // Absent mode: vacuously valid.
        let none = participation(plain_text("f"), None);
        assert!(none.is_mode_valid(service));

        // 193 = "not specified" in the openEHR "participation mode" group.
        let ok = participation(plain_text("f"), Some(coded("193", "not specified")));
        assert!(ok.is_mode_valid(service));

        let bad = participation(plain_text("f"), Some(coded("999999", "nope")));
        assert!(!bad.is_mode_valid(service));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.generic — docs/research/spec-cache/RM-1.1.0/uml_classes/participation.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master04-generic_package.adoc §Participation / uml_classes/participation.adoc §PARTICIPATION Class
//   confidence: high
//   todos: 0
//   note: Function_valid and Mode_valid now working methods (ADR-003 d.8) with spec-derived tests. Function_valid is the conditional DV_CODED_TEXT case (DvText::Coded checked against the openEHR "participation function" group; plain DvText::Text vacuously valid); Mode_valid checks a present DV_CODED_TEXT mode against the "participation mode" group, absent mode vacuously valid — both via &TerminologyService. Only remaining deferral is P11 Validate-framework wiring.
// ─────────────────────────────────────────────
