//! `Env` — real-world environment access (current date/time/timezone).
//!
//! openEHR class: `Env` (interface), package `base.base_types.builtins`.
//!
//! Class representing the real-world environment, providing basic
//! information like current time, date, etc.
// TODO(port): the four ISO 8601 temporal types this interface returns
// (`Iso8601_date`, `Iso8601_time`, `Iso8601_date_time`, `Iso8601_timezone`)
// belong to `base.foundation_types.time` and have not been transcribed into
// `openehr-foundation` yet. The `use` paths below name where they are
// expected to land per the crate layout (PORT_MASTER_PLAN.md Section 9);
// update once those files exist.
use openehr_foundation::time::iso8601_date::Iso8601Date;
use openehr_foundation::time::iso8601_date_time::Iso8601DateTime;
use openehr_foundation::time::iso8601_time::Iso8601Time;
use openehr_foundation::time::iso8601_timezone::Iso8601Timezone;

/// `Env` is a pure function interface (an openEHR "interface" class,
/// declaring functions but no attributes and no state), so it is
/// transcribed as a Rust trait, mirroring the `Any`/`Numeric` pattern in
/// `openehr-foundation::primitive_types` (ADR-001 §1) rather than as a
/// struct.
///
/// A caller reaches "the current environment" through some concrete `impl
/// Env` supplied at the call site (e.g. a system-clock implementation) —
/// the spec does not itself name a singleton accessor, so none is invented
/// here.
pub trait Env {
    /// `current_date` (): `Iso8601_date`.
    ///
    /// Return today's date in the current locale.
    ///
    /// TODO(port): no concrete `impl Env` exists yet in this crate; a
    /// system-clock-backed implementation is a later-phase concern (this
    /// trait only fixes the spec's interface shape).
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

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.builtins — docs/research/spec-cache/BASE-1.2.0/uml_classes/env.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master04-builtins_package.adoc §Class Definitions / env.adoc §Env Interface
//   confidence: medium
//   todos: 2
//   note: forward-references Iso8601_date/time/date_time/timezone, none of which are transcribed yet in openehr-foundation::time; trait has no impl (no concrete clock source specified by the spec itself).
// ─────────────────────────────────────────────
