//! `DV_ORDERED` — abstract class defining the concept of ordered values.
//!
//! openEHR class: `DV_ORDERED` (abstract), package `rm.data_types.quantity`.
//! Inherits: `DATA_VALUE`, `Ordered` (BASE foundation_types).
//!
//! Abstract class defining the concept of ordered values, which includes
//! ordinals as well as true quantities. It defines the functions `<` and
//! `is_strictly_comparable_to()`, the latter of which must evaluate to
//! `True` for instances being compared with the `<` function, or used as
//! limits in the `DV_INTERVAL<T>` class.
//!
//! Data value types which are to be used as limits in the `DV_INTERVAL<T>`
//! class must inherit from this class, and implement the function
//! `is_strictly_comparable_to()` to ensure that instances compare
//! meaningfully. For example, instances of `DV_QUANTITY` can only be
//! compared if they measure the same kind of physical quantity.
use super::dv_count::DvCount;
use super::dv_interval::DvInterval;
use super::dv_ordinal::DvOrdinal;
use super::dv_proportion::DvProportion;
use super::dv_quantity::DvQuantity;
use super::dv_scale::DvScale;
use super::reference_range::ReferenceRange;
// TODO(port): forward-references DATA_VALUE (rm.data_types.basic), not yet
// transcribed by the sibling package agent covering `data_types::basic` in a
// concurrent worktree; wire this `use` up once that module lands.
use crate::data_types::basic::data_value::DataValue;
// TODO(port): forward-references CODE_PHRASE (rm.data_types.text), not yet
// transcribed by the sibling package agent covering `data_types::text`.
use crate::data_types::text::code_phrase::CodePhrase;
// TODO(port): forward-references DV_DATE/DV_TIME/DV_DATE_TIME/DV_DURATION
// (rm.data_types.date_time), not yet transcribed by the sibling package
// agent covering `data_types::date_time`.
use crate::data_types::date_time::dv_date::DvDate;
use crate::data_types::date_time::dv_date_time::DvDateTime;
use crate::data_types::date_time::dv_duration::DvDuration;
use crate::data_types::date_time::dv_time::DvTime;
use openehr_foundation::primitive_types::ordered::Ordered;

/// Shared attribute state of `DV_ORDERED` and its descendants.
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct + marker
/// trait), every concrete `DV_ORDERED` subtype embeds this struct rather
/// than inheriting from it.
///
/// # Self-typed (F-bounded) generic parameter
///
/// The spec's own attribute table types `normal_range` and
/// `other_reference_ranges` as the *unparameterized* `DV_INTERVAL` /
/// `List<REFERENCE_RANGE>` at the `DV_ORDERED` level, but every concrete
/// leaf class transcribed in this package (`DV_QUANTITY`, `DV_COUNT`,
/// `DV_PROPORTION`) marks both attributes `(redefined)` in its own table,
/// narrowing them to `DV_INTERVAL<Self>` / `List<REFERENCE_RANGE<Self>>`
/// (e.g. `DV_QUANTITY.normal_range: DV_INTERVAL<DV_QUANTITY>`). `DV_ORDINAL`
/// and `DV_SCALE` show no `(redefined)` row for these two attributes, but
/// since a `DV_INTERVAL<T>`/`REFERENCE_RANGE<T>` is only ever meaningful
/// when compared against instances of the *same* concrete `DV_ORDERED`
/// subtype (see `DV_INTERVAL`'s own `Limits_consistent` invariant, which
/// requires `lower.is_strictly_comparable_to(upper)`), every concrete
/// descendant in practice needs its own range typed against itself.
///
/// This is modelled here as an **F-bounded generic**: `DvOrderedData<T>` is
/// generic over the very type that will embed it (`T: DvOrderedApi`), and
/// each concrete leaf instantiates `DvOrderedData<Self>`. This is a
/// judgment call beyond the literal per-attribute table (which does not
/// itself use this idiom, since Eiffel's covariant redefinition mechanism
/// covers the same ground natively); it keeps a single struct definition
/// serving both the "no `(redefined)` row" leaves (`DvOrdinal`, `DvScale`)
/// and the "explicit `(redefined)` row" leaves (`DvQuantity`, `DvCount`,
/// `DvProportion`) with the same self-typed shape, since both groups
/// resolve to the identical `DvOrderedData<Self>` instantiation. Flagged
/// here for reviewer scrutiny as a genuine hazard beyond ADR-001's existing
/// worked examples (which only cover `Self`-typed generics for `Interval<T>`
/// itself, not for an *embedded field inside* an abstract parent struct).
#[derive(Debug, Clone, PartialEq)]
pub struct DvOrderedData<T: DvOrderedApi> {
    /// `normal_status`: `CODE_PHRASE` (0..1).
    ///
    /// Optional normal status indicator of value with respect to normal
    /// range for this value. Often included by lab, even if the normal
    /// range itself is not included. Coded by ordinals in series HHH, HH,
    /// H, (nothing), L, LL, LLL; see openEHR terminology group
    /// `normal_status`.
    ///
    /// Invariant `Normal_status_validity`: `normal_status /= Void implies
    /// code_set (Code_set_id_normal_statuses).has_code (normal_status)`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl — needs `openehr-terminology`'s bundled code set access wired
    /// through, pending the `openehr-rm` → `openehr-terminology` dependency
    /// actually being exercised here.
    pub normal_status: Option<CodePhrase>,

