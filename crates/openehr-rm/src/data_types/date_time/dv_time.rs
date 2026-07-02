//! `DV_TIME` — an absolute point in time from the start of the current day.
//!
//! openEHR class: `DV_TIME`, package `rm.data_types.quantity.date_time`.
//! Inherits: `DV_TEMPORAL`, `Iso8601_time`.
//!
//! Represents an absolute point in time from an origin usually interpreted
//! as meaning the start of the current day, specified to a fraction of a
//! second. Semantics defined by ISO 8601.
//!
//! Used for recording real world times, rather than scientifically measured
//! fine amounts of time. The partial form is used for approximate times of
//! events and substance administrations.
//!
//! # Dual inheritance
//!
//! Same shape as `DV_DATE` (see that file's module doc for the full
//! rationale): `DV_TEMPORAL` (RM abstract ancestor, embedded state + trait)
//! composed alongside `Iso8601_time` (BASE foundation-types mixin,
//! `openehr_foundation::time::iso8601_time`), both as fields/trait impls
//! rather than Rust inheritance.
use crate::data_types::date_time::dv_duration::DvDuration;
use crate::data_types::date_time::dv_temporal::{DvTemporal, DvTemporalData};

/// `DV_TIME`.
///
/// openEHR class: `DV_TIME`.
#[derive(Debug, Clone, PartialEq)]
pub struct DvTime {
    /// Embedded `DV_TEMPORAL` state.
    pub temporal: DvTemporalData,

    /// `value`: `String` (`1..1`, redefined).
    ///
    /// ISO8601 time string.
    ///
    /// TODO(port): `Value_valid` invariant (`valid_iso8601_time(value)`) not
    /// yet enforced; see `invariant_value_valid` below.
    pub value: String,
}

pub const TYPE_NAME: &str = "DV_TIME";

impl DvTime {
    /// `magnitude` `(): Real` (effected).
    ///
    /// Numeric value of the time as seconds since the start of day, i.e.
    /// `00:00:00`.
    ///
    /// Per `docs/ROSETTA.md`, spec `Real` is backed by `f64` in this
    /// codebase (the primitive_types transcription's directed deviation),
    /// consistent with every other `Real`-returning function in this
    /// package.
    ///
    /// TODO(port): requires parsing `value` as a (possibly partial) ISO 8601
    /// time and computing seconds-since-midnight — deferred to the
    /// jiff-backed engine at P17.
    pub fn magnitude(&self) -> f64 {
        todo!(
            "DV_TIME.magnitude: seconds-since-midnight, deferred to the jiff-backed engine at P17"
        )
    }

    /// `add` __alias__ `"+"` `(a_diff: DV_DURATION[1]): DV_TIME` (redefined
    /// via the `DvTemporal` impl below).
    ///
    /// `subtract`, `diff` likewise — see the `DvTemporal for DvTime` impl.

    /// `less_than` __alias__ `"<"` `(other: DV_TIME[1]): Boolean` (effected).
    ///
    /// `Post_result`: `Result = magnitude > other.magnitude` as published.
    ///
    /// PORT NOTE: same inverted-postcondition wording flagged on `DvDate`'s
    /// `less_than` (see that file for the full discrepancy note) — the
    /// published text reads backwards for a function named `less_than`.
    /// Transcribed with name-implied semantics for consistency with
    /// `DV_DURATION`'s internally-correct wording.
    pub fn less_than(&self, other: &Self) -> bool {
        self.magnitude() < other.magnitude()
    }

    /// `is_strictly_comparable_to` `(other: DV_TIME[1]): Boolean` (effected).
    ///
    /// True, for any two Times.
    pub fn is_strictly_comparable_to(&self, _other: &Self) -> bool {
        true
    }

    /// `Value_valid` invariant: `valid_iso8601_time(value)`.
    ///
    /// TODO(port): bridges to the foundation-types validity predicate once
    /// the jiff-backed ISO 8601 parsing engine lands (P17).
    pub fn invariant_value_valid(&self) -> bool {
        todo!(
            "DV_TIME.invariant_value_valid: valid_iso8601_time bridges to the jiff-backed engine at P17"
        )
    }
}

impl DvTemporal for DvTime {
    fn temporal_data(&self) -> &DvTemporalData {
        &self.temporal
    }

    /// `add` __alias__ `"+"` `(a_diff: DV_DURATION[1]): DV_TIME` (redefined).
    ///
    /// Addition of a Duration to this Time.
    ///
    /// TODO(port): ISO 8601 clock arithmetic, deferred to the jiff-backed
    /// engine at P17.
    fn add(&self, a_diff: &DvDuration) -> Self {
        let _ = a_diff;
        todo!("DV_TIME.add: ISO 8601 clock arithmetic deferred to the jiff-backed engine at P17")
    }

    /// `subtract` __alias__ `"-"` `(a_diff: DV_DURATION[1]): DV_TIME`
    /// (redefined).
    ///
    /// Subtract a Duration from this Time.
    ///
    /// TODO(port): see `add` above.
    fn subtract(&self, a_diff: &DvDuration) -> Self {
        let _ = a_diff;
        todo!(
            "DV_TIME.subtract: ISO 8601 clock arithmetic deferred to the jiff-backed engine at P17"
        )
    }

    /// `diff` __alias__ `"-"` `(other: DV_TIME[1]): DV_DURATION` (redefined).
    ///
    /// Difference between this Time and `other`.
    ///
    /// TODO(port): see `add` above.
    fn diff(&self, other: &Self) -> DvDuration {
        let _ = other;
        todo!("DV_TIME.diff: ISO 8601 clock arithmetic deferred to the jiff-backed engine at P17")
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.date_time — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_time.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master07-date_time_package.adoc §Class Descriptions / dv_time.adoc §DV_TIME Class
//   confidence: medium
//   todos: 5
//   note: same dual-inheritance shape as DV_DATE; magnitude/add/subtract/diff/invariant_value_valid deferred to the jiff-backed engine at P17; less_than transcribed with name-implied semantics against the same likely copy-paste Post_result defect flagged on DV_DATE.
// ─────────────────────────────────────────────
