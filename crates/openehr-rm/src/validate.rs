//! RM-level validation glue (hand-written spec behaviour; preserved
//! across `openehr-codegen` regeneration — the generator does not emit or overwrite it, so
//! the generator's `declare_hand_written_modules` keeps it and `lib.rs`
//! auto-declares `pub mod validate;`).
//!
//! Two things live here:
//!
//! 1. **The allocation-free fast-path RM class-invariant check**
//!    ([`try_fast_validate`] → [`fast`]) over a live canonical-JSON node, plus
//!    the **shared invariant helpers** used by the sibling `*_impl.rs`
//!    behaviour files (the DV_AMOUNT / DV_QUANTIFIED accuracy + magnitude-status
//!    rules, the LOCATABLE `Archetype_node_id_valid` rule, ISO-8601 value
//!    checks). These are pure RM model semantics.
//! 2. The typed-dispatch tier that *deserializes* a node into its concrete RM
//!    type — the wire-boundary operation — lives in `openehr-its`
//!    (`openehr_its::rm_validate`), because it drives the native canonical-JSON
//!    codec (`from_json_value`), which is defined downstream in that crate. The
//!    `Validate` trait and the invariant impls (`*_impl.rs`) stay here as model
//!    semantics; the two-tier entry point `openehr_its::rm_validate::validate_rm_value`
//!    calls [`try_fast_validate`] then falls back to its typed dispatch.
//!
//! # Fidelity to the reference implementation (archie)
//!
//! The RM class invariants mirror openEHR's reference implementation
//! **archie** (`com.nedap.archie.rmobjectvalidator`). Archie runs each
//! `@Invariant`-annotated boolean method and, on failure, emits one uniform
//! message: `Invariant <Name> failed on type <RM_TYPE>`. We reproduce that
//! message verbatim (see [`invariant_failed`]) so a violation is identifiable
//! by archie's own invariant name.
//!
//! What we deliberately do **not** implement here (`// NOTE:`):
//! - **Terminology-bound invariants** (archie's `Language_valid`,
//!   `Encoding_valid`, `Category_validity`, `Setting_valid`, `Change_type_valid`,
//!   `Normal_status_validity`, `Media_type_valid`, `Current_state_valid`, …).
//!   `openehr-rm` has no `openehr-term` dependency; these belong to the
//! composition validator + terminology binding, which resolves
//!   codes against the openEHR terminology bundle.
//! - **archie's `ignored = true` invariants** (never executed by archie —
//!   implementing them would over-reject relative to the reference).
//! - **Cross-child recursion**: each `Validate` impl checks only its own class
//!   invariants; the composition validator recurses into children (and prefixes
//!   the absolute RM path onto each [`InvariantViolation`]).

use serde_json::Value;

pub use openehr_base::validate::{InvariantViolation, Validate};

mod fast;
/// The generated RM class-invariant cores (`openehr-codegen -- emit-validate`):
/// one `pub(crate) fn <name>_core` per mechanically-shaped invariant group, the
/// single source both the typed `Validate` impls and [`fast`] call. This is the
/// ONE hand-declared module for that `// @generated` file — the runtime helpers
/// the cores call (`invariant_failed`, the ISO-8601 validators, the dialect
/// predicates) stay hand-written below.
pub(crate) mod generated;

/// Run the allocation-free fast-path RM class-invariant check for a single
/// canonical-JSON node, dispatching on its `_type`. Returns `true` when the fast
/// path vouched for (fully handled) the node — nothing is appended on `false`.
///
/// This is the public seam the wire-boundary two-tier dispatcher
/// (`openehr_its::rm_validate::validate_rm_value`) calls before falling back to
/// the typed deserialize path. Kept here because the fast path is untyped
/// (walks `&serde_json::Value` against the generated RM model) and needs no
/// canonical-JSON codec — pure RM model semantics.
///
/// NOTE: no openEHR spec governs the fast path — it is our own performance
/// design; the *semantics* it realizes are exactly the RM class invariants of
/// the `*_impl.rs` siblings (see [`fast`]).
#[must_use]
pub fn try_fast_validate(ty: &str, value: &Value, out: &mut Vec<InvariantViolation>) -> bool {
    fast::try_validate(ty, value, out)
}