    /// `normal_range`: `DV_INTERVAL<T>` (0..1).
    ///
    /// Optional normal range. Boxed per the recursive-containment rule
    /// (`DV_INTERVAL<T>` embeds `Interval<T>` and, via `REFERENCE_RANGE`,
    /// can transitively reference further `DV_ORDERED` structure).
    pub normal_range: Option<Box<DvInterval<T>>>,

    /// `other_reference_ranges`: `List<REFERENCE_RANGE<T>>` (0..1).
    ///
    /// Optional tagged other reference ranges for this value in its
    /// particular measurement context.
    ///
    /// Invariant `Other_reference_ranges_validity`:
    /// `other_reference_ranges /= Void implies not
    /// other_reference_ranges.is_empty`.
    ///
    /// PORT NOTE: transcribed as `Option<Vec<ReferenceRange<T>>>` rather
    /// than the foundation `List<T>` newtype — `List<T>` is the openEHR
    /// foundation-types container class itself (see
    /// `openehr_foundation::structure_types::list::List`), but every other
    /// RM attribute typed `List<X>` observed in sibling BASE transcriptions
    /// (e.g. `AUTHORED_RESOURCE`) uses a plain `Vec<X>` at the RM layer,
    /// reserving the `List<T>` wrapper for BASE foundation-types classes
    /// themselves. Followed here for consistency; flagged since this is the
    /// first RM attribute in this package literally typed `List<...>` in
    /// its own table.
    pub other_reference_ranges: Option<Vec<ReferenceRange<T>>>,
}

/// `DV_ORDERED` is abstract and used polymorphically wherever an attribute
/// is declared of that type — most notably `DV_INTERVAL<T: DV_ORDERED>` and
/// `REFERENCE_RANGE<T: DV_ORDERED>` (this package), plus every quantity-typed
/// RM attribute elsewhere in the model. Per ADR-001 §4 (closed subtype set →
/// enum), every concrete `DV_ORDERED` descendant across both this package
/// and the sibling `date_time` package is collected into this closed enum.
///
/// Variant ownership: `Ordinal`, `Scale`, `Quantity`, `Count`, and
/// `Proportion` are transcribed in this package; `Date`, `Time`, `DateTime`,
/// and `Duration` are owned by the sibling `date_time` package (transcribed
/// concurrently in a separate worktree) and referenced here purely as
/// forward `use` paths per the task's explicit variant list.
#[derive(Debug, Clone, PartialEq)]
pub enum DvOrdered {
    /// `DV_ORDINAL`.
    Ordinal(DvOrdinal),
    /// `DV_SCALE`.
    Scale(DvScale),
    /// `DV_QUANTITY`.
    Quantity(DvQuantity),
    /// `DV_COUNT`.
    Count(DvCount),
    /// `DV_PROPORTION`.
    Proportion(DvProportion),
    /// `DV_DATE` (sibling `date_time` package).
    Date(DvDate),
    /// `DV_TIME` (sibling `date_time` package).
    Time(DvTime),
    /// `DV_DATE_TIME` (sibling `date_time` package).
    DateTime(DvDateTime),
    /// `DV_DURATION` (sibling `date_time` package).
    Duration(DvDuration),
}

