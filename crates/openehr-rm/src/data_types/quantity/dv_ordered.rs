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
use crate::data_types::data_value::DataValue;
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
use openehr_foundation::primitive_types::any::Any;
use openehr_foundation::primitive_types::ordered::Ordered;
use openehr_terminology::{CodeSetAccess, OpenehrCodeSetIdentifiers, TerminologyService};
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// PORT NOTE: no explicit `#[serde(bound = ...)]` — verified by direct
// experiment that serde's derive auto-generates the correct `T: Serialize`
// / `T: Deserialize<'de>` bound per field type (here, through `DvInterval<T>`
// and `ReferenceRange<T>`) without needing an override, as long as the
// struct's own `T: DvOrderedApi` bound is preserved (it is, unchanged
// below). Use the minimal bound only if a future compile actually demands
// one — do not add `#[serde(bound = "T: DvOrderedApi + Serialize")]`
// speculatively.
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normal_status: Option<CodePhrase>,

    /// `normal_range`: `DV_INTERVAL<T>` (0..1).
    ///
    /// Optional normal range. Boxed per the recursive-containment rule
    /// (`DV_INTERVAL<T>` embeds `Interval<T>` and, via `REFERENCE_RANGE`,
    /// can transitively reference further `DV_ORDERED` structure).
    ///
    /// PORT NOTE: `skip_serializing_if` only, deliberately **without** a
    /// `default` sub-attribute — verified by direct experiment that adding
    /// `default` here spuriously requires `T: Default` to derive
    /// `Deserialize`, once this struct is reached through a
    /// `#[serde(flatten)]` chain (as it is, via `DvQuantifiedData::ordered`
    /// in `dv_quantified.rs`) — an absent `Option` field already
    /// deserializes to `None` without the redundant attribute. See the
    /// round-trip test in `dv_quantity.rs` for the full write-up.
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_reference_ranges: Option<Vec<ReferenceRange<T>>>,
}

impl<T: DvOrderedApi> DvOrderedData<T> {
    /// `Other_reference_ranges_validity` class invariant, as a working
    /// method per ADR-003 decision 8 (invariants become `is_valid()`-family
    /// methods):
    ///
    /// `other_reference_ranges /= Void implies not
    /// other_reference_ranges.is_empty`.
    pub fn invariant_other_reference_ranges_validity(&self) -> bool {
        self.other_reference_ranges
            .as_ref()
            .is_none_or(|ranges| !ranges.is_empty())
    }

    /// `Normal_status_validity` class invariant (terminology-bound, so it
    /// takes `&TerminologyService` per ADR-003 decision 8):
    ///
    /// `normal_status /= Void implies code_set
    /// (Code_set_id_normal_statuses).has_code (normal_status)`.
    pub fn invariant_normal_status_validity(&self, service: &TerminologyService) -> bool {
        self.normal_status.as_ref().is_none_or(|status| {
            service
                .code_set_for_id(OpenehrCodeSetIdentifiers::CODE_SET_ID_NORMAL_STATUSES)
                .is_some_and(|code_set| code_set.has_code(&status.code_string))
        })
    }

    /// `Is_simple_validity` class invariant:
    ///
    /// `(normal_range = Void and other_reference_ranges = Void) implies
    /// is_simple`.
    ///
    /// PORT NOTE: `is_simple` is a query on the *containing* `DV_ORDERED`
    /// value, which this embedded state struct cannot reach on its own —
    /// the owner is passed explicitly (`owner` embeds `self` per ADR-001
    /// §3's composition shape).
    pub fn invariant_is_simple_validity(&self, owner: &T) -> bool {
        !(self.normal_range.is_none() && self.other_reference_ranges.is_none()) || owner.is_simple()
    }

