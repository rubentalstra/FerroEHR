//! `EVENT<T>` — abstract notion of a single event in a series.
//!
//! openEHR class: `EVENT<T>` (abstract, generic), package
//! `rm.data_structures.history`.
//!
//! Defines the abstract notion of a single event in a series. This class
//! is generic, allowing types to be generated which are locked to
//! particular spatial types, such as `EVENT<ITEM_LIST>`. Subtypes express
//! point or interval data.
//!
//! Inherits `LOCATABLE` (per the spec table) — **not** `PATHABLE` directly;
//! this is distinct from the settled `EVENT_CONTEXT`/`INSTRUCTION_DETAILS`/
//! `ISM_TRANSITION` watch-out in `.claude/rules/rm-transcription.md`, which
//! names those three specific classes as `PATHABLE`-not-`LOCATABLE`. `EVENT`
//! is not one of them; its own spec table states `Inherit: LOCATABLE`
//! explicitly.

use super::interval_event::IntervalEvent;
use super::point_event::PointEvent;
use crate::data_structures::item_structure::item_structure::ItemStructure;
// PORT NOTE: `DV_DATE_TIME`/`DV_DURATION` belong to `rm.data_types.date_time`,
// transcribed concurrently by a sibling agent; see `representation/element.rs`
// for the identical forward-reference rationale and assumed module path.
use crate::data_types::date_time::dv_date_time::DvDateTime;
use crate::data_types::date_time::dv_duration::DvDuration;
// PORT NOTE: `LOCATABLE` is owned by the `common` package cluster,
// transcribed concurrently; see `representation/item.rs` for the identical
// forward-reference rationale.
use crate::common::archetyped::locatable::LocatableData;

/// Shared attribute state of `EVENT<T>` and its descendants.
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct +
/// marker trait) and §5 (constrained generic). `T` corresponds to the
/// spec's own type parameter, constrained to `ITEM_STRUCTURE` by the
/// `data` attribute's declared type (`data: T`) together with the
/// package narrative's statement that `HISTORY<T -> ITEM_STRUCTURE>` locks
/// its `EVENT`s to the same `T`. The `T: Into<ItemStructure>` style bound
/// is not used here; instead `T` is bound directly to the closed
/// `ItemStructure` enum's own constituent types where a concrete `EVENT<T>`
/// is instantiated (e.g. `EVENT<ItemTree>`), matching how `HISTORY<T:
/// ITEM_STRUCTURE>` is transcribed in `history.rs`.
#[derive(Debug, Clone, PartialEq)]
pub struct EventData<T> {
    /// Inherited `LOCATABLE` state.
    ///
    /// TODO(port): forward reference; see `representation/item.rs`.
    pub locatable: LocatableData,

    /// `time`: time of this event. If the width is non-zero, it is the
    /// time point of the trailing edge of the event.
    ///
    /// Cardinality `1..1`.
    pub time: DvDateTime,

    /// `state`: optional state data for this event.
    ///
    /// Cardinality `0..1`.
    pub state: Option<ItemStructure>,

    /// `data`: the data of this event.
    ///
    /// Cardinality `1..1`. Spec type `T`, the generic parameter.
    pub data: T,
}

/// `EVENT<T>` is abstract in the spec and is used polymorphically wherever
/// an attribute or return type is declared `EVENT<T>` (e.g.
/// `HISTORY<T>.events: List<EVENT<T>>`). Per ADR-001 §4 (closed subtype
/// set → enum) combined with §5 (constrained generic), the two concrete
/// subtypes `POINT_EVENT<T>` and `INTERVAL_EVENT<T>` are collected into
/// this closed, still-generic `enum` so a field or return type can be
/// declared `Event<T>` exactly where the spec declares it `EVENT<T>`.
#[derive(Debug, Clone, PartialEq)]
pub enum Event<T> {
    /// `POINT_EVENT<T>`.
    Point(PointEvent<T>),
    /// `INTERVAL_EVENT<T>`.
    Interval(IntervalEvent<T>),
}

/// Marker/accessor trait shared by every `EVENT<T>` descendant, exposing
/// the abstract class's attributes and the `offset()` function uniformly
/// whether the caller holds a concrete type or an `Event<T>` enum value.
pub trait EventApi<T> {
    /// Access the shared `EVENT<T>` state.
    fn event_data(&self) -> &EventData<T>;

    /// `offset`: offset of this event from origin, computed as
    /// `time.diff(parent.origin)`.
    ///
    /// Postcondition `Post_condition`: `Result = time.diff(parent.origin)`.
    ///
    /// `parent` here is the `PATHABLE.parent()` reverse pointer up to the
    /// owning `HISTORY<T>` — per
    /// `.claude/rules/rm-transcription.md`/ADR-001 §8, this must be a
    /// `Weak<..>` or path-index, never an owning back-reference. `EVENT`
    /// inherits `LOCATABLE` (which itself inherits `PATHABLE`), so the
    /// `parent()` accessor is expected to live on the `LOCATABLE`/
    /// `PATHABLE` embedding once the `common` package lands; this method
    /// cannot be implemented until that back-reference mechanism is
    /// available, and is therefore left `todo!()` on every implementor
    /// rather than guessed at here on the trait.
    fn offset(&self) -> DvDuration {
        // TODO(port): needs the PATHABLE.parent() back-reference (Weak/
        // path-index, per the settled hazard) to reach the owning
        // HISTORY<T>.origin, plus DvDateTime::diff(). Both the parent
        // back-reference mechanism (common package) and DvDateTime's own
        // diff() (data_types package) are forward references pending
        // concurrent transcription.
        todo!(
            "offset(): needs PATHABLE.parent() back-reference to the owning HISTORY<T> and DvDateTime::diff()"
        )
    }
}

impl<T> EventApi<T> for Event<T> {
    fn event_data(&self) -> &EventData<T> {
        match self {
            Event::Point(v) => v.event_data(),
            Event::Interval(v) => v.event_data(),
        }
    }
}

// TODO(port): invariant `Offset_validity1`:
// `offset /= Void and then offset = time.diff(parent.origin)` — restates
// the `offset()` postcondition as a class invariant; deferred to the same
// `PATHABLE.parent()` + `DvDateTime::diff()` dependencies as `offset()`
// itself, once the RM `Validate` trait framework lands.

pub const TYPE_NAME: &str = "EVENT";

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.history §EVENT — docs/research/spec-cache/RM-1.1.0/uml_classes/event.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-history_package.adoc §Class Descriptions / event.adoc §EVENT Class
//   confidence: medium
//   todos: 3
//   note: EVENT inherits LOCATABLE per its own spec table (confirmed distinct from the PATHABLE-not-LOCATABLE watch-out class list); offset() and its restating invariant both block on the PATHABLE.parent() back-reference plus DvDateTime::diff(), neither of which has landed yet.
// ─────────────────────────────────────────────