/// Behaviour trait shared by every `DV_ORDERED` descendant, exposing the
/// abstract class's attributes and functions uniformly whether the caller
/// holds a concrete type or a `DvOrdered` enum value.
///
/// `is_strictly_comparable_to` and `less_than` are declared `(abstract)`/
/// `(effected)` at the `DV_ORDERED` level with no common body — each
/// concrete descendant provides its own comparability rule (e.g.
/// `DV_QUANTITY` compares `units`, `DV_COUNT` always returns `true`,
/// `DV_ORDINAL`/`DV_SCALE` presumably compare `symbol`'s terminology, though
/// their per-class tables give no explicit body either). `is_simple` and
/// `is_normal` are given real default bodies here since the spec effects
/// them directly on `DV_ORDERED` in terms of the other abstract members.
pub trait DvOrderedApi: Ordered {
    /// `normal_status`: optional normal status indicator.
    fn normal_status(&self) -> Option<&CodePhrase>;

    /// `is_strictly_comparable_to(other: DV_ORDERED) -> Boolean` (abstract).
    ///
    /// Test if two instances are strictly comparable. Effected in
    /// descendants.
    ///
    /// PORT NOTE: the spec types `other` as the abstract `DV_ORDERED`
    /// itself, but a strict-comparability test is only meaningful between
    /// instances of the *same* concrete descendant (e.g. two `DV_QUANTITY`s
    /// with matching units) — narrowed to `&Self` here per the same pattern
    /// used throughout `openehr-foundation` (see `Interval::is_equal`'s PORT
    /// NOTE for the precedent).
    fn is_strictly_comparable_to(&self, other: &Self) -> bool;

    /// `is_simple(): Boolean`.
    ///
    /// True if this quantity has no reference ranges.
    fn is_simple(&self) -> bool
    where
        Self: Sized,
    {
        // TODO(port): needs `normal_range`/`other_reference_ranges`
        // accessors exposed generically on this trait (they currently live
        // on the concrete `DvOrderedData<Self>` embedded field, which this
        // trait cannot reach without an associated-type or accessor-method
        // bridge). Left as a documented gap rather than a guessed body.
        todo!(
            "DvOrderedApi::is_simple: needs generic normal_range/other_reference_ranges accessors"
        )
    }

    /// `is_normal(): Boolean`.
    ///
    /// Value is in the normal range, determined by comparison of the value
    /// to `normal_range` if present, or by the `normal_status` marker if
    /// present.
    ///
    /// Spec `Pre`: `normal_range /= Void or normal_status /= Void`.
    /// Spec `Post_range`: `normal_range /= Void implies Result =
    /// normal_range.has (self)`.
    /// Spec `Post_status`: `normal_status /= Void implies
    /// normal_status.code_string.is_equal ("N")`.
    fn is_normal(&self) -> bool
    where
        Self: Sized,
    {
        // TODO(port): same generic-accessor gap as `is_simple`, plus needs
        // `DV_INTERVAL::has` (itself `todo!()` pending `Interval::has`'s
        // ambiguous postcondition parenthesization — see
        // `openehr_foundation::interval::interval::Interval::has`) and
        // `CODE_PHRASE.code_string` (not yet transcribed).
        todo!(
            "DvOrderedApi::is_normal: needs generic range accessors, DV_INTERVAL::has, and CODE_PHRASE.code_string"
        )
    }

