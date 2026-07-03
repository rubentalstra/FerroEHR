//! `Env` — real-world environment access (current date/time/timezone).
//!
//! openEHR class: `Env` (interface), package `base.base_types.builtins`.
//!
//! Class representing the real-world environment, providing basic
//! information like current time, date, etc.
use openehr_foundation::time::iso8601_date::Iso8601Date;
use openehr_foundation::time::iso8601_date_time::Iso8601DateTime;
use openehr_foundation::time::iso8601_time::Iso8601Time;
use openehr_foundation::time::iso8601_timezone::Iso8601Timezone;
use openehr_foundation::time::iso8601_type::Iso8601TypeCore;

/// `Env` is a pure function interface (an openEHR "interface" class,
/// declaring functions but no attributes and no state), so it is
/// transcribed as a Rust trait, mirroring the `Any`/`Numeric` pattern in
/// `openehr-foundation::primitive_types` (ADR-001 §1) rather than as a
/// struct.
///
/// A caller reaches "the current environment" through some concrete `impl
/// Env` supplied at the call site — the spec does not itself name a
/// singleton accessor, so none is invented here. [`SystemEnv`] is the
/// system-clock-backed implementation.
pub trait Env {
    /// `current_date` (): `Iso8601_date`.
    ///
    /// Return today's date in the current locale.
    fn current_date(&self) -> Iso8601Date;

    /// `current_time` (): `Iso8601_time`.
    ///
    /// Return current time in the current locale.
    fn current_time(&self) -> Iso8601Time;

    /// `current_date_time` (): `Iso8601_date_time`.
    ///
    /// Return current date/time in the current locale.
    fn current_date_time(&self) -> Iso8601DateTime;

    /// `current_time_zone` (): `Iso8601_timezone`.
    ///
    /// Return the timezone of the current locale.
    fn current_time_zone(&self) -> Iso8601Timezone;
}

/// System-clock implementation of [`Env`], reading the current instant in
/// the system's local time zone via `jiff::Zoned::now()`.
///
/// PORT NOTE: this concrete type is a Rust addition — the spec declares
/// only the `Env` interface and names no implementation or accessor for
/// "the current environment". "The current locale" is read as the process's
/// local time zone (jiff's system-zone lookup), which is the direct
/// analogue of what a JVM implementation would return from
/// `LocalDate.now()` and friends.
///
/// All values are produced in ISO 8601 *extended* form (`YYYY-MM-DD`,
/// `hh:mm:ss`, `Z`/`±hh:mm`), the canonical openEHR interchange preference.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SystemEnv;

impl SystemEnv {
    fn now() -> jiff::Zoned {
        jiff::Zoned::now()
    }

    fn timezone_value(now: &jiff::Zoned) -> String {
        let seconds = now.offset().seconds();
        if seconds == 0 {
            // `Z` is the ISO 8601 literal for UTC (offset +00:00).
            return "Z".to_string();
        }
        let sign = if seconds < 0 { '-' } else { '+' };
        let total_minutes = seconds.abs() / 60;
        format!(
            "{}{:02}:{:02}",
            sign,
            total_minutes / 60,
            total_minutes % 60
        )
    }

    fn date_value(now: &jiff::Zoned) -> String {
        format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day())
    }

    fn time_value(now: &jiff::Zoned) -> String {
        format!("{:02}:{:02}:{:02}", now.hour(), now.minute(), now.second())
    }
}

impl Env for SystemEnv {
    fn current_date(&self) -> Iso8601Date {
        Iso8601Date {
            core: Iso8601TypeCore {
                value: Self::date_value(&Self::now()),
            },
        }
    }

    fn current_time(&self) -> Iso8601Time {
        Iso8601Time {
            core: Iso8601TypeCore {
                value: Self::time_value(&Self::now()),
            },
        }
    }

    fn current_date_time(&self) -> Iso8601DateTime {
        let now = Self::now();
        Iso8601DateTime {
            core: Iso8601TypeCore {
                value: format!(
                    "{}T{}{}",
                    Self::date_value(&now),
                    Self::time_value(&now),
                    Self::timezone_value(&now)
                ),
            },
        }
    }

    fn current_time_zone(&self) -> Iso8601Timezone {
        Iso8601Timezone {
            core: Iso8601TypeCore {
                value: Self::timezone_value(&Self::now()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openehr_foundation::time::iso8601_parser::{
        parse_date, parse_date_time, parse_time, parse_timezone,
    };

    #[test]
    fn system_env_current_date_is_a_valid_full_precision_iso8601_date() {
        let date = SystemEnv.current_date();
        let parsed = parse_date(&date.core.value).expect("valid ISO 8601 date");
        assert!(parsed.month.is_some());
        assert!(parsed.day.is_some());
        assert!(date.year() >= 2026);
    }

    #[test]
    fn system_env_current_time_is_a_valid_full_precision_iso8601_time() {
        let time = SystemEnv.current_time();
        let parsed = parse_time(&time.core.value).expect("valid ISO 8601 time");
        assert!(parsed.minute.is_some());
        assert!(parsed.second.is_some());
    }

    #[test]
    fn system_env_current_date_time_parses_including_its_timezone() {
        let date_time = SystemEnv.current_date_time();
        assert!(parse_date_time(&date_time.core.value).is_some());
    }

    #[test]
    fn system_env_current_time_zone_is_a_valid_iso8601_timezone() {
        let timezone = SystemEnv.current_time_zone();
        let parsed = parse_timezone(&timezone.core.value).expect("valid ISO 8601 timezone");
        assert!((0..=14).contains(&parsed.hour));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.builtins — docs/research/spec-cache/BASE-1.2.0/uml_classes/env.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master04-builtins_package.adoc §Class Definitions / env.adoc §Env Interface
//   confidence: high
//   todos: 0
//   note: returns the real openehr-foundation Iso8601 types (former forward-reference TODO resolved); SystemEnv is a PORT-NOTEd Rust addition backed by jiff::Zoned::now(), emitting extended-form values.
// ─────────────────────────────────────────────
