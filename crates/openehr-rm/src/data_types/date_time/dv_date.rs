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
use serde::{Deserialize, Serialize};
// PORT NOTE: `Iso8601Date`/`Iso8601Type` are named in the module doc above
// for the inheritance narrative but not imported here — `value: String` is
// declared directly on this struct (redefined per the class table) rather
// than embedding `Iso8601TypeCore`, since the RM class table types `value`
// itself as `String` (not as an `Iso8601Date` instance) and adds its own
// `Value_valid` invariant calling `valid_iso8601_date`. A later phase may
// still choose to embed `openehr_foundation::time::iso8601_date::Iso8601Date`
// directly once the crate boundary (`openehr-rm` depends on
// `openehr-foundation`) and serde-flatten shape are both settled at P4/P17.

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
    /// TODO(port): the invariant is not yet enforced by a constructor or
    /// `Validate` impl; the validating predicate itself
    /// (`valid_iso8601_date`) is expected to bridge to
    /// `openehr_foundation::time::iso8601_date::Iso8601Date`'s own
    /// (currently `todo!()`) partial-precision validity logic once the
    /// jiff-backed engine lands at P17.
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
    /// TODO(port): requires parsing `value` as a (possibly partial) ISO 8601
    /// date and computing a Gregorian day-count from the `0001-01-01` origin
    /// — deferred to the jiff-backed engine (P17), consistent with every
    /// other `Iso8601_*` string-parsing TODO in `openehr-foundation`.
    pub fn magnitude(&self) -> i32 {
        todo!(
            "DV_DATE.magnitude: Gregorian day-count from 0001-01-01, deferred to the jiff-backed engine at P17"
        )
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
    /// TODO(port): forwards to `magnitude()` comparison once that is
    /// implemented.
    pub fn is_equal(&self, other: &Self) -> bool {
        let _ = other;
        todo!(
            "DV_DATE.is_equal: pending DV_QUANTIFIED equality once DataValue enum + magnitude() land"
        )
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
    /// TODO(port): bridges to the foundation-types validity predicate once
    /// the jiff-backed ISO 8601 parsing engine lands (P17); see the doc
    /// comment on `value` above.
    pub fn invariant_value_valid(&self) -> bool {
        todo!(
            "DV_DATE.invariant_value_valid: valid_iso8601_date bridges to the jiff-backed engine at P17"
        )
    }
}

impl Any for DvDate {
    /// Delegates to the inherent [`DvDate::is_equal`] (the spec's effected
    /// `is_equal`, itself pending `magnitude()`).
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
    /// supplies the concrete `DvDate`-specific arithmetic body rather than
    /// needing a distinct signature — encoded per ADR-001 §6.
    ///
    /// TODO(port): ISO 8601 calendar arithmetic (`0001-01-01`-origin day
    /// count plus/minus a duration), deferred to the jiff-backed engine at
    /// P17.
    fn add(&self, a_diff: &DvDuration) -> Self {
        let _ = a_diff;
        todo!("DV_DATE.add: ISO 8601 calendar arithmetic deferred to the jiff-backed engine at P17")
    }

    /// `subtract` __alias__ `"-"` `(a_diff: DV_DURATION[1]): DV_DATE`
    /// (redefined).
    ///
    /// Subtract a Duration from this Date.
    ///
    /// TODO(port): see `add` above.
    fn subtract(&self, a_diff: &DvDuration) -> Self {
        let _ = a_diff;
        todo!(
            "DV_DATE.subtract: ISO 8601 calendar arithmetic deferred to the jiff-backed engine at P17"
        )
    }

    /// `diff` __alias__ `"-"` `(other: DV_DATE[1]): DV_DURATION` (redefined).
    ///
    /// Difference between this Date and `other`.
    ///
    /// TODO(port): see `add` above.
    fn diff(&self, other: &Self) -> DvDuration {
        let _ = other;
        todo!(
            "DV_DATE.diff: ISO 8601 calendar arithmetic deferred to the jiff-backed engine at P17"
        )
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.date_time — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_date.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master07-date_time_package.adoc §Class Descriptions / dv_date.adoc §DV_DATE Class
//   confidence: medium
//   todos: 7
//   note: dual inheritance (DV_TEMPORAL RM ancestor + Iso8601_date foundation mixin) composed as field+trait; magnitude/add/subtract/diff/is_equal/invariant_value_valid all deferred to the jiff-backed engine at P17; less_than transcribed with name-implied semantics against a likely copy-paste defect in the published Post_result postcondition (flagged, matches DV_DURATION's internally-consistent wording, not DV_TIME/DV_DATE_TIME's inverted one); Any/Ordered/DvOrderedApi impls delegate to the inherent effected functions so DvDate satisfies the DvOrdered enum's trait chain. P4: Serialize/Deserialize added; `temporal` (DvTemporalData<DvDate>) flattened, schema-verified (normal_status/normal_range/other_reference_ranges/magnitude_status/accuracy all sit flat alongside DV_DATE's own `value`); ADR-002 self-tagging applied (TypeTag<Self> first field + TypeName from TYPE_NAME) — the tag is the sole wire-level discriminator vs the structure-identical DV_TIME/DV_DATE_TIME.
// ─────────────────────────────────────────────
