//! `DV_DURATION` — a period of time with respect to a notional (unstated)
//! origin point.
//!
//! openEHR class: `DV_DURATION`, package `rm.data_types.quantity.date_time`.
//! Inherits: `DV_AMOUNT`, `Iso8601_duration`.
//!
//! Represents a period of time with respect to a notional point in time,
//! which is not specified. A sign may be used to indicate the duration is
//! "backwards" in time rather than forwards.
//!
//! NOTE (spec): two deviations from ISO 8601 are supported, the first, to
//! allow a negative sign, and the second allowing the `'W'` designator to be
//! mixed with other designators. See time types section in the Foundation
//! Types model.
//!
//! Used for recording the duration of something in the real world,
//! particularly when there is a need a) to represent the duration in
//! customary format, i.e. days, hours, minutes etc, and b) if it will be
//! used in computational operations with date/time quantities, i.e.
//! additions, subtractions etc.
//!
//! Misuse: Durations cannot be used to represent points in time, or
//! intervals of time.
//!
//! # Multiple inheritance — the named Section 7.2 hazard
//!
//! `DV_DURATION` is explicitly named in PORT_MASTER_PLAN.md §7.2 alongside
//! `Ordered_Numeric` and `Iso8601_type` as one of the three multiple-
//! inheritance hazards to resolve deliberately rather than reactively. Its
//! two parents contribute genuinely disjoint state and behaviour, which is
//! the defining feature of this MI case (contrast the `DV_DATE`/`DV_TIME`/
//! `DV_DATE_TIME` classes in this same package, whose second parent —
//! `Iso8601_date`/`_time`/`_date_time` — is a pure string-value mixin with
//! no attributes beyond `value` itself, already subsumed into the RM class's
//! own redefined `value: String` field):
//!
//! * `DV_AMOUNT` (RM abstract ancestor, transcribed here via composition per
//!   ADR-001 §3) contributes `accuracy_is_percent: Boolean` and a redefined
//!   `accuracy: Real`, plus the `DV_QUANTIFIED`/`DV_ORDERED` chain's
//!   `magnitude`-based comparison contract, and the `+`/`-` arithmetic
//!   `DV_AMOUNT` newly defines relative to `DV_ABSOLUTE_QUANTITY`.
//! * `Iso8601_duration` (BASE foundation-types mixin,
//!   `openehr_foundation::time::iso8601_duration`) contributes the actual
//!   `value: String` ISO 8601 duration representation and every
//!   component-accessor (`years()`, `months()`, ..., `to_seconds()`).
//!
//! Per the RM transcription rule for multiple inheritance ("composition of
//! fields from all parents plus one trait per parent behaviour", restated
//! in ADR-001 §3), this struct embeds `DV_AMOUNT`'s state directly (there is
//! no separate `DvAmountData`/`DvAmount` trait split written yet by this
//! transcription pass — the `quantity` package cluster transcribing
//! `DV_AMOUNT` itself is concurrent with this one; see the forward-reference
//! note below) and embeds `Iso8601_duration`'s state via
//! `openehr_foundation::time::iso8601_duration::Iso8601Duration` directly
//! (not merely its `Iso8601TypeCore`), since every one of this RM class's
//! own effected functions (`magnitude`, `negative`) explicitly delegates to
//! an `Iso8601_duration`-level method (`to_seconds()`) per the spec's own
//! function descriptions — the full `Iso8601Duration` API surface, not just
//! its raw string, is genuinely needed here.
//!
//! # Forward references
//!
//! `DV_AMOUNT`'s own attributes (`accuracy_is_percent`, `accuracy`) are
//! inlined directly onto this struct rather than through a shared
//! `DvAmountData` embed, since that shared struct is owned by the concurrent
//! `quantity` package transcription. `// TODO(port):` (P17 wiring) marks the
//! reconciliation point: once `DvAmountData`/`DvAmount` land, this struct
//! should hold `pub amount: DvAmountData` instead of the two inlined fields
//! below, mirroring the `DvTemporalData` embedding used by
//! `DvDate`/`DvTime`/`DvDateTime` in this same package.
use crate::data_types::quantity::dv_ordered::DvOrderedApi;
use crate::data_types::text::code_phrase::CodePhrase;
use openehr_foundation::primitive_types::any::Any;
use openehr_foundation::primitive_types::ordered::Ordered;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use openehr_foundation::time::iso8601_duration::Iso8601Duration;
use openehr_foundation::time::time_definitions::TimeDefinitions;
use serde::{Deserialize, Serialize};

