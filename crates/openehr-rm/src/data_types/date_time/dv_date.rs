//! `DV_DATE` — an absolute point in time on the Gregorian calendar, to the day.
//!
//! openEHR class: `DV_DATE`, package `rm.data_types.quantity.date_time`.
//! Inherits: `DV_TEMPORAL`, `Iso8601_date`.
//!
//! Represents an absolute point in time, as measured on the Gregorian
//! calendar, and specified only to the day. Semantics defined by ISO 8601.
//! Used for recording dates in real world time. The partial form is used for
//! approximate birth dates, dates of death, etc.
//!
//! # Dual inheritance: RM abstract ancestor + foundation-types ISO 8601 mixin
//!
//! `DV_DATE` inherits from two classes of very different character:
//! `DV_TEMPORAL` (an RM abstract class carrying attributes, composed here via
//! `DvTemporalData`/`DvTemporal` per ADR-001 §3), and `Iso8601_date` (the
//! BASE foundation-types class transcribed in
//! `openehr_foundation::time::iso8601_date`, whose own multiple-inheritance
//! shape is the ADR-001 §2 worked example). This is not the `DV_DURATION`
//! style of multiple inheritance (two *parallel* parents contributing
//! disjoint state) — `Iso8601_date` supplies the string-value contract and
//! partial-precision semantics that `value` below actually holds, while
//! `DV_TEMPORAL` supplies the RM-level `accuracy`/temporal-arithmetic
//! contract. Both are composed as fields/trait impls rather than Rust
//! inheritance, per the standard RM transcription rule.
use crate::data_types::date_time::dv_duration::DvDuration;
use crate::data_types::date_time::dv_temporal::{DvTemporal, DvTemporalData};
use crate::data_types::quantity::dv_ordered::DvOrderedApi;
use crate::data_types::text::code_phrase::CodePhrase;
use openehr_foundation::primitive_types::any::Any;
use openehr_foundation::primitive_types::ordered::Ordered;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use openehr_foundation::time::iso8601_date::Iso8601Date;
use openehr_foundation::time::iso8601_parser::{days_since_origin, parse_date};
use openehr_foundation::time::iso8601_type::Iso8601TypeCore;
use openehr_foundation::time::time_definitions::TimeDefinitions;
use serde::{Deserialize, Serialize};
// PORT NOTE: `value: String` is declared directly on this struct (redefined
// per the class table) rather than embedding `Iso8601TypeCore`, since the RM
// class table types `value` itself as `String` (not as an `Iso8601Date`
// instance) and adds its own `Value_valid` invariant calling
// `valid_iso8601_date`. The foundation `Iso8601Date` is nonetheless used
// transiently by the `DV_TEMPORAL` arithmetic (`add`/`subtract`/`diff`
// below), which delegate to its jiff-backed engine (ADR-003 policies 1-3): a
// throwaway `Iso8601Date` is built from `value`, the computation runs there,
// and the resulting string is stored back. A later phase may still choose to
// embed `Iso8601Date` directly once the serde-flatten shape is settled.

/// `DV_DATE`.
///
/// openEHR class: `DV_DATE`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvDate {
    /// Canonical `_type` discriminator (`"DV_DATE"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    ///
    /// This tag is what distinguishes `DV_DATE` from the structure-identical
    /// `DV_TIME`/`DV_DATE_TIME` (`{value: String}` on the wire) in untagged
    /// enum dispatch — do not add extra fields to disambiguate.
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `DV_TEMPORAL` state, self-typed per the F-bounded threading
    /// documented on `DvTemporalData` (see `dv_temporal.rs`).
    #[serde(flatten)]
    pub temporal: DvTemporalData<DvDate>,

    /// `value`: `String` (`1..1`, redefined).
    ///
    /// ISO8601 date string.
    ///
    /// Invariant `Value_valid`: `valid_iso8601_date(value)`.
    ///
    /// PORT NOTE: the invariant is exposed as
    /// [`DvDate::invariant_value_valid`], but is not yet enforced by a
    /// constructor or `Validate` impl.
    pub value: String,
}

pub const TYPE_NAME: &str = "DV_DATE";

impl TypeName for DvDate {
    const NAME: &'static str = TYPE_NAME;
}

impl DvDate {
    /// `magnitude` `(): Integer` (effected).
    ///
    /// Numeric value of the date as days since the calendar origin date
    /// `0001-01-01`.
    ///
    /// PORT NOTE: partial dates are valid `DV_DATE` values but do not define
    /// a unique calendar day. This returns the day count for complete dates
    /// and `0` for incomplete or invalid strings; callers that need to
    /// distinguish those cases should check `invariant_value_valid()` and
    /// the foundation `Iso8601_date` partial accessors.
    pub fn magnitude(&self) -> i32 {
        parse_date(&self.value)
            .and_then(
                openehr_foundation::time::iso8601_parser::ParsedIso8601Date::as_complete_jiff_date,
            )
            .map_or(0, days_since_origin)
    }

