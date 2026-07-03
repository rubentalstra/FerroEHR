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
// PORT NOTE: `DV_DATE_TIME`/`DV_DURATION` belong to `rm.data_types.date_time`
// (now landed). `DvTemporal` supplies the `diff` operation `offset()`
// delegates to; the underlying ISO 8601 arithmetic is that package's P17
// deliverable (currently `todo!()`), so `offset()` type-checks and is wired
// correctly today and returns real values once the arithmetic lands.
use crate::data_types::date_time::dv_date_time::DvDateTime;
use crate::data_types::date_time::dv_duration::DvDuration;
use crate::data_types::date_time::dv_temporal::DvTemporal;
// PORT NOTE: `LOCATABLE` is owned by the `common` package cluster,
// transcribed concurrently; see `representation/item.rs` for the identical
// forward-reference rationale.
use crate::common::archetyped::locatable::LocatableData;
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventData<T> {
    /// Inherited `LOCATABLE` state.
    ///
    /// PORT NOTE: reconciled with `common::archetyped::locatable::LocatableData`
    /// (now landed) — no longer a forward reference.
    #[serde(flatten)]
    pub locatable: LocatableData,

    /// `time`: time of this event. If the width is non-zero, it is the
    /// time point of the trailing edge of the event.
    ///
    /// Cardinality `1..1`.
    pub time: DvDateTime,

    /// `state`: optional state data for this event.
    ///
    /// Cardinality `0..1`.
    #[serde(skip_serializing_if = "Option::is_none")]
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
// PORT NOTE: `#[serde(untagged)]` per ADR-002 — dispatch is driven by each
// variant payload's own `TypeTag` (`PointEvent`/`IntervalEvent` self-tag
// with `_type`), whose `Deserialize` fails on a mismatched `_type` string,
// so untagged probing is tag-driven rather than structure-driven. A
// struct-level `#[serde(tag = "_type")]` here would duplicate the payloads'
// own `_type` keys on the wire. Variant order lists the structurally richer
// payload first per ADR-002 (`IntervalEvent` requires `width` and
// `math_function`, which a bare `POINT_EVENT` payload lacks; the reverse
// probe would swallow an interval event into `Point` on tag-less input) —
// this inverts the spec's own POINT_EVENT-then-INTERVAL_EVENT listing
// order, deliberately.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Event<T> {
    /// `INTERVAL_EVENT<T>`.
    Interval(IntervalEvent<T>),
    /// `POINT_EVENT<T>`.
    Point(PointEvent<T>),
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
    /// PORT NOTE (reshaped for the reverse-pointer rule): the spec's
    /// parameterless `offset()` reaches `parent.origin` via the
    /// `PATHABLE.parent()` reverse pointer up to the owning `HISTORY<T>`. In
    /// this port a bare `EVENT` is not wired with an owning back-reference
    /// that can yield the parent's `origin` (per the settled rule the
    /// back-reference is a `Weak<dyn PathableApi>`, and `PathableApi`
    /// exposes no `origin`), so the origin is supplied explicitly by the
    /// caller — in practice the owning `HISTORY<T>`, which is the only place
    /// `origin` is meaningfully available. This mirrors the postcondition
    /// `time.diff(parent.origin)` exactly, with `parent.origin` passed in.
    /// See `History::offset_of` (`history.rs`) for the parent-driven form.
    fn offset(&self, origin: &DvDateTime) -> DvDuration {
        self.event_data().time.diff(origin)
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
// the `offset()` postcondition as a class invariant. It is definitionally
// satisfied by the `offset()` implementation above (`time.diff(origin)`);
// a standalone `Validate` check would re-run the same P17 `DvDateTime::diff`
// arithmetic, so it is deferred to the P11 `Validate` framework rather than
// duplicated here.

pub const TYPE_NAME: &str = "EVENT";

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.history §EVENT — docs/research/spec-cache/RM-1.1.0/uml_classes/event.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-history_package.adoc §Class Descriptions / event.adoc §EVENT Class
//   confidence: high
//   todos: 1
//   note: EVENT inherits LOCATABLE per its own spec table (confirmed distinct from the PATHABLE-not-LOCATABLE watch-out class list). offset() implemented as time.diff(origin): the spec's parameterless offset() reaches parent.origin via the PATHABLE.parent() back-reference, which a bare EVENT is not wired with here (PathableApi exposes no origin), so origin is passed in explicitly — in practice by the owning HISTORY (History::offset_of). The underlying DvDateTime::diff arithmetic is the data_types agent's P17 deliverable (currently todo!()), so offset() is wired correctly and returns real values once that lands. Offset_validity1 restates offset()'s postcondition, deferred to P11 Validate. P4/ADR-002: Event<T> enum is #[serde(untagged)] with richer Interval variant listed first (payload TypeTags drive dispatch); EventData<T> stays untagged (abstract).
// ─────────────────────────────────────────────