/// Build an archie-style class-invariant violation:
/// `"Invariant <name> failed on type <RM_TYPE>"` — the exact message the
/// reference implementation's `RMObjectValidator` emits for every invariant
/// failure. The path is left empty (the value itself); the composition
/// validator prefixes the absolute RM path.
#[must_use]
pub(crate) fn invariant_failed(name: &str, rm_type: &str) -> InvariantViolation {
    InvariantViolation::here(format!("Invariant {name} failed on type {rm_type}"))
}

/// `true` when a floating value denotes a whole number (archie `isInteger`).
#[must_use]
#[allow(clippy::float_cmp)] // exact-integrality test, mirrors archie's `x.floor() == x`
pub(crate) fn is_integral(v: f64) -> bool {
    v.is_finite() && v.floor() == v
}

// ── named runtime realizations of the BMM assertion-dialect predicates ────────
//
// These are the callable runtime helpers the assertion-dialect emitter maps its
// leaf predicates onto (the `plan::overrides` dialect table names each). They
// were previously inlined into the invariant cores below; extracting them under
// the BMM predicate spelling makes the emitter's future generated cores call one
// named runtime function per dialect predicate. Behaviour is identical to the
// former inline forms.

/// BASE/RM `valid_magnitude_status (s)`: `s` is one of `= < > <= >= ~` — the
/// DV_QUANTIFIED `Magnitude_status_valid` predicate
/// (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.dv_quantified.adoc`).
#[must_use]
pub(crate) fn valid_magnitude_status(s: &str) -> bool {
    matches!(s, "=" | "<" | ">" | "<=" | ">=" | "~")
}

/// RM `valid_percentage (v)`: `0 <= v <= 100` — the DV_AMOUNT `Accuracy_validity`
/// predicate for a percent-recorded accuracy
/// (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.dv_amount.adoc`).
#[must_use]
pub(crate) fn valid_percentage(v: f64) -> bool {
    (0.0..=100.0).contains(&v)
}

/// RM `valid_proportion_kind (k)`: `k` is one of the PROPORTION_KIND codes
/// `0..=4` (ratio, unitary, percent, fraction, integer_fraction) — the
/// DV_PROPORTION `Type_validity` predicate
/// (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.proportion_kind.adoc`).
#[must_use]
pub(crate) fn valid_proportion_kind(k: i32) -> bool {
    (0..=4).contains(&k)
}

// ── ISO-8601 value validation ────────────────────────────────────────────────
//
// NOTE: archie has no `@Invariant` for DV_DATE/DV_TIME/DV_DATE_TIME/
// DV_DURATION value well-formedness — it enforces it structurally by parsing
// `value` into a typed `java.time` object at construction. In our model the
// value is a `String`, so we express the same guarantee as an explicit RM class
// invariant (`Value_valid`). The forms accepted are the openEHR ISO-8601 subset
// (partial precision permitted; DV_DURATION permits a leading sign and a `W`
// designator mixed with others, per the openEHR deviation). Kept intentionally
// lenient: it rejects clearly-malformed values, not valid partial ones.

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn digits_n(s: &str, n: usize) -> bool {
    s.len() == n && all_digits(s)
}

fn in_range(s: &str, lo: u32, hi: u32) -> bool {
    s.len() == 2 && all_digits(s) && s.parse::<u32>().is_ok_and(|v| (lo..=hi).contains(&v))
}

