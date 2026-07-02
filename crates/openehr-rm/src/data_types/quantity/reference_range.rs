//! `REFERENCE_RANGE<T>` — a named range associated with any `DV_ORDERED`
//! datum.
//!
//! openEHR class: `REFERENCE_RANGE<T>`, package `rm.data_types.quantity`.
//! Inherits: none listed (spec table has no `Inherit` row).
//!
//! Defines a named range to be associated with any `DV_ORDERED` datum. Each
//! such range is particular to the patient and context, e.g. sex, age, and
//! any other factor which affects ranges. May be used to represent normal,
//! therapeutic, dangerous, critical etc ranges.
use super::dv_interval::DvInterval;
use super::dv_ordered::DvOrderedApi;
// TODO(port): forward-references DV_TEXT (rm.data_types.text), not yet
// transcribed by the sibling package agent covering `data_types::text`.
use crate::data_types::text::dv_text::DvText;

/// Canonical `_type` discriminator string for this class in serialized
/// form (serde derives wait until P4 per ADR-001 "Refinements").
pub const TYPE_NAME: &str = "REFERENCE_RANGE";

/// `REFERENCE_RANGE<T>` has no `Inherit` row in its per-class table (unlike
/// `DV_INTERVAL<T>`, which inherits `DATA_VALUE` + `Interval<T>`); it is a
/// standalone generic struct carrying its two declared attributes directly.
///
/// `T: DvOrderedApi` matches the class description's "associated with any
/// `DV_ORDERED` datum" and the generic parameter written on `DV_INTERVAL<T>`
/// in `range`'s declared type — per ADR-001 §5 (constrained generic →
/// generic with trait bound), mirrored identically from `dv_interval.rs`.
#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceRange<T: DvOrderedApi> {
    /// `meaning`: `DV_TEXT` (1..1).
    ///
    /// Term whose value indicates the meaning of this range, e.g. normal,
    /// critical, therapeutic etc.
    pub meaning: DvText,

    /// `range`: `DV_INTERVAL<T>` (1..1).
    ///
    /// The data range for this meaning, e.g. critical etc.
    pub range: DvInterval<T>,
}

impl<T: DvOrderedApi> ReferenceRange<T> {
    /// `is_in_range(v: DV_ORDERED) -> Boolean`.
    ///
    /// Indicates if the value `v` is inside the range.
    ///
    /// PORT NOTE: the spec types `v` as the abstract `DV_ORDERED`, but a
    /// meaningful "in range" test can only compare `v` against this
    /// `ReferenceRange<T>`'s own `range: DV_INTERVAL<T>`, which is typed
    /// over the same concrete `T` — narrowed to `&T` here, per the same
    /// "concrete-type narrowing of an `Any`/ancestor-typed parameter"
    /// pattern used throughout `openehr-foundation` (see
    /// `Interval::is_equal`'s PORT NOTE).
    pub fn is_in_range(&self, v: &T) -> bool {
        // TODO(port): delegates to `DV_INTERVAL::has`, which itself
        // delegates to the foundation `Interval::has` — still `todo!()`
        // pending resolution of that method's ambiguous postcondition
        // parenthesization (see
        // `openehr_foundation::interval::interval::Interval::has`).
        self.range.range.has(v)
    }
}

// TODO(port): `Range_is_simple` class invariant is not yet encoded as a
// `Validate` impl, per `.claude/rules/rm-transcription.md`'s "Invariants"
// section — recorded here as a documented TODO rather than silently
// omitted:
//
// `Range_is_simple`: `(range.lower_unbounded or else range.lower.is_simple)
// and (range.upper_unbounded or else range.upper.is_simple)`
//
// PORT NOTE: `is_simple` here is `DvOrderedApi::is_simple` on the interval's
// own `lower`/`upper` limit values (both `T: DvOrderedApi`), which is itself
// currently `todo!()` at the `DV_ORDERED` level pending generic range
// accessors — see `dv_ordered.rs`. `or else` is Eiffel's short-circuit
// (semi-strict) disjunction; transcribed conceptually as ordinary boolean
// `||` once both operands are computable, since Rust's `||` is already
// short-circuiting.

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/reference_range.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / reference_range.adoc §REFERENCE_RANGE Class
//   confidence: high
//   todos: 3
//   note: is_in_range narrows the spec's abstract DV_ORDERED parameter to the concrete T (documented PORT NOTE); is_in_range's DV_INTERVAL::has delegation and the Range_is_simple invariant both remain unenforced pending Interval::has and a Validate framework respectively.
// ─────────────────────────────────────────────
