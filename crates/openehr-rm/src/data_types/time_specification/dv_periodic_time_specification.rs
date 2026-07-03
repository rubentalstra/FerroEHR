//! `DV_PERIODIC_TIME_SPECIFICATION` — periodic (phase-linked or event-linked)
//! points in time.
//!
//! openEHR class: `DV_PERIODIC_TIME_SPECIFICATION`, package
//! `rm.data_types.time_specification`.
//! Inherits: `DV_TIME_SPECIFICATION`.
//!
//! Specifies periodic points in time, linked to the calendar (phase-linked),
//! or a real world repeating event, such as "breakfast" (event-linked).
//! Based on the HL7v3 data types `PIVL<T>` and `EIVL<T>`.
//!
//! Used in therapeutic prescriptions, expressed as `INSTRUCTION`s in the
//! openEHR model.
//!
//! # Phase-linked syntax (HL7v3 `PIVL<T>`)
//!
//! ```text
//! "[" interval "]" "/" "(" difference ")" [ "@" alignment ] [ "IST" ]
//! ```
//!
//! Examples:
//! * `[200004181100;200004181110]/(7d)@DW` = every Tuesday from 11:00 to
//!   11:10 AM.
//! * `[200004181100;200004181110]/(1mo)@DM` = every 18th of the month 11:00
//!   to 11:10 AM.
//!
//! # Event-linked syntax (HL7v3 `EIVL<T>`)
//!
//! Examples:
//! * `PC+[1h;1h]` = one hour after meal.
//! * `HS-[50min;1h]` = one hour before bedtime for 10 minutes.
//!
//! See
//! `docs/research/spec-cache/RM-1.1.0/data_types/master08-time_specification_package.adoc`
//! for the full EBNF grammar of both syntaxes.
use crate::data_types::date_time::dv_duration::DvDuration;
use crate::data_types::encapsulated::dv_parsable::DvParsable;
use crate::data_types::time_specification::dv_time_specification::DvTimeSpecification;
use crate::data_types::time_specification::hl7v3_syntax::{
    self, TimeSpecSyntax, TimeSpecSyntaxError,
};
use openehr_foundation::serde_support::{TypeName, TypeTag};
use openehr_foundation::time::iso8601_duration::Iso8601Duration;
use openehr_foundation::time::iso8601_type::Iso8601TypeCore;
use serde::{Deserialize, Serialize};

/// `DV_PERIODIC_TIME_SPECIFICATION`.
///
/// openEHR class: `DV_PERIODIC_TIME_SPECIFICATION`.
///
/// Declares no attributes of its own beyond the inherited `value:
/// DV_PARSABLE` from `DV_TIME_SPECIFICATION` — held directly here per
/// ADR-001 §3 (there is no further parent state to compose, since
/// `DV_TIME_SPECIFICATION` was transcribed as a pure trait with no
/// embeddable struct).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvPeriodicTimeSpecification {
    /// Canonical `_type` discriminator (`"DV_PERIODIC_TIME_SPECIFICATION"`),
    /// always serialized first; tolerated-absent and validated-if-present on
    /// input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `value`: `DV_PARSABLE` (`1..1`), inherited from
    /// `DV_TIME_SPECIFICATION`.
    ///
    /// The specification, in the HL7v3 syntax for `PIVL` or `EIVL` types.
    pub value: DvParsable,
}

pub const TYPE_NAME: &str = "DV_PERIODIC_TIME_SPECIFICATION";

impl TypeName for DvPeriodicTimeSpecification {
    const NAME: &'static str = TYPE_NAME;
}

impl DvPeriodicTimeSpecification {
    /// Parses `value.value` against the two published grammars (phase-linked
    /// `PIVL`, event-linked `EIVL`).
    ///
    /// PORT NOTE: helper beyond the spec's own function list — the single
    /// parse the three spec-declared extraction functions below share.
    ///
    /// # Errors
    ///
    /// [`TimeSpecSyntaxError`] where `value.value` matches neither grammar.
    pub fn parsed(&self) -> Result<TimeSpecSyntax, TimeSpecSyntaxError> {
        hl7v3_syntax::parse_time_spec(&self.value.value)
    }

    /// `period` `(): DV_DURATION`.
    ///
    /// The period of the repetition, computationally derived from the
    /// syntax representation. Extracted from the `value` attribute — the
    /// phase-linked grammar's `difference` term.
    ///
    /// PORT NOTE: returns `Option` rather than the spec's bare
    /// `DV_DURATION` (the `TerminologyService` Option-instead-of-contract-
    /// violation precedent): the event-linked (`EIVL`) grammar declares
    /// **no** period/difference term at all, so a period is genuinely
    /// undefined for an event-linked value — as it is for an unparseable
    /// one. This is the published table's own gap (it declares `period()`
    /// `1..1` on a class whose `Value_valid` invariant admits `HL7:EIVL`
    /// values), flagged here rather than silently inventing a zero period.
    #[must_use]
    pub fn period(&self) -> Option<DvDuration> {
        match self.parsed().ok()? {
            TimeSpecSyntax::PhaseLinked(p) => Some(DvDuration {
                type_tag: TypeTag::new(),
                accuracy_is_percent: None,
                accuracy: None,
                iso8601: Iso8601Duration {
                    core: Iso8601TypeCore {
                        value: p.difference,
                    },
                },
            }),
            TimeSpecSyntax::EventLinked(_) => None,
        }
    }

