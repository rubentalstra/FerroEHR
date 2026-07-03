//! HL7v3 time-specification syntax subset used by the
//! `data_types.time_specification` package.
//!
//! PORT NOTE: this module is **not** an openEHR class. It implements exactly
//! the two parse specifications the RM 1.1.0 package chapter publishes
//! (`docs/research/spec-cache/RM-1.1.0/data_types/master08-time_specification_package.adoc`
//! §Syntaxes) — the *phase-linked* (HL7v3 `PIVL<TS>`) and *event-linked*
//! (HL7v3 `EIVL<TS>`) grammars — so that `DV_PERIODIC_TIME_SPECIFICATION`'s
//! and `DV_GENERAL_TIME_SPECIFICATION`'s spec-declared extraction functions
//! (`period()`, `calendar_alignment()`, `event_alignment()`,
//! `institution_specified()`) have something to extract *from*. It is
//! deliberately not a full HL7v3 GTS engine: the general GTS grammar's
//! recursive union/intersection/exclusion composition is out of scope (see
//! `dv_general_time_specification.rs` for the cited gap).
//!
//! The published grammars, verbatim:
//!
//! ```text
//! phase_linked_time_spec = pure_phase_linked_time_spec [ "IST" ] ;
//! pure_phase_linked_time_spec = phase [ "@" alignment ] ;
//! phase = interval "/" "(" difference ")" ;
//! alignment = "DW" | etc ;      (* terms from "HL7::CalendarCycle" domain *)
//! difference = ;                (* ISO 8601 for time difference *)
//! interval = "[" interval_spec "]" ;
//! interval_spec = ";" | ";" date_time | date_time ";" date_time | date_time ";" ;
//! date_time = (* ISO 8601 for date/time string yyyymmdd[hh[mm[ss]]] *) ;
//!
//! event_linked_time_spec = event | event offset ;
//! event = "AC" | "ACD" | etc ;  (* HL7 domain "HL7::TimingEvent" *)
//! offset = ( "+" | "-" ) dur_interval ;
//! dur_interval = ;              (* ISO 8601 for duration interval *)
//! ```
//!
//! PORT NOTE (difference/duration tokens): the grammar's `difference`
//! comment says "ISO 8601 for time difference", but every example the
//! chapter itself gives uses the HL7v3 physical-quantity shorthand instead
//! (`(7d)`, `(1mo)`, `[1h;1h]`, `[50min;1h]`). Both forms are therefore
//! accepted here: a literal ISO 8601 duration (`P…`/`PT…`) is passed
//! through verbatim, and the HL7 time-unit shorthand
//! (`a`/`mo`/`wk`/`d`/`h`/`min`/`s`) is normalised to its exact ISO 8601
//! duration equivalent (`1mo` → `P1M`, `50min` → `PT50M`, …) so callers
//! always receive an ISO 8601 duration string.
use std::sync::LazyLock;

use regex::Regex;

/// Errors from parsing a `PIVL`/`EIVL` textual time specification.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TimeSpecSyntaxError {
    /// The value matches neither the phase-linked nor the event-linked
    /// grammar.
    #[error("not a phase-linked (PIVL) or event-linked (EIVL) time specification: {0:?}")]
    UnrecognisedSyntax(String),
    /// A `difference`/`dur_interval` token is neither an ISO 8601 duration
    /// nor an HL7 time-unit shorthand.
    #[error("unrecognised duration token {0:?} (expected ISO 8601 `P…` or HL7 shorthand)")]
    UnrecognisedDuration(String),
}