    /// `Normal_range_and_status_consistency` class invariant:
    ///
    /// `(normal_range /= Void and normal_status /= Void) implies
    /// (normal_status.code_string.is_equal ("N") xor not normal_range.has
    /// (self))`.
    ///
    /// PORT NOTE: same owner-passing shape as
    /// [`Self::invariant_is_simple_validity`] — the spec's `self` is the
    /// containing `DV_ORDERED` value.
    pub fn invariant_normal_range_and_status_consistency(&self, owner: &T) -> bool {
        match (&self.normal_range, &self.normal_status) {
            (Some(range), Some(status)) => (status.code_string == "N") ^ !range.has(owner),
            _ => true,
        }
    }
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
///
/// PORT NOTE: `#[serde(untagged)]` per ADR-002 — abstract-set enums carry no
/// tag of their own; the `_type` discriminator is emitted (and, on input,
/// dispatched) by each concrete variant payload's own self-tagging
/// `TypeTag<Self>` first field, whose `Deserialize` fails on a mismatched
/// `_type` string, making serde's untagged variant probing tag-driven rather
/// than structure-driven. The former `#[serde(tag = "_type")]` + per-variant
/// renames would duplicate the payload's own tag (`serde` would emit `_type`
/// twice once the payloads self-tag). The five variants owned by this
/// package (`Ordinal` … `Proportion`) self-tag as of this pass; the four
/// `date_time` variants are converted by the sibling package's own ADR-002
/// pass (mid-wave, they may briefly still dispatch structurally).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
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

