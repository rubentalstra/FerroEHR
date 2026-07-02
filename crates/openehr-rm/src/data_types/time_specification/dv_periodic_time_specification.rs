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
use crate::data_types::encapsulated::dv_parsable::DvParsable;
use crate::data_types::time_specification::dv_time_specification::DvTimeSpecification;

/// `DV_PERIODIC_TIME_SPECIFICATION`.
///
/// openEHR class: `DV_PERIODIC_TIME_SPECIFICATION`.
///
/// Declares no attributes of its own beyond the inherited `value:
/// DV_PARSABLE` from `DV_TIME_SPECIFICATION` — held directly here per
/// ADR-001 §3 (there is no further parent state to compose, since
/// `DV_TIME_SPECIFICATION` was transcribed as a pure trait with no
/// embeddable struct).
#[derive(Debug, Clone, PartialEq)]
pub struct DvPeriodicTimeSpecification {
    /// `value`: `DV_PARSABLE` (`1..1`), inherited from
    /// `DV_TIME_SPECIFICATION`.
    ///
    /// The specification, in the HL7v3 syntax for `PIVL` or `EIVL` types.
    pub value: DvParsable,
}

pub const TYPE_NAME: &str = "DV_PERIODIC_TIME_SPECIFICATION";

impl DvPeriodicTimeSpecification {
    /// `period` `(): DV_DURATION`.
    ///
    /// The period of the repetition, computationally derived from the
    /// syntax representation. Extracted from the `value` attribute.
    ///
    /// TODO(port): requires parsing the HL7v3 `PIVL`/`EIVL` syntax's
    /// `difference` component (see the phase-linked syntax grammar in the
    /// module doc above) and constructing a `DV_DURATION`; no HL7v3 syntax
    /// parser exists yet in this crate.
    pub fn period(&self) -> crate::data_types::date_time::dv_duration::DvDuration {
        todo!(
            "DV_PERIODIC_TIME_SPECIFICATION.period: requires an HL7v3 PIVL/EIVL syntax parser, not yet designed"
        )
    }

    /// `Value_valid` invariant:
    /// `value.formalism.is_equal("HL7:PIVL") or value.formalism.is_equal("HL7:EIVL")`.
    ///
    /// TODO(port): `DvParsable::formalism` exists (see `dv_parsable.rs`);
    /// this check is a plain string-equality comparison once that field is
    /// populated, so it is not itself blocked on the syntax parser above —
    /// left as an explicit method rather than a `Validate` impl for
    /// consistency with the rest of this file's TODO-status members, but is
    /// the one member here that could be de-TODO'd without waiting on P17.
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
    /// Calendar alignment extracted from value.
    ///
    /// TODO(port): requires the phase-linked syntax's `alignment` term
    /// (`"@" alignment`, drawn from the HL7 `CalendarCycle` domain, e.g.
    /// `"DW"`, `"DM"`) — see the module doc's phase-linked grammar. No
    /// parser exists yet.
    fn calendar_alignment(&self) -> String {
        todo!(
            "DV_PERIODIC_TIME_SPECIFICATION.calendar_alignment: requires an HL7v3 PIVL alignment-term parser, not yet designed"
        )
    }

    /// `event_alignment` `(): String` (effected).
    ///
    /// Event alignment extracted from value.
    ///
    /// TODO(port): requires the event-linked syntax's `event` term (HL7
    /// domain `HL7::TimingEvent`, e.g. `"PC"`, `"HS"`, `"AC"`, `"ACD"`) —
    /// see the module doc's event-linked grammar. No parser exists yet.
    fn event_alignment(&self) -> String {
        todo!(
            "DV_PERIODIC_TIME_SPECIFICATION.event_alignment: requires an HL7v3 EIVL event-term parser, not yet designed"
        )
    }

    /// `institution_specified` `(): Boolean` (effected).
    ///
    /// Extracted from value.
    ///
    /// TODO(port): the spec's own description gives no further detail on
    /// how this Boolean is derived from the syntax beyond "extracted from
    /// value" — flagged as genuinely underspecified in the source table
    /// (contrast `calendar_alignment`/`event_alignment`, which at least
    /// name the specific grammar term they extract). Deferred alongside the
    /// HL7v3 syntax parser.
    fn institution_specified(&self) -> bool {
        todo!(
            "DV_PERIODIC_TIME_SPECIFICATION.institution_specified: extraction rule underspecified in the source table beyond \"extracted from value\"; requires an HL7v3 syntax parser, not yet designed"
        )
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.time_specification — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_periodic_time_specification.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-time_specification_package.adoc §Class Descriptions / dv_periodic_time_specification.adoc §DV_PERIODIC_TIME_SPECIFICATION Class
//   confidence: medium
//   todos: 4
//   note: period/calendar_alignment/event_alignment/institution_specified all require an HL7v3 PIVL/EIVL syntax parser that does not exist yet (a distinct grammar from the ISO-8601 jiff-bridging plan); institution_specified's derivation rule is flagged as genuinely underspecified in the published table beyond "extracted from value"; invariant_value_valid is fully implemented (plain string comparison, not blocked on the parser).
// ─────────────────────────────────────────────