/// A parsed phase-linked (`PIVL<TS>`) specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseLinkedTimeSpec {
    /// The interval's lower `date_time` bound (`yyyymmdd[hh[mm[ss]]]`),
    /// verbatim; `None` where the `interval_spec` omits it.
    pub interval_low: Option<String>,
    /// The interval's upper `date_time` bound, verbatim; `None` where the
    /// `interval_spec` omits it.
    pub interval_high: Option<String>,
    /// The `difference` (period of repetition), normalised to an ISO 8601
    /// duration string (see the module PORT NOTE).
    pub difference: String,
    /// The `@`-prefixed `alignment` term from the `HL7::CalendarCycle`
    /// domain (e.g. `DW`, `DM`), if present.
    pub alignment: Option<String>,
    /// True if the trailing `IST` (institution-specified time) flag is
    /// present.
    pub institution_specified: bool,
}

/// A parsed event-linked (`EIVL<TS>`) specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventLinkedTimeSpec {
    /// The `event` term from the `HL7::TimingEvent` domain (e.g. `PC`,
    /// `HS`, `AC`).
    pub event: String,
    /// The optional `( "+" | "-" ) dur_interval` offset.
    pub offset: Option<EventOffset>,
}

/// The `offset` production of the event-linked grammar: a signed
/// `[low;high]` duration interval relative to the event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventOffset {
    /// True for a `-` offset (before the event), false for `+` (after).
    pub before: bool,
    /// Lower duration bound, normalised to ISO 8601.
    pub low: String,
    /// Upper duration bound, normalised to ISO 8601.
    pub high: String,
}

/// Either of the two periodic time-specification syntaxes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeSpecSyntax {
    /// Phase-linked (`PIVL<TS>`).
    PhaseLinked(PhaseLinkedTimeSpec),
    /// Event-linked (`EIVL<TS>`).
    EventLinked(EventLinkedTimeSpec),
}

/// `HL7::TimingEvent` domain terms accepted for the `event` production.
/// The chapter's EBNF elides the full list (`"AC" | "ACD" | etc`); this is
/// the HL7v3 TimingEvent vocabulary the `etc` points at.
const TIMING_EVENTS: [&str; 18] = [
    "AC", "ACD", "ACM", "ACV", // ante cibus (before meal) family
    "C", "CD", "CM", "CV", // cibus (meal) family
    "HS", // hora somni (bedtime)
    "IC", "ICD", "ICM", "ICV", // inter cibus (between meals) family
    "PC", "PCD", "PCM", "PCV",  // post cibus (after meal) family
    "WAKE", // on waking
];

// interval_spec = ";" | ";" date_time | date_time ";" date_time | date_time ";"
// date_time = yyyymmdd[hh[mm[ss]]]
static PHASE_LINKED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"^\[(?P<low>\d{8}(?:\d{2}){0,3})?;(?P<high>\d{8}(?:\d{2}){0,3})?\]",
        r"/\((?P<difference>[^)]+)\)",
        r"(?:@(?P<alignment>[A-Z]{2,4}))?",
        r"(?P<ist>IST)?$",
    ))
    .expect("PHASE_LINKED regex is a hard-coded literal")
});

static EVENT_LINKED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<event>[A-Z]+)(?:(?P<sign>[+-])\[(?P<low>[^;\]]+);(?P<high>[^;\]]+)\])?$")
        .expect("EVENT_LINKED regex is a hard-coded literal")
});

// ISO 8601 duration (as accepted verbatim) or HL7 shorthand `<n><unit>`.
static ISO_DURATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[+-]?P(\d+(\.\d+)?[YMWD])*(T(\d+(\.\d+)?[HMS])+)?$")
        .expect("ISO_DURATION regex is a hard-coded literal")
});

static HL7_SHORTHAND: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<n>\d+(?:\.\d+)?)\s*(?P<unit>a|mo|wk|d|h|min|s)$")
        .expect("HL7_SHORTHAND regex is a hard-coded literal")
});

