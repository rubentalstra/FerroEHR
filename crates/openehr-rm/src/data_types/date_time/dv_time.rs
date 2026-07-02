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
use crate::data_types::quantity::dv_ordered::DvOrderedApi;
use crate::data_types::text::code_phrase::CodePhrase;
use openehr_foundation::primitive_types::any::Any;
use openehr_foundation::primitive_types::ordered::Ordered;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use openehr_foundation::time::iso8601_parser::parse_time;
use openehr_foundation::time::time_definitions::TimeDefinitions;
use serde::{Deserialize, Serialize};

/// `DV_TIME`.
///
/// openEHR class: `DV_TIME`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvTime {
    /// Canonical `_type` discriminator (`"DV_TIME"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    ///
    /// This tag is what distinguishes `DV_TIME` from the structure-identical
    /// `DV_DATE`/`DV_DATE_TIME` (`{value: String}` on the wire) in untagged
    /// enum dispatch — do not add extra fields to disambiguate.
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `DV_TEMPORAL` state, self-typed per the F-bounded threading
    /// documented on `DvTemporalData` (see `dv_temporal.rs`).
    #[serde(flatten)]
    pub temporal: DvTemporalData<DvTime>,

    /// `value`: `String` (`1..1`, redefined).
    ///
    /// ISO8601 time string.
    ///
    /// PORT NOTE: the `Value_valid` invariant is exposed as
    /// [`DvTime::invariant_value_valid`], but is not yet enforced by a
    /// constructor or `Validate` impl.
    pub value: String,
}

pub const TYPE_NAME: &str = "DV_TIME";

impl TypeName for DvTime {
    const NAME: &'static str = TYPE_NAME;
}

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
    /// PORT NOTE: partial times use zero for unknown trailing components,
    /// matching the BASE `Iso8601_time` component accessors (`minute()` /
    /// `second()` return 0 when not present).
    pub fn magnitude(&self) -> f64 {
        parse_time(&self.value).map_or(0.0, |parsed| parsed.seconds_since_midnight())
    }

    // `add` __alias__ `"+"` `(a_diff: DV_DURATION[1]): DV_TIME` (redefined
    // via the `DvTemporal` impl below).
    //
    // `subtract`, `diff` likewise — see the `DvTemporal for DvTime` impl.

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
    pub fn invariant_value_valid(&self) -> bool {
        TimeDefinitions::valid_iso8601_time(&self.value)
    }
}

impl Any for DvTime {
    /// `is_equal(other)` inherited through the `DV_QUANTIFIED` chain
    /// (magnitude-based comparison).
    fn is_equal(&self, other: &Self) -> bool {
        match (parse_time(&self.value), parse_time(&other.value)) {
            (Some(left), Some(right)) => {
                left.seconds_since_midnight() == right.seconds_since_midnight()
            }
            _ => self.value == other.value,
        }
    }

    fn type_of(&self) -> String {
        "DvTime".to_string()
    }
}

impl Ordered for DvTime {
    /// Delegates to the inherent [`DvTime::less_than`] (the spec's effected
    /// `less_than`, magnitude-based).
    fn less_than(&self, other: &Self) -> bool {
        DvTime::less_than(self, other)
    }
}

impl DvOrderedApi for DvTime {
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

    /// Delegates to the inherent [`DvTime::is_strictly_comparable_to`]
    /// ("True, for any two Times").
    fn is_strictly_comparable_to(&self, other: &Self) -> bool {
        DvTime::is_strictly_comparable_to(self, other)
    }
}

impl DvTemporal for DvTime {
    fn temporal_data(&self) -> &DvTemporalData<Self> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_magnitude_is_seconds_since_midnight() {
        let time: DvTime =
            serde_json::from_str(r#"{"_type":"DV_TIME","value":"01:02:03.5"}"#).unwrap();

        assert!(time.invariant_value_valid());
        assert_eq!(time.magnitude(), 3_723.5);
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.date_time — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_time.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master07-date_time_package.adoc §Class Descriptions / dv_time.adoc §DV_TIME Class
//   confidence: medium
//   todos: 3
//   note: same dual-inheritance shape as DV_DATE; magnitude/is_equal/invariant_value_valid now delegate to the foundation BASE ISO 8601 parser. add/subtract/diff remain TODO(port) pending an explicit clock-wrapping policy. less_than transcribed with name-implied semantics against the same likely copy-paste Post_result defect flagged on DV_DATE; Any/Ordered/DvOrderedApi impls added so DvTime satisfies the DvOrdered enum's trait chain. P4: Serialize/Deserialize added; `temporal` (DvTemporalData<DvTime>) flattened (same schema-verified shape as DV_DATE); ADR-002 self-tagging applied (TypeTag<Self> first field + TypeName from TYPE_NAME) — the tag is the sole wire-level discriminator vs the structure-identical DV_DATE/DV_DATE_TIME.
// ─────────────────────────────────────────────