/// `true` for a Gregorian leap year: divisible by 4, except centuries not
/// divisible by 400 (BASE `Time_definitions`; the calendar `days_in_month`
/// depends on it).
fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Calendar days in a given month of a given year — the `days_in_month (m, y)`
/// the BASE `Time_definitions.valid_day` postcondition dispatches through
/// (`docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.foundation_types.time_definitions.adoc`
/// lines 95–103). Returns `0` for a month outside `1..=12` (caller has already
/// range-checked the month, so that branch is defensive).
fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// A calendar-valid two-digit day for the 4-digit `y` / 2-digit `m` strings:
/// `d` is `01`..`days_in_month(m, y)` — the BASE `Iso8601_date` invariant
/// `Day_valid: not day_unknown implies valid_day (year, month, day)` with
/// `valid_day (y, m, d) = (d >= 1 and d <= days_in_month (m, y))`
/// (`org.openehr.base.foundation_types.iso8601_date.adoc` line 107;
/// `time_definitions.adoc` line 102). This is calendar-exact — it rejects
/// `2021-02-31`, `2021-04-31`, and `2021-02-29` (non-leap) while accepting
/// `2020-02-29` (leap).
fn valid_day(y: &str, m: &str, d: &str) -> bool {
    if d.len() != 2 || !all_digits(d) {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (y.parse::<u32>(), m.parse::<u32>(), d.parse::<u32>())
    else {
        return false;
    };
    (1..=days_in_month(year, month)).contains(&day)
}

/// A valid openEHR ISO-8601 date: `YYYY`, `YYYY-MM`, `YYYY-MM-DD`, or the
/// compact `YYYYMM` / `YYYYMMDD` forms. Day validity is **calendar-exact**
/// (month lengths + leap years) per BASE `Iso8601_date.Day_valid`, not a bare
/// `1..=31` range.
#[must_use]
pub(crate) fn is_valid_iso_date(s: &str) -> bool {
    if s.contains('-') {
        match s.split('-').collect::<Vec<_>>().as_slice() {
            [y] => digits_n(y, 4),
            [y, m] => digits_n(y, 4) && in_range(m, 1, 12),
            [y, m, d] => digits_n(y, 4) && in_range(m, 1, 12) && valid_day(y, m, d),
            _ => false,
        }
    } else {
        match s.len() {
            4 => all_digits(s),
            6 => all_digits(s) && in_range(&s[4..6], 1, 12),
            8 => {
                all_digits(s)
                    && in_range(&s[4..6], 1, 12)
                    && valid_day(&s[0..4], &s[4..6], &s[6..8])
            }
            _ => false,
        }
    }
}

fn is_valid_tz(tz: &str) -> bool {
    if tz.is_empty() || tz == "Z" {
        return true;
    }
    let Some(rest) = tz.strip_prefix(['+', '-']) else {
        return false;
    };
    // BASE `Iso8601_timezone` bounds are ASYMMETRIC (`iso8601_timezone.adoc`
    // Max_hour_valid / Min_hour_valid; `time_definitions.adoc`
    // Max_timezone_hour = 14, Min_timezone_hour = 12): `+` offsets go to
    // +14:00, `-` offsets only to -12:00 (reject `-13:00`).
    //
    // NOTE (corpus adjudication): the invariants literally require
    // `hour > 0` when signed, but the canonical corpus + CNF data sets carry
    // `+00:00`/`-00:00` UTC forms in 42 files — the corpus outranks the prose
    // reading, so hour 0 is accepted with either sign (≡ `Z`).
    let max_hour = if tz.starts_with('+') { 14 } else { 12 };
    if rest.contains(':') {
        matches!(rest.split(':').collect::<Vec<_>>().as_slice(),
            [h, m] if in_range(h, 0, max_hour) && in_range(m, 0, 59))
    } else {
        match rest.len() {
            2 => in_range(rest, 0, max_hour),
            4 => in_range(&rest[0..2], 0, max_hour) && in_range(&rest[2..4], 0, 59),
            _ => false,
        }
    }
}

fn is_valid_time_core(s: &str) -> bool {
    // optional fractional seconds after '.' or ','
    let (base, frac) = match s.split_once(['.', ',']) {
        Some((b, f)) => (b, Some(f)),
        None => (s, None),
    };
    if let Some(f) = frac
        && !all_digits(f)
    {
        return false;
    }
    if base.contains(':') {
        match base.split(':').collect::<Vec<_>>().as_slice() {
            [h] => in_range(h, 0, 23),
            [h, m] => in_range(h, 0, 23) && in_range(m, 0, 59),
            [h, m, sec] => in_range(h, 0, 23) && in_range(m, 0, 59) && in_range(sec, 0, 60),
            _ => false,
        }
    } else {
        match base.len() {
            2 => in_range(base, 0, 23),
            4 => in_range(&base[0..2], 0, 23) && in_range(&base[2..4], 0, 59),
            6 => {
                in_range(&base[0..2], 0, 23)
                    && in_range(&base[2..4], 0, 59)
                    && in_range(&base[4..6], 0, 60)
            }
            _ => false,
        }
    }
}

/// A valid openEHR ISO-8601 time: `HH`, `HH:MM`, `HH:MM:SS[.fff]` (and the
/// compact `HHMM` / `HHMMSS` forms), with an optional `Z` / `±HH[:MM]` timezone.
#[must_use]
pub(crate) fn is_valid_iso_time(s: &str) -> bool {
    // Split off a trailing timezone (`Z`, or a `+`/`-` offset that is not the
    // fractional separator). Scan from the end for `Z`/`+`/`-`.
    if let Some(stripped) = s.strip_suffix('Z') {
        return is_valid_time_core(stripped);
    }
    if let Some(pos) = s.rfind(['+', '-']) {
        return is_valid_time_core(&s[..pos]) && is_valid_tz(&s[pos..]);
    }
    is_valid_time_core(s)
}

/// A valid openEHR ISO-8601 date-time: a date, then (if a time component is
/// present) `T` and a time. A `T`-less value is accepted as a date-only partial.
#[must_use]
pub(crate) fn is_valid_iso_date_time(s: &str) -> bool {
    match s.split_once('T') {
        Some((date, time)) => is_valid_iso_date(date) && is_valid_iso_time(time),
        None => is_valid_iso_date(s),
    }
}

fn parse_duration_components(s: &str, allowed: &[u8], any: &mut bool) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let mut has_fraction = false;
        if i < bytes.len() && (bytes[i] == b'.' || bytes[i] == b',') {
            has_fraction = true;
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }
        if i == start || i >= bytes.len() {
            return false; // no number, or number without a designator
        }
        if !allowed.contains(&bytes[i]) {
            return false;
        }
        // BASE `master06-time_types.adoc` §Primitive Time Types: "in openEHR,
        // only fractional seconds are supported" — a decimal fraction is only
        // permitted on the seconds ('S') component, never on Y/M/W/D/H/M.
        if has_fraction && bytes[i] != b'S' {
            return false;
        }
        i += 1;
        *any = true;
    }
    true
}