/// `DV_DURATION`.
///
/// openEHR class: `DV_DURATION`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvDuration {
    /// Canonical `_type` discriminator (`"DV_DURATION"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    // ---- DV_AMOUNT state (inlined; see the forward-reference note above) ----
    /// `accuracy_is_percent`: `Boolean` (`0..1`), inherited from `DV_AMOUNT`.
    ///
    /// If `true`, indicates that when this object was created, `accuracy`
    /// was recorded as a percent value; if `false`, as an absolute quantity
    /// value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy_is_percent: Option<bool>,

    /// `accuracy`: `Real` (`0..1`, redefined from `DV_AMOUNT`), inherited
    /// from `DV_AMOUNT`.
    ///
    /// Accuracy of measurement, expressed either as a half-range percent
    /// value (`accuracy_is_percent = true`) or a half-range quantity. A
    /// value of `0` means that accuracy is 100%, i.e. no error.
    ///
    /// A value of `unknown_accuracy_value` means that accuracy was not
    /// recorded.
    ///
    /// TODO(port) (P17 wiring): the spec's `unknown_accuracy_value` sentinel
    /// is not itself defined in this class's own table (it is referenced by
    /// `DV_AMOUNT`'s description only); modelled as `None` here rather than
    /// a magic `Real` sentinel, pending confirmation this is the intended
    /// reading once the concurrent `quantity` cluster lands `DV_AMOUNT`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<f64>,

    // ---- Iso8601_duration state ----
    /// Embedded `Iso8601_duration` state (in turn embedding
    /// `Iso8601_type.value: String` via `Iso8601TypeCore`).
    ///
    /// PORT NOTE: the RM class table separately redeclares `value: String`
    /// as `(redefined)` with its own `Value_valid` invariant
    /// (`valid_iso8601_duration(value)`), which reads as the RM level
    /// re-asserting/narrowing the type already fixed by `Iso8601_duration`
    /// rather than introducing new state — accessed here via
    /// `self.iso8601.core.value`, not duplicated as a second field.
    ///
    /// `#[serde(flatten)]` per this crate's established embedded-parent
    /// convention.
    ///
    /// PORT NOTE (P4): the foundation P4 pass supplied `Iso8601Duration`'s
    /// own serde derive *and* flattened its embedded `Iso8601TypeCore`, so
    /// this double flatten (`iso8601` here, `core` there) surfaces `value`
    /// as a plain top-level `String` — the canonical DV_DURATION wire shape
    /// is `{"_type":"DV_DURATION","value":"P1DT2H"}`, never a nested
    /// `{"iso8601":{...}}` or `{"core":{...}}` object.
    #[serde(flatten)]
    pub iso8601: Iso8601Duration,
}

pub const TYPE_NAME: &str = "DV_DURATION";

impl TypeName for DvDuration {
    const NAME: &'static str = TYPE_NAME;
}

impl DvDuration {
    /// Construct a `DV_DURATION` wrapping a foundation `Iso8601_duration`,
    /// with no `DV_AMOUNT` accuracy metadata.
    ///
    /// PORT NOTE: not a spec function — a construction convenience used by the
    /// `DV_TEMPORAL` `diff` implementations (`DvDate`/`DvTime`/`DvDateTime`),
    /// whose spec return type is a bare `DV_DURATION` computed from the
    /// foundation `Iso8601_*::diff`. Accuracy fields are `None` because a
    /// difference carries no measurement accuracy of its own.
    #[must_use]
    pub fn from_iso8601(iso8601: Iso8601Duration) -> Self {
        Self {
            type_tag: TypeTag::new(),
            accuracy_is_percent: None,
            accuracy: None,
            iso8601,
        }
    }

    /// `add` __alias__ `"+"` `(other: DV_DURATION[1]): DV_DURATION`
    /// (redefined from `DV_AMOUNT.add`).
    ///
    /// Sum of this Duration and `other`.
    ///
    pub fn add(&self, other: &Self) -> Self {
        self.with_iso8601(self.iso8601.add(&other.iso8601))
    }

    /// `subtract` __alias__ `"-"` `(other: DV_DURATION[1]): DV_DURATION`
    /// (redefined from `DV_AMOUNT.subtract`).
    ///
    /// Difference of this Duration and `other`.
    ///
    pub fn subtract(&self, other: &Self) -> Self {
        self.with_iso8601(self.iso8601.subtract(&other.iso8601))
    }

    /// `multiply` __alias__ `"*"` `(factor: Real[1]): DV_DURATION`
    /// (redefined from `DV_AMOUNT.multiply`).
    ///
    /// Product of this Duration and `factor`.
    ///
    pub fn multiply(&self, factor: f64) -> Self {
        self.with_iso8601(self.iso8601.multiply(factor))
    }

