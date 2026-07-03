//! `DV_QUANTIFIED` — abstract class defining the concept of true quantified
//! values.
//!
//! openEHR class: `DV_QUANTIFIED` (abstract), package
//! `rm.data_types.quantity`.
//! Inherits: `DV_ORDERED`.
//!
//! Abstract class defining the concept of true quantified values, i.e.
//! values which are not only ordered, but which have a precise magnitude.
use super::dv_ordered::{DvOrderedApi, DvOrderedData};
use openehr_foundation::primitive_types::ordered_numeric::OrderedNumeric;
use serde::{Deserialize, Serialize};

/// Shared attribute state of `DV_QUANTIFIED` and its descendants.
///
/// Per ADR-001 §3, embedded by the concrete/abstract types that inherit
/// `DV_QUANTIFIED` (`DV_AMOUNT`, `DV_ABSOLUTE_QUANTITY`, and transitively
/// `DV_QUANTITY`/`DV_COUNT`/`DV_PROPORTION`), rather than duplicated flat at
/// each level.
///
/// `T: DvOrderedApi` threads the same F-bounded self-type as
/// `DvOrderedData<T>` (see `dv_ordered.rs`), since `DV_QUANTIFIED`'s
/// `Inherit` row is `DV_ORDERED` and therefore also carries the
/// self-referential `normal_range`/`other_reference_ranges` attributes one
/// level further down the chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvQuantifiedData<T: DvOrderedApi> {
    /// Embedded `DV_ORDERED` parent state.
    #[serde(flatten)]
    pub ordered: DvOrderedData<T>,

    /// `magnitude_status`: `String` (0..1).
    ///
    /// Optional status of magnitude with values:
    ///
    /// * `"="`  : magnitude is a point value
    /// * `"<"`  : value is < magnitude
    /// * `">"`  : value is > magnitude
    /// * `"<="` : value is <= magnitude
    /// * `">="` : value is >= magnitude
    /// * `"~"`  : value is approximately magnitude
    ///
    /// If not present, assumed meaning is `"="`.
    ///
    /// Invariant `Magnitude_status_valid`: `magnitude_status /= Void
    /// implies valid_magnitude_status (magnitude_status)`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magnitude_status: Option<String>,

    /// `accuracy`: `Any` (0..1).
    ///
    /// Accuracy of measurement. Exact form of expression determined in
    /// descendants.
    ///
    /// PORT NOTE: the spec types this attribute `Any` at the abstract
    /// `DV_QUANTIFIED` level — genuinely open, since the concrete
    /// representation differs per descendant (`DV_AMOUNT` redefines it to
    /// `Real`, `DV_ABSOLUTE_QUANTITY` redefines it to `DV_AMOUNT`). Per the
    /// spec's own commentary ("Logically, an accuracy attribute should also
    /// be included in `DV_QUANTIFIED`, but as its modelling is different in
    /// the subtypes in a way that does not easily lend itself to a common
    /// ancestor, it is only included in the subtypes"), this field is
    /// **not actually instantiated** by any concrete leaf in this package —
    /// `DV_AMOUNT` and `DV_ABSOLUTE_QUANTITY` each declare their own
    /// concretely-typed `accuracy` field instead of using this one (see
    /// `dv_amount.rs`, `dv_absolute_quantity.rs`).
    ///
    /// TODO(port): a genuinely open `Any`-typed value has no faithful
    /// non-trait-object Rust representation, and a trait object here would
    /// need a hand-rolled `PartialEq`/`Clone` for a field no concrete leaf
    /// in this package ever populates. Represented as `Option<String>` — a
    /// deliberately inert stand-in, never intended to hold a real value —
    /// pending confirmation that this field is dead in every descendant, at
    /// which point it should be dropped from `DvQuantifiedData` entirely.
    /// Flagged as a structural ambiguity rather than silently resolved.
    /// `skip_serializing_if` added regardless (P4) so this dead field never
    /// emits a stray `null` in canonical JSON if a caller does populate it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<String>,
}

impl<T: DvOrderedApi> DvQuantifiedData<T> {
    /// `Magnitude_status_valid` class invariant, as a working method per
    /// ADR-003 decision 8 (invariants become `is_valid()`-family methods):
    ///
    /// `magnitude_status /= Void implies valid_magnitude_status
    /// (magnitude_status)`.
    pub fn invariant_magnitude_status_valid(&self) -> bool {
        self.magnitude_status
            .as_deref()
            .is_none_or(valid_magnitude_status)
    }
}

/// `valid_magnitude_status(): Boolean` — module-level body shared by the
/// [`DvQuantifiedApi::valid_magnitude_status`] trait default and the
/// [`DvQuantifiedData::invariant_magnitude_status_valid`] invariant method.
///
/// Spec `Post`: `Result = s in {"=", "<", ">", "<=", ">=", "~"}`.
pub fn valid_magnitude_status(s: &str) -> bool {
    matches!(s, "=" | "<" | ">" | "<=" | ">=" | "~")
}

/// Behaviour trait shared by every `DV_QUANTIFIED` descendant.
///
/// Extends [`DvOrderedApi`] (`DV_QUANTIFIED` inherits `DV_ORDERED`) with the
/// quantified-specific members.
pub trait DvQuantifiedApi<T: OrderedNumeric>: DvOrderedApi {
    /// `magnitude_status`: optional status of magnitude.
    fn magnitude_status(&self) -> Option<&str>;

