//! `DV_DATE_TIME` — an absolute point in time, specified to the second.
//!
//! openEHR class: `DV_DATE_TIME`, package `rm.data_types.quantity.date_time`.
//! Inherits: `DV_TEMPORAL`, `Iso8601_date_time`.
//!
//! Represents an absolute point in time, specified to the second. Semantics
//! defined by ISO 8601.
//!
//! Used for recording a precise point in real world time, and for
//! approximate time stamps, e.g. the origin of a `HISTORY` in an
//! `OBSERVATION` which is only partially known.
//!
//! # Dual inheritance
//!
//! Same shape as `DV_DATE`/`DV_TIME` (see `dv_date.rs`'s module doc for the
//! full rationale): `DV_TEMPORAL` (RM abstract ancestor, embedded state +
//! trait) composed alongside `Iso8601_date_time` (BASE foundation-types
//! mixin, `openehr_foundation::time::iso8601_date_time`), both as
//! fields/trait impls rather than Rust inheritance.
use crate::data_types::date_time::dv_duration::DvDuration;
use crate::data_types::date_time::dv_temporal::{DvTemporal, DvTemporalData};
use crate::data_types::quantity::dv_ordered::DvOrderedApi;
use crate::data_types::text::code_phrase::CodePhrase;
use openehr_foundation::primitive_types::any::Any;
use openehr_foundation::primitive_types::ordered::Ordered;

/// `DV_DATE_TIME`.
///
/// openEHR class: `DV_DATE_TIME`.
#[derive(Debug, Clone, PartialEq)]
pub struct DvDateTime {
    /// Embedded `DV_TEMPORAL` state, self-typed per the F-bounded threading
    /// documented on `DvTemporalData` (see `dv_temporal.rs`).
    pub temporal: DvTemporalData<DvDateTime>,

    /// `value`: `String` (`1..1`, redefined).
    ///
    /// ISO8601 date/time string.
    ///
    /// TODO(port): `Value_valid` invariant
    /// (`valid_iso8601_date_time(value)`) not yet enforced; see
    /// `invariant_value_valid` below.
    pub value: String,
}

pub const TYPE_NAME: &str = "DV_DATE_TIME";

impl DvDateTime {
    /// `magnitude` `(): double` (effected).
    ///
    /// Numeric value of the date/time as seconds since the calendar origin
    /// date/time `0001-01-01T00:00:00Z`.
    ///
    /// The published table types this `double` (lower-case, unlike every
    /// sibling function elsewhere in this package using `Real`/`Double`) —
    /// per `docs/ROSETTA.md`, both `Real` and `Double` are backed by `f64`
    /// in this codebase, so the lower-case spelling carries no distinct
    /// behaviour here; flagged for visibility only.
    ///
    /// TODO(port): requires parsing `value` as a (possibly partial) ISO 8601
    /// date-time and computing seconds-since-origin — deferred to the
    /// jiff-backed engine at P17.
    pub fn magnitude(&self) -> f64 {
        todo!(
            "DV_DATE_TIME.magnitude: seconds-since-0001-01-01T00:00:00Z, deferred to the jiff-backed engine at P17"
        )
    }

    /// `less_than` __alias__ `"<"` `(other: DV_DATE_TIME[1]): Boolean`
    /// (effected).
    ///
    /// `Post_result`: `Result = magnitude > other.magnitude` as published.
    ///
    /// PORT NOTE: same inverted-postcondition wording flagged on `DvDate`'s
    /// `less_than` (see that file for the full discrepancy note). Transcribed
    /// with name-implied semantics for consistency with `DV_DURATION`'s
    /// internally-correct wording.
    pub fn less_than(&self, other: &Self) -> bool {
        self.magnitude() < other.magnitude()
    }

    /// `is_strictly_comparable_to` `(other: DV_DATE_TIME[1]): Boolean`
    /// (effected).
    ///
    /// True, for any two Date/times.
    pub fn is_strictly_comparable_to(&self, _other: &Self) -> bool {
        true
    }

    /// `Value_valid` invariant: `valid_iso8601_date_time(value)`.
    ///
    /// TODO(port): bridges to the foundation-types validity predicate once
    /// the jiff-backed ISO 8601 parsing engine lands (P17).
    pub fn invariant_value_valid(&self) -> bool {
        todo!(
            "DV_DATE_TIME.invariant_value_valid: valid_iso8601_date_time bridges to the jiff-backed engine at P17"
        )
    }
}