    /// `less_than` __alias__ `"<"` `(other: DV_DURATION[1]): Boolean`
    /// (effected).
    ///
    /// `Post_result`: `Result = magnitude < other.magnitude`.
    ///
    /// This is the one class in this package whose published `Post_result`
    /// wording is internally consistent with its own function name (see the
    /// PORT NOTEs on `DvDate::less_than`, `DvTime::less_than`,
    /// `DvDateTime::less_than` in sibling files, which flag the opposite,
    /// inverted wording on those three classes as a likely copy-paste
    /// defect) — transcribed literally as published, with no discrepancy to
    /// flag here.
    pub fn less_than(&self, other: &Self) -> bool {
        self.magnitude() < other.magnitude()
    }

    /// `is_strictly_comparable_to` `(other: DV_DURATION[1]): Boolean`
    /// (effected).
    ///
    /// True, for any two Durations.
    pub fn is_strictly_comparable_to(&self, _other: &Self) -> bool {
        true
    }

    /// `negative` __alias__ `"-"` `(): DV_DURATION` (redefined from
    /// `DV_AMOUNT.negative`).
    ///
    /// Negated version of current duration.
    ///
    /// Assuming the current duration is positive, the negated version
    /// represents a time prior to some origin point, or a negative age
    /// (e.g. so-called 'adjusted age' of premature infant).
    ///
    pub fn negative(&self) -> Self {
        self.with_iso8601(self.iso8601.negative())
    }

    /// `magnitude` `(): Double` (effected).
    ///
    /// Numeric value of the duration as a number of seconds. Computed using
    /// the method `to_seconds()` inherited from `Iso8601_duration`.
    ///
    /// This is the one function on this class whose spec description
    /// explicitly names the cross-parent delegation this MI hazard is about:
    /// the RM-level `magnitude()` (part of the `DV_ORDERED`/`DV_QUANTIFIED`
    /// comparison contract inherited via `DV_AMOUNT`) is implemented purely
    /// in terms of the `Iso8601_duration`-parent's own `to_seconds()`.
    ///
    pub fn magnitude(&self) -> f64 {
        self.iso8601.to_seconds()
    }

    /// `Value_valid` invariant: `valid_iso8601_duration(value)`.
    ///
    pub fn invariant_value_valid(&self) -> bool {
        TimeDefinitions::valid_iso8601_duration(&self.iso8601.core.value)
    }

    fn with_iso8601(&self, iso8601: Iso8601Duration) -> Self {
        Self {
            type_tag: self.type_tag,
            accuracy_is_percent: self.accuracy_is_percent,
            accuracy: self.accuracy,
            iso8601,
        }
    }

    // ---- DV_AMOUNT invariants (inlined; see the forward-reference note above) ----

    /// `Accuracy_is_percent_validity` invariant (inherited from
    /// `DV_AMOUNT`): `accuracy = 0 implies not accuracy_is_percent`.
    pub fn invariant_accuracy_is_percent_validity(&self) -> bool {
        if self.accuracy == Some(0.0) {
            !self.accuracy_is_percent.unwrap_or(false)
        } else {
            true
        }
    }

    /// `Accuracy_validity` invariant (inherited from `DV_AMOUNT`):
    /// `accuracy_is_percent implies valid_percentage(accuracy)`.
    ///
    /// TODO(port) (P17 wiring): `valid_percentage` (`DV_AMOUNT.valid_percentage`)
    /// is owned by the concurrent `quantity` cluster's `DV_AMOUNT`; a literal
    /// `0..=100` percentage range check is the described contract in the
    /// interim.
    pub fn invariant_accuracy_validity(&self) -> bool {
        if self.accuracy_is_percent.unwrap_or(false) {
            match self.accuracy {
                Some(a) => (0.0..=100.0).contains(&a),
                None => false,
            }
        } else {
            true
        }
    }
}

impl Any for DvDuration {
    /// `is_equal(other)` inherited through the `DV_AMOUNT`/`DV_QUANTIFIED`
    /// chain.
    ///
    /// PORT NOTE: this class's own table gives no explicit `is_equal` row;
    /// compares every declared attribute directly as the most literal
    /// reading, mirroring `DvQuantity::is_equal`'s identical situation. A
    /// future DV_AMOUNT reconciliation can decide whether equality should be
    /// normalized to magnitude rather than preserving the literal value
    /// string and accuracy fields.
    fn is_equal(&self, other: &Self) -> bool {
        self.accuracy_is_percent == other.accuracy_is_percent
            && self.accuracy == other.accuracy
            && self.iso8601 == other.iso8601
    }

    fn type_of(&self) -> String {
        "DvDuration".to_string()
    }
}