    /// `valid_magnitude_status(): Boolean`.
    ///
    /// Test whether a string value is one of the valid values for the
    /// magnitude_status attribute.
    ///
    /// Spec `Post`: `Result = s in {"=", "<", ">", "<=", ">=", "~"}`.
    ///
    /// PORT NOTE: the spec signature/description here is ambiguous about
    /// whether `valid_magnitude_status` is a free-standing class query
    /// taking an explicit parameter `s`, or an instance query testing
    /// `self.magnitude_status`. The signature row itself shows no
    /// parameter (`valid_magnitude_status (): Boolean`), but the
    /// postcondition references a free variable `s` not bound anywhere in
    /// the row. Transcribed here as a free function taking an explicit
    /// `&str` parameter (matching the postcondition's `s`), which also
    /// matches how `Magnitude_status_valid`'s own invariant calls it:
    /// `valid_magnitude_status (magnitude_status)`. Delegates to the
    /// module-level [`valid_magnitude_status`] so the
    /// `Magnitude_status_valid` invariant on [`DvQuantifiedData`] (which is
    /// not itself a `DvQuantifiedApi` implementor) shares the same body.
    fn valid_magnitude_status(s: &str) -> bool
    where
        Self: Sized,
    {
        valid_magnitude_status(s)
    }

    /// `magnitude(): Ordered_Numeric` (abstract).
    ///
    /// Effective magnitude of the quantified value. Effected in
    /// descendants; type parameterized here as `T: OrderedNumeric` per
    /// ADR-001 §5, since the concrete magnitude type varies per descendant
    /// (`Integer64` for `DV_COUNT`, `f64`/`Real` for `DV_QUANTITY`, etc.).
    fn magnitude(&self) -> T;

    /// `accuracy_unknown(): Boolean`.
    ///
    /// True if accuracy is not known, e.g. due to not being recorded or
    /// discernable.
    ///
    /// PORT NOTE: declared abstract on `DV_QUANTIFIED` per the class
    /// description's own text ("An abstract Boolean feature
    /// `accuracy_unknown` is defined in the parent class `DV_QUANTIFIED`
    /// ... implemented in the respective descendants by concrete functions
    /// that check for the special values"), though the per-class table
    /// marks it `1..1` without an explicit `(abstract)` tag. No default
    /// body is given here since the special-value convention differs per
    /// descendant (`DV_AMOUNT` uses `-1`, `DV_ABSOLUTE_QUANTITY` uses a
    /// `Void`/`None` accuracy).
    fn accuracy_unknown(&self) -> bool;

    /// `is_equal(other: DV_QUANTIFIED) -> Boolean` (effected).
    ///
    /// Return `true` if this `DV_QUANTIFIED` is considered equal to
    /// `other`.
    ///
    /// PORT NOTE: the spec types `other` as the abstract `DV_QUANTIFIED`
    /// itself; narrowed to `&Self` here per the recurring pattern (see
    /// `dv_ordered.rs`'s `is_strictly_comparable_to` PORT NOTE for the same
    /// rationale).
    fn is_equal_quantified(&self, other: &Self) -> bool
    where
        Self: Sized;

    /// `less_than` __alias__ `"<"` `(other: DV_QUANTIFIED) -> Boolean`
    /// (effected).
    ///
    /// True if `other` is less than this Quantified object. Based on
    /// comparison of `magnitude`.
    ///
    /// Spec `Post_result`: `Result = magnitude < other.magnitude`.
    fn less_than_quantified(&self, other: &Self) -> bool
    where
        Self: Sized,
    {
        self.magnitude().less_than(&other.magnitude())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::quantity::dv_count::DvCount;
    use crate::data_types::quantity::dv_ordered::DvOrderedData;

    fn quantified_data(magnitude_status: Option<&str>) -> DvQuantifiedData<DvCount> {
        DvQuantifiedData {
            ordered: DvOrderedData {
                normal_status: None,
                normal_range: None,
                other_reference_ranges: None,
            },
            magnitude_status: magnitude_status.map(str::to_string),
            accuracy: None,
        }
    }

    /// Spec `Post`: `Result = s in {"=", "<", ">", "<=", ">=", "~"}`.
    #[test]
    fn valid_magnitude_status_accepts_exactly_the_six_spec_values() {
        for status in ["=", "<", ">", "<=", ">=", "~"] {
            assert!(valid_magnitude_status(status), "{status:?} must be valid");
        }
        for status in ["", "==", "=<", "≈", "approx", " ~"] {
            assert!(
                !valid_magnitude_status(status),
                "{status:?} must be invalid"
            );
        }
    }

    /// `Magnitude_status_valid`: `magnitude_status /= Void implies
    /// valid_magnitude_status (magnitude_status)`.
    #[test]
    fn magnitude_status_invariant_holds_for_absent_and_valid_values() {
        assert!(quantified_data(None).invariant_magnitude_status_valid());
        assert!(quantified_data(Some("~")).invariant_magnitude_status_valid());
        assert!(!quantified_data(Some("about")).invariant_magnitude_status_valid());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_quantified.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / dv_quantified.adoc §DV_QUANTIFIED Class
//   confidence: medium
//   todos: 1
//   note: accuracy: Any is a genuinely open abstract field the spec itself says is "only included in the subtypes" — represented as an inert Option<String> stand-in expected to stay unused (flagged, the one remaining TODO); valid_magnitude_status's signature/postcondition mismatch (empty parens vs a free variable s) resolved by reading it as a parameterized query, now shared as a module-level fn; Magnitude_status_valid invariant implemented as invariant_magnitude_status_valid() per ADR-003 §8, unit-tested. P4: DvQuantifiedData<T> derives Serialize/Deserialize; `ordered` carries #[serde(flatten)] (confirmed against DV_QUANTITY's own flat schema shape); both Option fields skip when None.
// ─────────────────────────────────────────────