impl Any for DvDateTime {
    /// `is_equal(other)` inherited through the `DV_QUANTIFIED` chain
    /// (magnitude-based comparison).
    ///
    /// TODO(port): forwards to `magnitude()` comparison once that is
    /// implemented, mirroring `DvDate::is_equal`.
    fn is_equal(&self, other: &Self) -> bool {
        let _ = other;
        todo!("DV_DATE_TIME.is_equal: pending DV_QUANTIFIED equality once magnitude() lands")
    }

    fn type_of(&self) -> String {
        "DvDateTime".to_string()
    }
}

impl Ordered for DvDateTime {
    /// Delegates to the inherent [`DvDateTime::less_than`] (the spec's
    /// effected `less_than`, magnitude-based).
    fn less_than(&self, other: &Self) -> bool {
        DvDateTime::less_than(self, other)
    }
}

impl DvOrderedApi for DvDateTime {
    /// `normal_status`: accessor into the embedded
    /// `DV_ORDERED` state reached through the
    /// `DV_TEMPORAL` → `DV_ABSOLUTE_QUANTITY` → `DV_QUANTIFIED` chain.
    fn normal_status(&self) -> Option<&CodePhrase> {
        self.temporal
            .quantified
            .quantified
            .ordered
            .normal_status
            .as_ref()
    }

    /// Delegates to the inherent [`DvDateTime::is_strictly_comparable_to`]
    /// ("True, for any two Date/times").
    fn is_strictly_comparable_to(&self, other: &Self) -> bool {
        DvDateTime::is_strictly_comparable_to(self, other)
    }
}

impl DvTemporal for DvDateTime {
    fn temporal_data(&self) -> &DvTemporalData<Self> {
        &self.temporal
    }

    /// `add` __alias__ `"+"` `(a_diff: DV_DURATION[1]): DV_DATE_TIME`
    /// (redefined).
    ///
    /// Addition of a Duration to this Date/time.
    ///
    /// TODO(port): ISO 8601 calendar+clock arithmetic, deferred to the
    /// jiff-backed engine at P17.
    fn add(&self, a_diff: &DvDuration) -> Self {
        let _ = a_diff;
        todo!(
            "DV_DATE_TIME.add: ISO 8601 calendar+clock arithmetic deferred to the jiff-backed engine at P17"
        )
    }

    /// `subtract` __alias__ `"-"` `(a_diff: DV_DURATION[1]): DV_DATE_TIME`
    /// (redefined).
    ///
    /// Subtract a Duration from this Date/time.
    ///
    /// TODO(port): see `add` above.
    fn subtract(&self, a_diff: &DvDuration) -> Self {
        let _ = a_diff;
        todo!(
            "DV_DATE_TIME.subtract: ISO 8601 calendar+clock arithmetic deferred to the jiff-backed engine at P17"
        )
    }

    /// `diff` __alias__ `"-"` `(other: DV_DATE_TIME[1]): DV_DURATION`
    /// (redefined).
    ///
    /// Difference between this Date/time and `other`.
    ///
    /// TODO(port): see `add` above.
    fn diff(&self, other: &Self) -> DvDuration {
        let _ = other;
        todo!(
            "DV_DATE_TIME.diff: ISO 8601 calendar+clock arithmetic deferred to the jiff-backed engine at P17"
        )
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.date_time — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_date_time.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master07-date_time_package.adoc §Class Descriptions / dv_date_time.adoc §DV_DATE_TIME Class
//   confidence: medium
//   todos: 6
//   note: same dual-inheritance shape as DV_DATE/DV_TIME; magnitude/add/subtract/diff/invariant_value_valid deferred to the jiff-backed engine at P17; less_than transcribed with name-implied semantics against the same likely copy-paste Post_result defect flagged on DV_DATE; magnitude's published lower-case "double" return type is the same f64 as Real/Double elsewhere per ROSETTA (flagged, no behavioural difference); Any/Ordered/DvOrderedApi impls added so DvDateTime satisfies the DvOrdered enum's trait chain (is_equal stubbed pending magnitude()).
// ─────────────────────────────────────────────