    /// `Value_valid` invariant:
    /// `value.formalism.is_equal("HL7:PIVL") or value.formalism.is_equal("HL7:EIVL")`.
    pub fn invariant_value_valid(&self) -> bool {
        self.value.formalism == "HL7:PIVL" || self.value.formalism == "HL7:EIVL"
    }
}

impl DvTimeSpecification for DvPeriodicTimeSpecification {
    fn value(&self) -> &DvParsable {
        &self.value
    }

    /// `calendar_alignment` `(): String` (effected).
    ///
    /// Calendar alignment extracted from value — the phase-linked grammar's
    /// `"@" alignment` term (HL7 `CalendarCycle` domain, e.g. `"DW"`,
    /// `"DM"`). Per the parent class's description, "Empty if not aligned"
    /// — which covers event-linked values (whose grammar has no alignment
    /// term) and unparseable values alike.
    fn calendar_alignment(&self) -> String {
        match self.parsed() {
            Ok(TimeSpecSyntax::PhaseLinked(p)) => p.alignment.unwrap_or_default(),
            _ => String::new(),
        }
    }

    /// `event_alignment` `(): String` (effected).
    ///
    /// Event alignment extracted from value — the event-linked grammar's
    /// `event` term (HL7 `HL7::TimingEvent` domain, e.g. `"PC"`, `"HS"`,
    /// `"AC"`). Empty for phase-linked (no event term) and unparseable
    /// values.
    fn event_alignment(&self) -> String {
        match self.parsed() {
            Ok(TimeSpecSyntax::EventLinked(e)) => e.event,
            _ => String::new(),
        }
    }

    /// `institution_specified` `(): Boolean` (effected).
    ///
    /// Extracted from value.
    ///
    /// PORT NOTE: the published table says no more than "extracted from
    /// value", but the phase-linked grammar's optional trailing `IST`
    /// token (HL7v3 `PIVL.institutionSpecified`, "institution-specified
    /// time") is the one term in either grammar this function's name can
    /// refer to — transcribed as its presence. The event-linked grammar
    /// has no such term, so event-linked values yield `false`.
    fn institution_specified(&self) -> bool {
        match self.parsed() {
            Ok(TimeSpecSyntax::PhaseLinked(p)) => p.institution_specified,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::encapsulated::dv_encapsulated::DvEncapsulatedData;

    fn spec(formalism: &str, value: &str) -> DvPeriodicTimeSpecification {
        DvPeriodicTimeSpecification {
            type_tag: TypeTag::new(),
            value: DvParsable {
                type_tag: TypeTag::new(),
                encapsulated: DvEncapsulatedData {
                    charset: None,
                    language: None,
                },
                value: value.to_string(),
                formalism: formalism.to_string(),
            },
        }
    }

    /// `period()` extracts the phase-linked `difference` term as a
    /// `DV_DURATION` — the chapter's own `(7d)` example normalises to
    /// `P7D`.
    #[test]
    fn period_extracts_the_pivl_difference() {
        let s = spec("HL7:PIVL", "[200004181100;200004181110]/(7d)@DW");
        let period = s.period().expect("phase-linked value has a period");
        assert_eq!(period.iso8601.core.value, "P7D");
        // And in seconds, exactly seven days.
        assert!((period.magnitude() - 7.0 * 86_400.0).abs() < f64::EPSILON);
    }

    /// Event-linked values have no period term (published-table gap — see
    /// the `period()` PORT NOTE).
    #[test]
    fn period_is_undefined_for_eivl_values() {
        assert_eq!(spec("HL7:EIVL", "PC+[1h;1h]").period(), None);
    }

    /// The three effected extraction functions, over both grammars.
    #[test]
    fn effected_extractions_cover_both_grammars() {
        let phase = spec("HL7:PIVL", "[200004181100;200004181110]/(1mo)@DMIST");
        assert_eq!(phase.calendar_alignment(), "DM");
        assert_eq!(phase.event_alignment(), "");
        assert!(phase.institution_specified());

        let event = spec("HL7:EIVL", "HS-[50min;1h]");
        assert_eq!(event.calendar_alignment(), "");
        assert_eq!(event.event_alignment(), "HS");
        assert!(!event.institution_specified());
    }

    /// `Value_valid`: formalism must be `HL7:PIVL` or `HL7:EIVL`.
    #[test]
    fn invariant_value_valid_checks_the_formalism() {
        assert!(spec("HL7:PIVL", "[;]/(7d)").invariant_value_valid());
        assert!(spec("HL7:EIVL", "PC").invariant_value_valid());
        assert!(!spec("HL7:GTS", "PC").invariant_value_valid());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.time_specification — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_periodic_time_specification.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-time_specification_package.adoc §Class Descriptions / dv_periodic_time_specification.adoc §DV_PERIODIC_TIME_SPECIFICATION Class
//   confidence: high
//   todos: 0
//   note: period/calendar_alignment/event_alignment/institution_specified implemented over the hl7v3_syntax module (the two published EBNF grammars); period() returns Option per the flagged published-table gap (EIVL grammar has no difference term — PORT NOTE on the method); institution_specified reads the phase-linked grammar's IST token (PORT NOTE); invariant_value_valid is a plain formalism check. P4: Serialize/Deserialize added; `value` is mandatory, no skip needed; ADR-002 self-tagging applied (TypeTag<Self> first field + TypeName from TYPE_NAME) — the tag is the sole wire-level discriminator vs the structure-identical DV_GENERAL_TIME_SPECIFICATION.
// ─────────────────────────────────────────────
