//! RM-mandated default codes/values used by the FLAT/STRUCTURED reverse
//! converter, in one place so they cannot drift (F-10-09/F-10-14; the
//! `rm_version`/`DEFAULT_TIME`/setting slices of F-13-26).
//!
//! These are the values the converter fabricates to satisfy RM-mandatory fields
//! that the simplified format never surfaces. A value defined here never appears
//! in FLAT, so it does not affect the `flat ⇄ flat` round-trip.

/// openEHR reference-model release stamped into a rebuilt
/// `ARCHETYPED.rm_version`. The RM spec (`RM/.../archetyped.adoc`) defines it as
/// the "version of the openEHR reference model used to create this object";
/// since this system creates data against the pinned RM, it is tied to the
/// workspace RM pin (`docs/VERSIONS.md`: RM 1.2.0), not the legacy archie/EHRbase
/// `1.0.4` literal.
pub(crate) const RM_VERSION: &str = "1.2.0";

/// Deterministic fill for the RM-mandatory temporal fields FLAT never surfaces
/// as data (`HISTORY.origin`, `EVENT.time`, and the compacted structural nodes
/// re-materialised on FLAT→RM). These are never present in FLAT, so a fixed
/// value keeps the `flat ⇄ flat` round-trip stable (a fresh `now()` computed on
/// each conversion would make two successive `to_flat` runs disagree).
///
/// The SM `app_context.time` "current time" default (`SM/.../app_context.adoc`:
/// "If not specified current time will be used") applies to an unset `ctx/time`
/// (`EVENT_CONTEXT.start_time`) — that default is `now()`, applied in
/// [`super::context::apply_ctx`]. It is safe there because [`super::context::emit_ctx`]
/// always emits `ctx/time`, so a round-trip never re-materialises it from `now()`.
pub(crate) const DEFAULT_TIME: &str = "1970-01-01T00:00:00Z";

/// Default `EVENT_CONTEXT.setting` (openEHR terminology group "setting",
/// `openehr::238` "other care") — Better `ConversionContext.Builder` default.
pub(crate) const DEFAULT_SETTING_CODE: &str = "238";
pub(crate) const DEFAULT_SETTING_VALUE: &str = "other care";
pub(crate) const DEFAULT_SETTING_TERM: &str = "openehr";