    /// `is_equal` `(other: DV_QUANTIFIED[1]): Boolean` (effected).
    ///
    /// Return `true` if this `DV_QUANTIFIED` is considered equal to `other`.
    ///
    /// PORT NOTE: the class table types `other` as the abstract ancestor
    /// `DV_QUANTIFIED`, not `DV_DATE` — transcribed here narrowed to `&Self`
    /// since a faithful heterogeneous-`DV_QUANTIFIED` comparison would
    /// require a `DataValue`-family enum dispatch that does not yet exist at
    /// this point in the transcription (the `data_types.quantity` cluster is
    /// concurrent, not yet landed). Revisit once `DV_QUANTIFIED`/`DataValue`
    /// exist as a closed enum (ADR-001 §4).
    ///
    pub fn is_equal(&self, other: &Self) -> bool {
        match (
            parse_date(&self.value).and_then(
                openehr_foundation::time::iso8601_parser::ParsedIso8601Date::as_complete_jiff_date,
            ),
            parse_date(&other.value).and_then(
                openehr_foundation::time::iso8601_parser::ParsedIso8601Date::as_complete_jiff_date,
            ),
        ) {
            (Some(left), Some(right)) => days_since_origin(left) == days_since_origin(right),
            _ => self.value == other.value,
        }
    }

    /// `less_than` __alias__ `"<"` `(other: DV_DATE[1]): Boolean` (effected).
    ///
    /// `Post_result`: `Result = magnitude > other.magnitude`.
    ///
    /// True if `other` is less than this Quantified object. Based on
    /// comparison of `magnitude`.
    ///
    /// PORT NOTE: the spec's own `Post_result` postcondition text
    /// (`Result = magnitude > other.magnitude`) reads backwards relative to
    /// the function name `less_than` — a `less_than(other)` call returning
    /// `magnitude > other.magnitude` would mean "self is less than other"
    /// is true exactly when "self's magnitude is greater", which is the
    /// opposite of the ordinary sense. This same inverted-postcondition
    /// wording repeats verbatim across `DV_TIME`, `DV_DATE_TIME`, and
    /// `DV_DURATION` in this package (`DV_DURATION`'s reads
    /// `magnitude < other.magnitude`, the only one internally consistent
    /// with its own name) — flagged as a likely copy-paste defect in the
    /// published spec table rather than silently "corrected": transcribed
    /// with the name-implied ("self's magnitude is less than other's")
    /// semantics, matching the `DV_DURATION` case.
    pub fn less_than(&self, other: &Self) -> bool {
        self.magnitude() < other.magnitude()
    }

    /// `is_strictly_comparable_to` `(other: DV_DATE[1]): Boolean` (effected).
    ///
    /// True, for any two Dates.
    pub fn is_strictly_comparable_to(&self, _other: &Self) -> bool {
        true
    }

    /// `Value_valid` invariant: `valid_iso8601_date(value)`.
    ///
    pub fn invariant_value_valid(&self) -> bool {
        TimeDefinitions::valid_iso8601_date(&self.value)
    }

    /// The foundation `Iso8601_date` mirror of this value, used transiently
    /// to delegate `DV_TEMPORAL` arithmetic (ADR-003 policies 1-3).
    fn as_iso8601_date(&self) -> Iso8601Date {
        Iso8601Date {
            core: Iso8601TypeCore {
                value: self.value.clone(),
            },
        }
    }

    /// Rebuild this `DV_DATE` with a new ISO 8601 `value`, preserving the
    /// embedded `DV_TEMPORAL` state and the type tag.
    fn with_value(&self, value: String) -> Self {
        Self {
            type_tag: self.type_tag,
            temporal: self.temporal.clone(),
            value,
        }
    }
}

impl Any for DvDate {
    /// Delegates to the inherent [`DvDate::is_equal`] (the spec's effected
    /// `is_equal`, itself backed by `magnitude()` for complete dates and raw
    /// value equality for partial dates.
    fn is_equal(&self, other: &Self) -> bool {
        DvDate::is_equal(self, other)
    }

    fn type_of(&self) -> String {
        "DvDate".to_string()
    }
}

impl Ordered for DvDate {
    /// Delegates to the inherent [`DvDate::less_than`] (the spec's effected
    /// `less_than`, magnitude-based).
    fn less_than(&self, other: &Self) -> bool {
        DvDate::less_than(self, other)
    }
}

impl DvOrderedApi for DvDate {
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

    /// Delegates to the inherent [`DvDate::is_strictly_comparable_to`]
    /// ("True, for any two Dates").
    fn is_strictly_comparable_to(&self, other: &Self) -> bool {
        DvDate::is_strictly_comparable_to(self, other)
    }
}

impl DvTemporal for DvDate {
    fn temporal_data(&self) -> &DvTemporalData<Self> {
        &self.temporal
    }

