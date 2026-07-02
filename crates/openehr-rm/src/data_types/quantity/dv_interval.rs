//! `DV_INTERVAL<T>` — generic class defining an interval (range) of a
//! comparable type.
//!
//! openEHR class: `DV_INTERVAL<T>`, package `rm.data_types.quantity`.
//! Inherits: `DATA_VALUE`, `Interval<T>` (BASE foundation_types).
//!
//! Generic class defining an interval (i.e. range) of a comparable type. An
//! interval is a contiguous subrange of a comparable base type. Used to
//! define intervals of dates, times, quantities (whose units match) and so
//! on. The type parameter, `T`, must be a descendant of the type
//! `DV_ORDERED`, which is necessary (but not sufficient) for instances to be
//! compared (`strictly_comparable` is also needed).
//!
//! Without the `DV_INTERVAL` class, quite a few more `DV_` classes would be
//! needed to express logical intervals, namely interval versions of all the
//! date/time classes, and of quantity classes. Further, it allows the
//! semantics of intervals to be stated in one place unequivocally, including
//! the conditions for strict comparison.
//!
//! The basic semantics are derived from the class `Interval<T>`, described
//! in the support RM.
use super::dv_ordered::DvOrderedApi;
// TODO(port): forward-references DATA_VALUE (rm.data_types.basic), not yet
// transcribed by the sibling package agent covering `data_types::basic`.
use crate::data_types::data_value::DataValue;
use openehr_foundation::interval::interval::Interval;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class, single-sourced
/// into the [`TypeName`] impl below (ADR-002).
pub const TYPE_NAME: &str = "DV_INTERVAL";

/// `DV_INTERVAL<T>` inherits both `DATA_VALUE` and `Interval<T>` (BASE
/// foundation_types) per its `Inherit` row — a multiple-inheritance case
/// (ADR-001 §2/§3: composition of fields from all parents). The
/// `Interval<T>` parent is embedded by value as `range` rather than
/// flattened, matching the same composition shape already used by
/// `PointInterval<T>`/`ProperInterval<T>`
/// (`openehr_foundation::interval::{point_interval,proper_interval}`), so
/// `DV_INTERVAL<T>` stays structurally consistent with its own foundation
/// ancestor rather than duplicating the six `Interval<T>` attributes flat.
///
/// The class's own attribute table in the spec lists no additional
/// attributes of its own beyond what it inherits — its only genuinely new
/// content is the `Limits_consistent` class invariant (see below), which
/// tightens `Interval<T>`'s own (weaker, TODO-pending) invariants by adding
/// the `is_strictly_comparable_to` requirement specific to `DV_ORDERED`
/// limits.
///
/// `T: DvOrderedApi` matches the spec's `T` constraint ("must be a
/// descendant of the type `DV_ORDERED`"), per ADR-001 §5 (constrained
/// generic → generic with trait bound). `DvOrderedApi: Ordered` (see
/// `dv_ordered.rs`), so `T: DvOrderedApi` also satisfies `Interval<T>`'s own
/// `T: Ordered` bound.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvInterval<T: DvOrderedApi> {
    /// Canonical `_type` discriminator (`"DV_INTERVAL"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    ///
    /// The function-path `default = "TypeTag::new"` form is mandatory on a
    /// generic container — bare `default` makes serde's derive add a
    /// spurious `T: Default` bound.
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<DvInterval<T>>,

    /// Embedded parent state from `Interval<T>` (BASE foundation_types); see
    /// the struct-level PORT NOTE for why this is composition rather than a
    /// flattened field list. Carries `lower`, `upper`, `lower_unbounded`,
    /// `upper_unbounded`, `lower_included`, `upper_included`.
    ///
    /// `#[serde(flatten)]` per this crate's established embedded-parent
    /// convention (`DvCodedText.text`, `DvUri.uri`), so the wire shape is
    /// `{"_type":"DV_INTERVAL","lower":…,"upper":…,"lower_included":…,…}`
    /// with the six `Interval<T>` fields at top level, not nested under an
    /// `"interval"`/`"range"` key.
    ///
    /// PORT NOTE: the previously-flagged cross-crate blocker is closed —
    /// `openehr_foundation::interval::interval::Interval<T>` now derives
    /// `Serialize`/`Deserialize` with `skip_serializing_if` on the optional
    /// `lower`/`upper` limits, so nothing blocks this flatten.
    #[serde(flatten)]
    pub range: Interval<T>,
}

/// ADR-002: `_type` string for `DV_INTERVAL`, single-sourced from
/// [`TYPE_NAME`]. The impl repeats the struct's own declared
/// `T: DvOrderedApi` bound (required for the type `DvInterval<T>` to be
/// well-formed) but deliberately adds **no** further bounds — in particular
/// no `T: Serialize`/`T: Default` — so the tag never constrains the generic
/// parameter beyond the struct itself.
impl<T: DvOrderedApi> TypeName for DvInterval<T> {
    const NAME: &'static str = TYPE_NAME;
}

impl<T: DvOrderedApi> DvInterval<T> {
    // PORT NOTE: `DV_INTERVAL` declares no functions of its own in the
    // per-class table beyond what `Interval<T>` already provides
    // (`has`/`intersects`/`contains`, all still `todo!()` at the foundation
    // layer pending the `Interval::has` postcondition ambiguity — see
    // `openehr_foundation::interval::interval::Interval::has`). No new
    // methods are added here; callers reach `self.range.has(...)` etc.
    // directly until that foundation gap is resolved.
}

// TODO(port): `Limits_consistent` class invariant is not yet encoded as a
// `Validate` impl, per `.claude/rules/rm-transcription.md`'s "Invariants"
// section — recorded here as a documented TODO rather than silently
// omitted:
//
// `Limits_consistent`: `(not upper_unbounded and not lower_unbounded)
// implies (lower.is_strictly_comparable_to(upper) and lower <= upper)`
//
// This tightens (rather than duplicates) `Interval<T>`'s own
// `Limits_comparable`/`Limits_consistent` invariants (see
// `openehr_foundation::interval::interval::Interval`, which already has a
// TODO for those pending `Ordered::strictly_comparable_to` not existing on
// the `Ordered` trait) — at the `DV_INTERVAL<T: DV_ORDERED>` level the
// comparability check is `DvOrderedApi::is_strictly_comparable_to`
// specifically, not a BASE-wide `Ordered::strictly_comparable_to`.

// PORT NOTE: `DATA_VALUE` (the other half of `DV_INTERVAL`'s `Inherit` row)
// is not yet embedded here — it is owned by the sibling `data_types::basic`
// package (not yet transcribed in this worktree). Expected to be composed
// alongside `range: Interval<T>` once that package lands, per the same
// multi-parent pattern noted in `dv_ordered.rs`.
#[allow(unused_imports)]
use DataValue as _DataValueForwardRef;

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_interval.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / dv_interval.adoc §DV_INTERVAL Class
//   confidence: high
//   todos: 2
//   note: Limits_consistent invariant recorded but not enforced (needs a Validate framework); DATA_VALUE parent not yet embedded pending sibling data_types::basic package landing. Interval<T>'s own has/intersects/contains remain todo!() at the foundation layer, inherited transitively. P4/ADR-002: self-tags via TypeTag<DvInterval<T>> first field (function-path default, no extra bounds on T in the TypeName impl beyond the struct's own DvOrderedApi); `range` carries #[serde(flatten)], schema-verified (six Interval fields sit flat beside _type); foundation Interval<T> now derives serde, flatten unblocked.
// ─────────────────────────────────────────────