impl Ordered for DvDuration {
    /// Delegates to the inherent [`DvDuration::less_than`] (the spec's
    /// effected `less_than`, magnitude-based).
    fn less_than(&self, other: &Self) -> bool {
        DvDuration::less_than(self, other)
    }
}

impl DvOrderedApi for DvDuration {
    /// `normal_status`: inherited from `DV_ORDERED` through the `DV_AMOUNT`
    /// chain.
    ///
    /// TODO(port) (P17 wiring): `DvDuration` inlines `DV_AMOUNT`'s two
    /// attributes directly instead of embedding `DvAmountData<Self>` (see the
    /// forward-reference note in the module doc), so the `DV_ORDERED`-level
    /// state (`normal_status`/`normal_range`/`other_reference_ranges`) has no
    /// backing field yet — this cannot be implemented here without the
    /// concurrent `quantity` cluster's `DvAmountData` (crate-boundary owned).
    /// Stubbed pending that documented reconciliation.
    fn normal_status(&self) -> Option<&CodePhrase> {
        todo!(
            "DV_DURATION.normal_status: no backing DV_ORDERED state until the P17 DvAmountData<Self> embedding reconciliation (quantity cluster)"
        )
    }

    /// Delegates to the inherent [`DvDuration::is_strictly_comparable_to`]
    /// ("True, for any two Durations").
    fn is_strictly_comparable_to(&self, other: &Self) -> bool {
        DvDuration::is_strictly_comparable_to(self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openehr_foundation::time::iso8601_type::Iso8601TypeCore;

    /// Pins the canonical DV_DURATION wire shape: the double flatten
    /// (`DvDuration.iso8601` → `Iso8601Duration.core`) must surface `value`
    /// as a plain top-level string next to `_type`, never nested under
    /// `iso8601`/`core`.
    #[test]
    fn duration_value_serializes_flat_with_type_tag_first() {
        let d = DvDuration {
            type_tag: TypeTag::new(),
            accuracy_is_percent: None,
            accuracy: None,
            iso8601: Iso8601Duration {
                core: Iso8601TypeCore {
                    value: "P1DT2H".to_string(),
                },
            },
        };
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, r#"{"_type":"DV_DURATION","value":"P1DT2H"}"#);

        let back: DvDuration = serde_json::from_str(&json).unwrap();
        assert_eq!(back, d);
    }

    #[test]
    fn duration_magnitude_and_arithmetic_delegate_to_iso8601_parent() {
        let day = DvDuration {
            type_tag: TypeTag::new(),
            accuracy_is_percent: None,
            accuracy: None,
            iso8601: Iso8601Duration {
                core: Iso8601TypeCore {
                    value: "P1D".to_string(),
                },
            },
        };
        let half_day = DvDuration {
            type_tag: TypeTag::new(),
            accuracy_is_percent: None,
            accuracy: None,
            iso8601: Iso8601Duration {
                core: Iso8601TypeCore {
                    value: "PT12H".to_string(),
                },
            },
        };

        assert!(day.invariant_value_valid());
        assert_eq!(day.magnitude(), 86_400.0);
        assert_eq!(day.add(&half_day).iso8601.core.value, "PT129600S");
        assert_eq!(day.subtract(&half_day).iso8601.core.value, "PT43200S");
        assert_eq!(half_day.negative().iso8601.core.value, "-PT43200S");
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.date_time — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_duration.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master07-date_time_package.adoc §Class Descriptions / dv_duration.adoc §DV_DURATION Class
//   confidence: medium
//   todos: 4
//   note: the Section 7.2-named multiple-inheritance hazard (DV_AMOUNT + Iso8601_duration, genuinely disjoint parent state, contrast the value-mixin-only MI on DV_DATE/DV_TIME/DV_DATE_TIME). add/subtract/multiply/negative/magnitude/less_than and value validity delegate to the implemented Iso8601Duration methods while preserving receiver accuracy fields; from_iso8601 constructor added for the DV_TEMPORAL diff impls. The 4 remaining TODO(port) are all DV_AMOUNT/DV_ORDERED state owned by the concurrent quantity cluster (P17 wiring): the DvAmountData embedding reconciliation, unknown_accuracy_value sentinel, valid_percentage, and normal_status (still a todo!() with no backing DV_ORDERED field). less_than's Post_result wording is the one internally-consistent case in this package (contrast the three DvDate/DvTime/DvDateTime PORT NOTEs). P4: Serialize/Deserialize added; both Option fields skip when None; `iso8601` flattened over Iso8601Duration's own flattened core, so `value` sits flat — wire shape {"_type":"DV_DURATION","value":"P1DT2H"}; ADR-002 self-tagging applied (TypeTag<Self> first field + TypeName from TYPE_NAME); in-file round-trip test pins the flat shape.
// ─────────────────────────────────────────────