    /// `add` __alias__ `"+"` `(a_diff: DV_DURATION[1]): DV_DATE` (redefined
    /// from `DV_TEMPORAL.add`, which returns `DV_TEMPORAL`).
    ///
    /// Addition of a Duration to this Date.
    ///
    /// This is the covariant redefinition named by the class table's
    /// `(redefined)` marker: the parent `DvTemporal::add` returns `Self`
    /// already (a generic trait-method shape), so this override just
    /// supplies the concrete `DvDate`-specific arithmetic body — encoded per
    /// ADR-001 §6. Delegates to `Iso8601_date::add` (definite arithmetic,
    /// ADR-003 policy 1: an exact day/second shift with partial-precision
    /// anchoring and truncation), preserving this `DV_DATE`'s temporal state.
    fn add(&self, a_diff: &DvDuration) -> Self {
        self.with_value(self.as_iso8601_date().add(&a_diff.iso8601).core.value)
    }

    /// `subtract` __alias__ `"-"` `(a_diff: DV_DURATION[1]): DV_DATE`
    /// (redefined).
    ///
    /// Subtract a Duration from this Date. Delegates to
    /// `Iso8601_date::subtract`; see `add` above.
    fn subtract(&self, a_diff: &DvDuration) -> Self {
        self.with_value(self.as_iso8601_date().subtract(&a_diff.iso8601).core.value)
    }

    /// `diff` __alias__ `"-"` `(other: DV_DATE[1]): DV_DURATION` (redefined).
    ///
    /// Difference between this Date and `other`, as a `DV_DURATION` in
    /// definite units. Delegates to `Iso8601_date::diff` (receiver minus
    /// argument, ADR-003 policy 1).
    fn diff(&self, other: &Self) -> DvDuration {
        DvDuration::from_iso8601(self.as_iso8601_date().diff(&other.as_iso8601_date()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_magnitude_uses_calendar_origin() {
        let date: DvDate =
            serde_json::from_str(r#"{"_type":"DV_DATE","value":"0001-01-02"}"#).unwrap();

        assert!(date.invariant_value_valid());
        assert_eq!(date.magnitude(), 1);
    }

    #[test]
    fn partial_date_is_valid_but_has_no_day_magnitude() {
        let date: DvDate =
            serde_json::from_str(r#"{"_type":"DV_DATE","value":"2026-07"}"#).unwrap();

        assert!(date.invariant_value_valid());
        assert_eq!(date.magnitude(), 0);
    }

    fn date(value: &str) -> DvDate {
        serde_json::from_str(&format!(r#"{{"_type":"DV_DATE","value":"{value}"}}"#)).unwrap()
    }

    fn duration(value: &str) -> DvDuration {
        serde_json::from_str(&format!(r#"{{"_type":"DV_DURATION","value":"{value}"}}"#)).unwrap()
    }

    /// `add`/`subtract`/`diff` delegate to the foundation `Iso8601_date`
    /// engine: exact day arithmetic across a leap-year boundary, and `diff`
    /// yielding a definite-unit `DV_DURATION`.
    #[test]
    fn add_subtract_diff_delegate_to_iso8601_date() {
        assert_eq!(date("2004-02-28").add(&duration("P1D")).value, "2004-02-29");
        assert_eq!(
            date("2004-03-01").subtract(&duration("P1D")).value,
            "2004-02-29"
        );
        // diff returns a DV_DURATION in definite units (receiver - argument).
        let d = date("2004-03-16").diff(&date("2004-02-15"));
        assert_eq!(d.iso8601.core.value, "P30D");
        assert_eq!(
            date("2004-02-15")
                .diff(&date("2004-03-16"))
                .iso8601
                .core
                .value,
            "-P30D"
        );
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.date_time — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_date.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master07-date_time_package.adoc §Class Descriptions / dv_date.adoc §DV_DATE Class
//   confidence: high
//   todos: 0
//   note: dual inheritance (DV_TEMPORAL RM ancestor + Iso8601_date foundation mixin) composed as field+trait; magnitude/is_equal/invariant_value_valid delegate to the foundation BASE ISO 8601 parser. add/subtract/diff now implemented by delegating to Iso8601_date::add/subtract/diff (definite arithmetic, ADR-003 policy 1) via a throwaway Iso8601Date mirror, preserving the DV_TEMPORAL state; in-file test pins leap-boundary day arithmetic and antisymmetric diff. less_than transcribed with name-implied semantics against a likely copy-paste defect in the published Post_result postcondition (flagged, matches DV_DURATION's internally-consistent wording, not DV_TIME/DV_DATE_TIME's inverted one); Any/Ordered/DvOrderedApi impls delegate to the inherent effected functions so DvDate satisfies the DvOrdered enum's trait chain. P4: Serialize/Deserialize added; `temporal` (DvTemporalData<DvDate>) flattened, schema-verified; ADR-002 self-tagging applied (TypeTag<Self> first field + TypeName from TYPE_NAME) — the tag is the sole wire-level discriminator vs the structure-identical DV_TIME/DV_DATE_TIME.
// ─────────────────────────────────────────────