/// Normalises a `difference`/`dur_interval` token to an ISO 8601 duration
/// string (see the module PORT NOTE on the two accepted forms).
pub fn normalise_duration(token: &str) -> Result<String, TimeSpecSyntaxError> {
    let token = token.trim();
    if ISO_DURATION.is_match(token) && token.trim_start_matches(['+', '-']).len() > 1 {
        return Ok(token.to_string());
    }
    if let Some(caps) = HL7_SHORTHAND.captures(token) {
        let n = &caps["n"];
        let iso = match &caps["unit"] {
            "a" => format!("P{n}Y"),
            "mo" => format!("P{n}M"),
            "wk" => format!("P{n}W"),
            "d" => format!("P{n}D"),
            "h" => format!("PT{n}H"),
            "min" => format!("PT{n}M"),
            "s" => format!("PT{n}S"),
            // The alternation above is exhaustive over the regex's `unit`
            // group; anything else cannot match.
            _ => return Err(TimeSpecSyntaxError::UnrecognisedDuration(token.to_string())),
        };
        return Ok(iso);
    }
    Err(TimeSpecSyntaxError::UnrecognisedDuration(token.to_string()))
}

/// Parses a phase-linked (`PIVL`) or event-linked (`EIVL`) specification
/// per the two published grammars.
pub fn parse_time_spec(value: &str) -> Result<TimeSpecSyntax, TimeSpecSyntaxError> {
    let value = value.trim();
    if let Some(caps) = PHASE_LINKED.captures(value) {
        return Ok(TimeSpecSyntax::PhaseLinked(PhaseLinkedTimeSpec {
            interval_low: caps.name("low").map(|m| m.as_str().to_string()),
            interval_high: caps.name("high").map(|m| m.as_str().to_string()),
            difference: normalise_duration(&caps["difference"])?,
            alignment: caps.name("alignment").map(|m| m.as_str().to_string()),
            institution_specified: caps.name("ist").is_some(),
        }));
    }
    if let Some(caps) = EVENT_LINKED.captures(value) {
        let event = caps["event"].to_string();
        if TIMING_EVENTS.contains(&event.as_str()) {
            let offset = match (caps.name("sign"), caps.name("low"), caps.name("high")) {
                (Some(sign), Some(low), Some(high)) => Some(EventOffset {
                    before: sign.as_str() == "-",
                    low: normalise_duration(low.as_str())?,
                    high: normalise_duration(high.as_str())?,
                }),
                _ => None,
            };
            return Ok(TimeSpecSyntax::EventLinked(EventLinkedTimeSpec {
                event,
                offset,
            }));
        }
    }
    Err(TimeSpecSyntaxError::UnrecognisedSyntax(value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The chapter's own first example: `[200004181100;200004181110]/(7d)@DW`
    /// = every Tuesday from 11:00 to 11:10 AM.
    #[test]
    fn parses_the_published_weekly_pivl_example() {
        let parsed = parse_time_spec("[200004181100;200004181110]/(7d)@DW").unwrap();
        let TimeSpecSyntax::PhaseLinked(p) = parsed else {
            panic!("expected phase-linked");
        };
        assert_eq!(p.interval_low.as_deref(), Some("200004181100"));
        assert_eq!(p.interval_high.as_deref(), Some("200004181110"));
        assert_eq!(p.difference, "P7D");
        assert_eq!(p.alignment.as_deref(), Some("DW"));
        assert!(!p.institution_specified);
    }

    /// The chapter's second example: `[200004181100;200004181110]/(1mo)@DM`
    /// = every 18th of the month 11:00 to 11:10 AM.
    #[test]
    fn parses_the_published_monthly_pivl_example() {
        let parsed = parse_time_spec("[200004181100;200004181110]/(1mo)@DM").unwrap();
        let TimeSpecSyntax::PhaseLinked(p) = parsed else {
            panic!("expected phase-linked");
        };
        assert_eq!(p.difference, "P1M");
        assert_eq!(p.alignment.as_deref(), Some("DM"));
    }

    /// The grammar's optional trailing `IST` flag and the degenerate
    /// `interval_spec = ";"` production.
    #[test]
    fn parses_ist_flag_and_empty_interval() {
        // `PT6H`, not `P6H`: an hours component in ISO 8601 requires the
        // `T` date/time separator, so `P6H` is malformed and correctly
        // rejected by `normalise_duration`. This test exercises the
        // trailing `IST` flag and the degenerate `interval_spec = ";"`.
        let parsed = parse_time_spec("[;]/(PT6H)IST").unwrap();
        let TimeSpecSyntax::PhaseLinked(p) = parsed else {
            panic!("expected phase-linked");
        };
        assert_eq!(p.interval_low, None);
        assert_eq!(p.interval_high, None);
        assert_eq!(p.difference, "PT6H");
        assert_eq!(p.alignment, None);
        assert!(p.institution_specified);
    }

    /// ISO 8601 `difference` tokens pass through verbatim.
    #[test]
    fn accepts_iso_duration_difference() {
        let parsed = parse_time_spec("[20000418;]/(PT8H)").unwrap();
        let TimeSpecSyntax::PhaseLinked(p) = parsed else {
            panic!("expected phase-linked");
        };
        assert_eq!(p.difference, "PT8H");
        assert_eq!(p.interval_low.as_deref(), Some("20000418"));
        assert_eq!(p.interval_high, None);
    }

    /// The chapter's event-linked examples: `PC+[1h;1h]` (one hour after
    /// meal) and `HS-[50min;1h]` (one hour before bedtime for 10 minutes).
    #[test]
    fn parses_the_published_eivl_examples() {
        let parsed = parse_time_spec("PC+[1h;1h]").unwrap();
        let TimeSpecSyntax::EventLinked(e) = parsed else {
            panic!("expected event-linked");
        };
        assert_eq!(e.event, "PC");
        let offset = e.offset.unwrap();
        assert!(!offset.before);
        assert_eq!(offset.low, "PT1H");
        assert_eq!(offset.high, "PT1H");

        let parsed = parse_time_spec("HS-[50min;1h]").unwrap();
        let TimeSpecSyntax::EventLinked(e) = parsed else {
            panic!("expected event-linked");
        };
        assert_eq!(e.event, "HS");
        let offset = e.offset.unwrap();
        assert!(offset.before);
        assert_eq!(offset.low, "PT50M");
        assert_eq!(offset.high, "PT1H");
    }

    /// Bare event terms are valid (`event_linked_time_spec = event | ...`).
    #[test]
    fn parses_bare_event_term() {
        let parsed = parse_time_spec("WAKE").unwrap();
        assert_eq!(
            parsed,
            TimeSpecSyntax::EventLinked(EventLinkedTimeSpec {
                event: "WAKE".to_string(),
                offset: None,
            })
        );
    }

    /// Rejections: unknown event terms, malformed intervals, bad durations.
    #[test]
    fn rejects_malformed_specifications() {
        assert!(matches!(
            parse_time_spec("NOTANEVENT"),
            Err(TimeSpecSyntaxError::UnrecognisedSyntax(_))
        ));
        assert!(matches!(
            parse_time_spec("[2000;2001]/(7d)"),
            Err(TimeSpecSyntaxError::UnrecognisedSyntax(_))
        ));
        assert!(matches!(
            parse_time_spec("[;]/(7 fortnights)"),
            Err(TimeSpecSyntaxError::UnrecognisedDuration(_))
        ));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.time_specification §Syntaxes — docs/research/spec-cache/RM-1.1.0/data_types/master08-time_specification_package.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-time_specification_package.adoc §Syntaxes (phase-linked + event-linked EBNF)
//   confidence: medium
//   todos: 0
//   note: helper module, not a spec class — implements exactly the two published EBNF grammars (PIVL/EIVL) via regex + LazyLock; HL7 unit shorthand from the chapter's own examples normalised to ISO 8601 durations (PORT NOTE in module doc); the recursive GTS grammar is deliberately out of scope (cited on dv_general_time_specification.rs).
// ─────────────────────────────────────────────