    /// `less_than` __alias__ `"<"` `(other: DV_ORDERED) -> Boolean`
    /// (effected).
    ///
    /// True if this Ordered object is less than `other`. Redefined in
    /// descendants.
    ///
    /// Spec `Pre_comparable`: `is_strictly_comparable_to (other)`.
    ///
    /// PORT NOTE: this is the same symbol as `Ordered::less_than` (the
    /// `DV_ORDERED` class itself inherits `Ordered` from BASE
    /// foundation_types per the class's `Inherit` row), so this trait
    /// requires `Ordered` as a supertrait rather than redeclaring
    /// `less_than` under a different name; concrete types implement
    /// `Ordered::less_than` directly and this trait adds no separate method
    /// for it.
    fn less_than_ordered(&self, other: &Self) -> bool
    where
        Self: Sized,
    {
        self.less_than(other)
    }
}

impl DvOrderedApi for DvOrdered {
    fn normal_status(&self) -> Option<&CodePhrase> {
        match self {
            DvOrdered::Ordinal(v) => v.normal_status(),
            DvOrdered::Scale(v) => v.normal_status(),
            DvOrdered::Quantity(v) => v.normal_status(),
            DvOrdered::Count(v) => v.normal_status(),
            DvOrdered::Proportion(v) => v.normal_status(),
            // TODO(port): DvDate/DvTime/DvDateTime/DvDuration are owned by
            // the sibling date_time package and not available in this
            // worktree; stubbed pending that package's landing.
            DvOrdered::Date(_)
            | DvOrdered::Time(_)
            | DvOrdered::DateTime(_)
            | DvOrdered::Duration(_) => {
                todo!(
                    "DvOrdered::normal_status: date_time package variants not yet transcribed in this worktree"
                )
            }
        }
    }

    fn is_strictly_comparable_to(&self, _other: &Self) -> bool {
        // TODO(port): dispatching this across mixed enum variants requires
        // deciding what "strictly comparable" means *across* concrete
        // DV_ORDERED subtypes (e.g. is a DvCount ever strictly comparable to
        // a DvQuantity?) — the spec only defines this per matching concrete
        // type. Left unresolved pending a cross-variant comparability rule.
        todo!(
            "DvOrdered::is_strictly_comparable_to: cross-variant comparability rule not specified"
        )
    }
}

// TODO(port): the four class invariants below are not yet encoded as a
// `Validate` impl, per `.claude/rules/rm-transcription.md`'s "Invariants"
// section — recorded here as documented TODOs rather than silently omitted.
//
// - `Other_reference_ranges_validity`: `other_reference_ranges /= Void
//   implies not other_reference_ranges.is_empty`
// - `Is_simple_validity`: `(normal_range = Void and other_reference_ranges
//   = Void) implies is_simple`
// - `Normal_status_validity`: `normal_status /= Void implies code_set
//   (Code_set_id_normal_statuses).has_code (normal_status)`
// - `Normal_range_and_status_consistency`: `(normal_range /= Void and
//   normal_status /= Void) implies (normal_status.code_string.is_equal
//   ("N") xor not normal_range.has (self))`

// PORT NOTE: `DATA_VALUE` (the other half of `DV_ORDERED`'s `Inherit` row,
// alongside `Ordered`) is not yet embedded on `DvOrderedData`/`DvOrdered`
// here — it is owned by the sibling `data_types::basic` package (not yet
// transcribed in this worktree). Each concrete leaf (`DvOrdinal`,
// `DvQuantity`, etc.) is expected to separately embed `DataValue` alongside
// `DvOrderedData<Self>` once that package lands, mirroring the same
// multi-parent composition already used for `Iso8601_type`
// (`openehr_foundation::time::iso8601_type`).
#[allow(unused_imports)]
use DataValue as _DataValueForwardRef;

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_ordered.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / dv_ordered.adoc §DV_ORDERED Class
//   confidence: medium
//   todos: 9
//   note: F-bounded DvOrderedData<T: DvOrderedApi> is a judgment call (not a literal spec idiom) chosen so every concrete leaf's normal_range/other_reference_ranges narrow to Self uniformly, whether or not the leaf's own table shows an explicit (redefined) row; is_simple/is_normal/is_strictly_comparable_to are stubbed pending generic range accessors and DATA_VALUE/CODE_PHRASE landing from sibling packages.
// ─────────────────────────────────────────────