    /// Accessor to the embedded `DvOrderedData<Self>` parent state, so the
    /// `is_simple`/`is_normal` default bodies below can reach
    /// `normal_range`/`other_reference_ranges` generically.
    ///
    /// PORT NOTE: not itself a spec function — the Rust bridge for the
    /// abstract parent's attribute access (Eiffel inheritance makes the
    /// attributes directly visible; composition does not). Every type that
    /// embeds `DvOrderedData<Self>` must override this to return
    /// `Some(&self...ordered)`; the default returns `None` (treated as "no
    /// reference ranges") only so that adding this method does not break
    /// sibling-package implementors mid-wave.
    ///
    /// TODO(port): the four `date_time` implementors (`DvDate`, `DvTime`,
    /// `DvDateTime`, `DvDuration`) are owned by the sibling
    /// `data_types::date_time` package and still inherit this `None`
    /// default — their `is_simple`/`is_normal` ignore any populated ranges
    /// until that package overrides the accessor (P17 make-it-compile
    /// triage checkpoint).
    fn ordered_data(&self) -> Option<&DvOrderedData<Self>>
    where
        Self: Sized,
    {
        None
    }

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
        match self.ordered_data() {
            Some(data) => data.normal_range.is_none() && data.other_reference_ranges.is_none(),
            None => true,
        }
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
    ///
    /// PORT NOTE: when the precondition is violated (neither `normal_range`
    /// nor `normal_status` present) the spec leaves the result undefined —
    /// `false` is returned here ("normality cannot be established"),
    /// documented rather than panicking. The `normal_status` check compares
    /// `code_string` against the literal `"N"` per `Post_status`; validity
    /// of the code against the `normal statuses` code set is the separate,
    /// terminology-bound `Normal_status_validity` invariant
    /// (`DvOrderedData::invariant_normal_status_validity`).
    fn is_normal(&self) -> bool
    where
        Self: Sized,
    {
        if let Some(range) = self
            .ordered_data()
            .and_then(|data| data.normal_range.as_deref())
        {
            range.has(self)
        } else if let Some(status) = self.normal_status() {
            status.code_string == "N"
        } else {
            false
        }
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

impl Any for DvOrdered {
    /// Value equality dispatched per variant; two different concrete
    /// `DV_ORDERED` subtypes are never equal in value.
    fn is_equal(&self, other: &Self) -> bool {
        match (self, other) {
            (DvOrdered::Ordinal(a), DvOrdered::Ordinal(b)) => a.is_equal(b),
            (DvOrdered::Scale(a), DvOrdered::Scale(b)) => a.is_equal(b),
            (DvOrdered::Quantity(a), DvOrdered::Quantity(b)) => a.is_equal(b),
            (DvOrdered::Count(a), DvOrdered::Count(b)) => a.is_equal(b),
            (DvOrdered::Proportion(a), DvOrdered::Proportion(b)) => a.is_equal(b),
            (DvOrdered::Date(a), DvOrdered::Date(b)) => a.is_equal(b),
            (DvOrdered::Time(a), DvOrdered::Time(b)) => a.is_equal(b),
            (DvOrdered::DateTime(a), DvOrdered::DateTime(b)) => a.is_equal(b),
            (DvOrdered::Duration(a), DvOrdered::Duration(b)) => a.is_equal(b),
            _ => false,
        }
    }

    fn type_of(&self) -> String {
        match self {
            DvOrdered::Ordinal(v) => v.type_of(),
            DvOrdered::Scale(v) => v.type_of(),
            DvOrdered::Quantity(v) => v.type_of(),
            DvOrdered::Count(v) => v.type_of(),
            DvOrdered::Proportion(v) => v.type_of(),
            DvOrdered::Date(v) => v.type_of(),
            DvOrdered::Time(v) => v.type_of(),
            DvOrdered::DateTime(v) => v.type_of(),
            DvOrdered::Duration(v) => v.type_of(),
        }
    }
}

impl Ordered for DvOrdered {
    /// `less_than` dispatched per matching variant.
    fn less_than(&self, other: &Self) -> bool {
        match (self, other) {
            (DvOrdered::Ordinal(a), DvOrdered::Ordinal(b)) => a.less_than(b),
            (DvOrdered::Scale(a), DvOrdered::Scale(b)) => a.less_than(b),
            (DvOrdered::Quantity(a), DvOrdered::Quantity(b)) => a.less_than(b),
            (DvOrdered::Count(a), DvOrdered::Count(b)) => a.less_than(b),
            (DvOrdered::Proportion(a), DvOrdered::Proportion(b)) => a.less_than(b),
            (DvOrdered::Date(a), DvOrdered::Date(b)) => a.less_than(b),
            (DvOrdered::Time(a), DvOrdered::Time(b)) => a.less_than(b),
            (DvOrdered::DateTime(a), DvOrdered::DateTime(b)) => a.less_than(b),
            (DvOrdered::Duration(a), DvOrdered::Duration(b)) => a.less_than(b),
            // PORT NOTE: ordering across mixed concrete DV_ORDERED subtypes
            // is spec-undefined — the `Pre_comparable` precondition
            // (`is_strictly_comparable_to (other)`) can never hold across
            // variants (see `is_strictly_comparable_to` below, which
            // returns `false` for exactly these pairs), so the spec places
            // no obligation on this branch. `false` is returned as the
            // total-function completion: a caller that honours the
            // precondition never reaches it, and a caller that does not
            // gets a stable non-ordering rather than a panic.
            _ => false,
        }
    }
}

impl DvOrderedApi for DvOrdered {
    fn normal_status(&self) -> Option<&CodePhrase> {
        match self {
            DvOrdered::Ordinal(v) => v.normal_status(),
            DvOrdered::Scale(v) => v.normal_status(),
            DvOrdered::Quantity(v) => DvOrderedApi::normal_status(v),
            DvOrdered::Count(v) => DvOrderedApi::normal_status(v),
            DvOrdered::Proportion(v) => DvOrderedApi::normal_status(v),
            DvOrdered::Date(v) => v.normal_status(),
            DvOrdered::Time(v) => v.normal_status(),
            DvOrdered::DateTime(v) => v.normal_status(),
            DvOrdered::Duration(v) => v.normal_status(),
        }
    }

    /// Matching variants delegate to the concrete type's own rule; mixed
    /// variants are never strictly comparable.
    ///
    /// PORT NOTE: the spec defines `is_strictly_comparable_to` only per
    /// matching concrete type (each `(effected)` row types `other` as the
    /// same concrete class, or narrows the comparison to same-class state
    /// such as `DV_QUANTITY.units`); no cross-subtype pair can satisfy any
    /// of those per-class rules, so heterogeneous variants return `false` —
    /// consistent with the class description's own example ("instances of
    /// `DV_QUANTITY` can only be compared if they measure the same kind of
    /// physical quantity", which a `DV_COUNT` never does).
    fn is_strictly_comparable_to(&self, other: &Self) -> bool {
        match (self, other) {
            (DvOrdered::Ordinal(a), DvOrdered::Ordinal(b)) => a.is_strictly_comparable_to(b),
            (DvOrdered::Scale(a), DvOrdered::Scale(b)) => a.is_strictly_comparable_to(b),
            (DvOrdered::Quantity(a), DvOrdered::Quantity(b)) => a.is_strictly_comparable_to(b),
            (DvOrdered::Count(a), DvOrdered::Count(b)) => a.is_strictly_comparable_to(b),
            (DvOrdered::Proportion(a), DvOrdered::Proportion(b)) => a.is_strictly_comparable_to(b),
            (DvOrdered::Date(a), DvOrdered::Date(b)) => a.is_strictly_comparable_to(b),
            (DvOrdered::Time(a), DvOrdered::Time(b)) => a.is_strictly_comparable_to(b),
            (DvOrdered::DateTime(a), DvOrdered::DateTime(b)) => a.is_strictly_comparable_to(b),
            (DvOrdered::Duration(a), DvOrdered::Duration(b)) => a.is_strictly_comparable_to(b),
            _ => false,
        }
    }

    /// `is_simple`/`is_normal` dispatch per variant rather than inheriting
    /// the trait defaults — the enum cannot expose a single
    /// `DvOrderedData<DvOrdered>` (each variant embeds
    /// `DvOrderedData<Concrete>`), so `ordered_data()` keeps its `None`
    /// default here and these two overrides route to each concrete type's
    /// own working body instead.
    fn is_simple(&self) -> bool {
        match self {
            DvOrdered::Ordinal(v) => v.is_simple(),
            DvOrdered::Scale(v) => v.is_simple(),
            DvOrdered::Quantity(v) => v.is_simple(),
            DvOrdered::Count(v) => v.is_simple(),
            DvOrdered::Proportion(v) => v.is_simple(),
            DvOrdered::Date(v) => v.is_simple(),
            DvOrdered::Time(v) => v.is_simple(),
            DvOrdered::DateTime(v) => v.is_simple(),
            DvOrdered::Duration(v) => v.is_simple(),
        }
    }

    fn is_normal(&self) -> bool {
        match self {
            DvOrdered::Ordinal(v) => v.is_normal(),
            DvOrdered::Scale(v) => v.is_normal(),
            DvOrdered::Quantity(v) => v.is_normal(),
            DvOrdered::Count(v) => v.is_normal(),
            DvOrdered::Proportion(v) => v.is_normal(),
            DvOrdered::Date(v) => v.is_normal(),
            DvOrdered::Time(v) => v.is_normal(),
            DvOrdered::DateTime(v) => v.is_normal(),
            DvOrdered::Duration(v) => v.is_normal(),
        }
    }
}

// PORT NOTE: the four class invariants are implemented as working
// `invariant_*` methods on `DvOrderedData<T>` above, per ADR-003 decision 8
// (invariants become `is_valid()`-family methods now; the walker/accumulator
// `Validate` framework remains the P11 deliverable). The terminology-bound
// `Normal_status_validity` takes `&TerminologyService`; the two invariants
// whose spec text references the containing value (`is_simple`, `has
// (self)`) take the owner explicitly.

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::quantity::dv_count::DvCount;
    use crate::data_types::quantity::dv_quantity::DvQuantity;
    use crate::data_types::text::dv_text::DvText;
    use openehr_base::identification::object_id::ObjectIdData;
    use openehr_base::identification::terminology_id::TerminologyId;
    use openehr_foundation::interval::interval::Interval;
    use openehr_foundation::primitive_types::real::Real;
    use openehr_foundation::serde_support::TypeTag;

    fn code_phrase(code: &str) -> CodePhrase {
        CodePhrase {
            type_tag: TypeTag::new(),
            terminology_id: TerminologyId {
                type_tag: TypeTag::new(),
                object_id: ObjectIdData {
                    value: "openehr".to_string(),
                },
            },
            code_string: code.to_string(),
            preferred_term: None,
        }
    }

    fn count(magnitude: i64) -> DvCount {
        let mut count = count_with(magnitude, None, None);
        count.amount.quantified.ordered.other_reference_ranges = None;
        count
    }

    fn count_with(
        magnitude: i64,
        normal_status: Option<&str>,
        normal_range: Option<(i64, i64)>,
    ) -> DvCount {
        DvCount {
            type_tag: TypeTag::new(),
            amount: crate::data_types::quantity::dv_amount::DvAmountData {
                quantified: crate::data_types::quantity::dv_quantified::DvQuantifiedData {
                    ordered: DvOrderedData {
                        normal_status: normal_status.map(code_phrase),
                        normal_range: normal_range
                            .map(|(lower, upper)| Box::new(count_interval(lower, upper))),
                        other_reference_ranges: None,
                    },
                    magnitude_status: None,
                    accuracy: None,
                },
                accuracy_is_percent: None,
                accuracy: None,
            },
            magnitude,
        }
    }

    fn count_interval(lower: i64, upper: i64) -> DvInterval<DvCount> {
        DvInterval {
            type_tag: TypeTag::new(),
            range: Interval {
                lower: Some(count(lower)),
                upper: Some(count(upper)),
                lower_unbounded: false,
                upper_unbounded: false,
                lower_included: true,
                upper_included: true,
            },
        }
    }

    fn reference_range(lower: i64, upper: i64) -> ReferenceRange<DvCount> {
        ReferenceRange {
            type_tag: TypeTag::new(),
            meaning: DvText::Text {
                type_tag: TypeTag::new(),
                data: crate::data_types::text::dv_text::DvTextData {
                    value: "therapeutic".to_string(),
                    hyperlink: None,
                    formatting: None,
                    mappings: None,
                    language: None,
                    encoding: None,
                },
            },
            range: count_interval(lower, upper),
        }
    }

    /// Spec: "True if this quantity has no reference ranges."
    #[test]
    fn is_simple_reflects_absence_of_both_range_attributes() {
        assert!(count(5).is_simple());
        assert!(!count_with(5, None, Some((0, 10))).is_simple());

        let mut with_other = count(5);
        with_other.amount.quantified.ordered.other_reference_ranges =
            Some(vec![reference_range(0, 10)]);
        assert!(!with_other.is_simple());
    }

    /// `Post_range`: `normal_range /= Void implies Result =
    /// normal_range.has (self)`.
    #[test]
    fn is_normal_uses_normal_range_when_present() {
        assert!(count_with(5, None, Some((0, 10))).is_normal());
        assert!(!count_with(50, None, Some((0, 10))).is_normal());
        // normal_range wins over normal_status when both are present.
        assert!(count_with(5, Some("LL"), Some((0, 10))).is_normal());
    }

    /// `Post_status`: `normal_status /= Void implies
    /// normal_status.code_string.is_equal ("N")`.
    #[test]
    fn is_normal_falls_back_to_the_normal_status_marker() {
        assert!(count_with(5, Some("N"), None).is_normal());
        assert!(!count_with(5, Some("HH"), None).is_normal());
    }

    /// `Pre`: `normal_range /= Void or normal_status /= Void` — violated
    /// precondition yields the documented `false`.
    #[test]
    fn is_normal_without_range_or_status_is_false() {
        assert!(!count(5).is_normal());
    }

    /// `Other_reference_ranges_validity`: present implies non-empty.
    #[test]
    fn other_reference_ranges_validity_invariant() {
        let mut data = count(5).amount.quantified.ordered;
        assert!(data.invariant_other_reference_ranges_validity());
        data.other_reference_ranges = Some(Vec::new());
        assert!(!data.invariant_other_reference_ranges_validity());
        data.other_reference_ranges = Some(vec![reference_range(0, 10)]);
        assert!(data.invariant_other_reference_ranges_validity());
    }

    /// `Normal_status_validity` against the bundled openEHR `normal
    /// statuses` code set (HHH..LLL series).
    #[test]
    fn normal_status_validity_invariant_uses_the_bundled_code_set() {
        let service = TerminologyService::bundled().expect("bundled terminology");
        for code in ["HHH", "HH", "H", "N", "L", "LL", "LLL"] {
            let data = count_with(5, Some(code), None).amount.quantified.ordered;
            assert!(
                data.invariant_normal_status_validity(service),
                "{code:?} is a member of the normal statuses code set"
            );
        }
        let bogus = count_with(5, Some("XX"), None).amount.quantified.ordered;
        assert!(!bogus.invariant_normal_status_validity(service));
        let absent = count(5).amount.quantified.ordered;
        assert!(absent.invariant_normal_status_validity(service));
    }

    /// `Normal_range_and_status_consistency`: `"N"` xor not-in-range.
    #[test]
    fn normal_range_and_status_consistency_invariant() {
        // "N" and in range: consistent.
        let consistent = count_with(5, Some("N"), Some((0, 10)));
        assert!(
            consistent
                .amount
                .quantified
                .ordered
                .invariant_normal_range_and_status_consistency(&consistent)
        );
        // "N" but out of range: inconsistent.
        let n_out = count_with(50, Some("N"), Some((0, 10)));
        assert!(
            !n_out
                .amount
                .quantified
                .ordered
                .invariant_normal_range_and_status_consistency(&n_out)
        );
        // Abnormal marker but in range: inconsistent.
        let ll_in = count_with(5, Some("LL"), Some((0, 10)));
        assert!(
            !ll_in
                .amount
                .quantified
                .ordered
                .invariant_normal_range_and_status_consistency(&ll_in)
        );
        // Abnormal marker and out of range: consistent.
        let ll_out = count_with(50, Some("LL"), Some((0, 10)));
        assert!(
            ll_out
                .amount
                .quantified
                .ordered
                .invariant_normal_range_and_status_consistency(&ll_out)
        );
        // Either attribute absent: invariant vacuously holds.
        let absent = count_with(5, Some("N"), None);
        assert!(
            absent
                .amount
                .quantified
                .ordered
                .invariant_normal_range_and_status_consistency(&absent)
        );
    }

    /// Cross-variant strict comparability is false; matching variants
    /// delegate to the concrete rule.
    #[test]
    fn enum_strict_comparability_dispatch() {
        let count_a = DvOrdered::Count(count(1));
        let count_b = DvOrdered::Count(count(2));
        assert!(count_a.is_strictly_comparable_to(&count_b));

        let quantity = DvOrdered::Quantity(DvQuantity {
            type_tag: TypeTag::new(),
            amount: crate::data_types::quantity::dv_amount::DvAmountData {
                quantified: crate::data_types::quantity::dv_quantified::DvQuantifiedData {
                    ordered: DvOrderedData {
                        normal_status: None,
                        normal_range: None,
                        other_reference_ranges: None,
                    },
                    magnitude_status: None,
                    accuracy: None,
                },
                accuracy_is_percent: None,
                accuracy: None,
            },
            magnitude: Real(1.0),
            precision: None,
            units: "kg".to_string(),
            units_system: None,
            units_display_name: None,
        });
        assert!(!count_a.is_strictly_comparable_to(&quantity));
        // Pre_comparable can never hold across variants, so the documented
        // total-function completion of less_than is false.
        assert!(!count_a.less_than(&quantity));
        assert!(count_a.less_than(&count_b));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types.quantity — docs/research/spec-cache/RM-1.1.0/uml_classes/dv_ordered.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-quantity_package.adoc §Class Descriptions / dv_ordered.adoc §DV_ORDERED Class
//   confidence: medium
//   todos: 5
//   note: F-bounded DvOrderedData<T: DvOrderedApi> is a judgment call (not a literal spec idiom) chosen so every concrete leaf's normal_range/other_reference_ranges narrow to Self uniformly. P5 spec-completion pass: is_simple/is_normal now have working default bodies over the new non-spec ordered_data() accessor (default None, overridden by every concrete in this package; date_time overrides pending in the sibling package — flagged TODO); the DvOrdered enum overrides is_simple/is_normal/is_strictly_comparable_to per variant, with cross-variant comparability false (per-class rules can never hold across subtypes) and cross-variant less_than false (Pre_comparable unsatisfiable — documented total-function completion); the four class invariants are working invariant_* methods on DvOrderedData per ADR-003 §8, the terminology-bound one taking &TerminologyService against the bundled normal statuses code set — all unit-tested. P4: DvOrderedData<T> derives Serialize/Deserialize with no explicit #[serde(bound)]; all three fields carry skip_serializing_if (deliberately no `default` sub-attribute; see dv_quantity.rs's round-trip test doc comment). ADR-002: DvOrderedData is abstract and carries NO _type tag; DvOrdered is #[serde(untagged)] — dispatch driven by each variant payload's own TypeTag first field.
// ─────────────────────────────────────────────