/// A valid openEHR ISO-8601 duration: optional leading sign, `P`, then one or
/// more `nY nM nW nD` components and an optional `T nH nM nS` part (openEHR
/// permits the sign and a `W` designator mixed with the others).
#[must_use]
pub(crate) fn is_valid_iso_duration(s: &str) -> bool {
    let s = s.strip_prefix(['+', '-']).unwrap_or(s);
    let Some(rest) = s.strip_prefix('P') else {
        return false;
    };
    let (date_part, time_part) = match rest.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (rest, None),
    };
    let mut any = false;
    if !parse_duration_components(date_part, b"YMWD", &mut any) {
        return false;
    }
    if let Some(t) = time_part
        && (t.is_empty() || !parse_duration_components(t, b"HMS", &mut any))
    {
        return false;
    }
    any
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;

    #[test]
    fn iso_date_forms() {
        assert!(is_valid_iso_date("2021"));
        assert!(is_valid_iso_date("2021-05"));
        assert!(is_valid_iso_date("2021-05-17"));
        assert!(is_valid_iso_date("20210517"));
        assert!(!is_valid_iso_date("2021-13"));
        assert!(!is_valid_iso_date("2021-05-32"));
        assert!(!is_valid_iso_date("not-a-date"));
        assert!(!is_valid_iso_date(""));
    }

    /// BASE `Iso8601_date.Day_valid` (`valid_day = d <= days_in_month(m, y)`,
    /// `iso8601_date.adoc` line 107). Calendar-exact month lengths, both the
    /// extended (`YYYY-MM-DD`) and compact (`YYYYMMDD`) forms.
    #[test]
    fn iso_date_day_is_calendar_exact() {
        // 31-day months accept 31; 30-day months reject it.
        assert!(is_valid_iso_date("2021-01-31"));
        assert!(is_valid_iso_date("2021-12-31"));
        assert!(!is_valid_iso_date("2021-04-31")); // April has 30 days
        assert!(!is_valid_iso_date("2021-06-31"));
        assert!(!is_valid_iso_date("2021-09-31"));
        assert!(!is_valid_iso_date("2021-11-31"));
        assert!(is_valid_iso_date("2021-04-30"));

        // February: 28 in a common year, 29 in a leap year, never 30/31.
        assert!(!is_valid_iso_date("2021-02-31"));
        assert!(!is_valid_iso_date("2021-02-30"));
        assert!(!is_valid_iso_date("2021-02-29")); // 2021 is not a leap year
        assert!(is_valid_iso_date("2021-02-28"));
        assert!(is_valid_iso_date("2020-02-29")); // 2020 divisible by 4
        assert!(is_valid_iso_date("2000-02-29")); // 2000 divisible by 400
        assert!(!is_valid_iso_date("1900-02-29")); // 1900 century, not /400

        // Day 00 is never valid.
        assert!(!is_valid_iso_date("2021-05-00"));

        // Compact form is held to the same calendar rule.
        assert!(!is_valid_iso_date("20210431"));
        assert!(!is_valid_iso_date("20210229"));
        assert!(is_valid_iso_date("20200229"));
        assert!(is_valid_iso_date("20210131"));
    }

    #[test]
    fn iso_time_forms() {
        assert!(is_valid_iso_time("10"));
        assert!(is_valid_iso_time("10:30"));
        assert!(is_valid_iso_time("10:30:59"));
        assert!(is_valid_iso_time("10:30:59.250"));
        assert!(is_valid_iso_time("10:30:59Z"));
        assert!(is_valid_iso_time("10:30:59+01:00"));
        assert!(!is_valid_iso_time("25:00"));
        assert!(!is_valid_iso_time("10:61"));
        assert!(!is_valid_iso_time("abc"));
    }

    #[test]
    fn iso_date_time_forms() {
        assert!(is_valid_iso_date_time("2021-05-17T10:30:00"));
        assert!(is_valid_iso_date_time("2021-05-17T10:30:00+02:00"));
        assert!(is_valid_iso_date_time("2021-05-17"));
        assert!(!is_valid_iso_date_time("2021-05-17T99:00"));
        assert!(!is_valid_iso_date_time("nope"));
    }

    #[test]
    fn iso_duration_forms() {
        assert!(is_valid_iso_duration("P1Y"));
        assert!(is_valid_iso_duration("P1Y2M10D"));
        assert!(is_valid_iso_duration("PT2H30M"));
        assert!(is_valid_iso_duration("P1Y2M10DT2H30M"));
        assert!(is_valid_iso_duration("P2W"));
        assert!(is_valid_iso_duration("-P1D"));
        assert!(is_valid_iso_duration("PT0.5S"));
        assert!(!is_valid_iso_duration("P"));
        assert!(!is_valid_iso_duration("1Y"));
        assert!(!is_valid_iso_duration("P1X"));
        assert!(!is_valid_iso_duration("PT"));
    }

    /// BASE `master06-time_types.adoc` §Primitive Time Types: "in openEHR, only
    /// fractional seconds are supported" — a decimal fraction on any component
    /// other than seconds is invalid, even though the pattern of designators is
    /// otherwise well-formed.
    #[test]
    fn iso_duration_fraction_only_on_seconds() {
        // Fraction on seconds (period or comma) is the sole permitted case.
        assert!(is_valid_iso_duration("PT2H30M0.5S"));
        assert!(is_valid_iso_duration("PT0,5S"));
        // Fraction on any other component is rejected.
        assert!(!is_valid_iso_duration("P1Y3M4DT2.5H"));
        assert!(!is_valid_iso_duration("PT2H14.5M"));
        assert!(!is_valid_iso_duration("P1.5Y"));
        assert!(!is_valid_iso_duration("P1.5M"));
        assert!(!is_valid_iso_duration("P1.5W"));
        assert!(!is_valid_iso_duration("P1.5D"));
        assert!(!is_valid_iso_duration("PT1.5H"));
        assert!(!is_valid_iso_duration("PT2H14,5M"));
    }

    // NOTE: the `_type`-dispatch tests (fast/typed equivalence, corpus, mutation
    // battery) moved with the typed dispatcher to
    // `openehr-its/tests/rm_validation.rs`, where both the fast path
    // (`try_fast_validate`) and the typed path (`from_json_value`) are reachable.
    // The ISO-8601 helper tests below stay with the helpers they exercise.

    /// BASE `Iso8601_timezone`: `+` offsets reach +14:00, `-` offsets stop at
    /// -12:00; ±00:00 accepted per the corpus (see `is_valid_tz`).
    #[test]
    fn timezone_bounds_are_asymmetric() {
        assert!(is_valid_iso_time("10:00:00+14:00"));
        assert!(is_valid_iso_time("10:00:00-12:00"));
        assert!(is_valid_iso_time("10:00:00+00:00"));
        assert!(is_valid_iso_time("10:00:00-00:00"));
        assert!(
            !is_valid_iso_time("10:00:00+15:00"),
            "+15 exceeds Max_timezone_hour"
        );
        assert!(
            !is_valid_iso_time("10:00:00-13:00"),
            "-13 exceeds Min_timezone_hour"
        );
    }
}
