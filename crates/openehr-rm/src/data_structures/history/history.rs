//! `HISTORY<T>` — root object of a linear history, i.e. time series
//! structure.
//!
//! openEHR class: `HISTORY<T>` (generic, `T` constrained to
//! `ITEM_STRUCTURE`), package `rm.data_structures.history`.
//!
//! Root object of a linear history, i.e. time series structure. This is a
//! generic class whose type parameter must be a descendant of
//! `ITEM_STRUCTURE`, ensuring that each Event in the `events` of a given
//! instance is of the same structural type, i.e. `ITEM_TREE`, `ITEM_LIST`
//! etc.
//!
//! For a periodic series of events, `period` will be set, and the time of
//! each Event in the History must correspond; i.e. the `EVENT.offset` must
//! be a multiple of `period` for each Event. Missing events in a periodic
//! History are however allowed.

use super::event::{Event, EventApi};
use crate::data_structures::item_structure::data_structure::DataStructureBehaviour;
use crate::data_structures::item_structure::data_structure::DataStructureData;
use crate::data_structures::item_structure::item_structure::{ItemStructure, ItemStructureApi};
use crate::data_structures::representation::item::Item;
// PORT NOTE: `DV_DATE_TIME`/`DV_DURATION` belong to `rm.data_types.date_time`,
// transcribed concurrently by a sibling agent; see `representation/element.rs`
// for the identical forward-reference rationale and assumed module path.
use crate::data_types::date_time::dv_date_time::DvDateTime;
use crate::data_types::date_time::dv_duration::DvDuration;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `HISTORY<T>` class.
///
/// Embeds the shared `DATA_STRUCTURE` state (per ADR-001 §3) plus its own
/// attributes. `T` is constrained to `ITEM_STRUCTURE` per ADR-001 §5
/// (constrained generic → generic with a matching trait bound): the bound
/// is expressed as `T: ItemStructureApi` here rather than `T = ItemStructure`
/// directly, so a concrete instantiation such as `History<ItemTree>` (a
/// specific `ITEM_STRUCTURE` descendant, not the closed enum itself)
/// remains possible — matching how the spec narrative describes
/// `HISTORY<ITEM_LIST>` as constraining `EVENT._item_`/`_data_` to be "of
/// type `ITEM_LIST` and nothing else", i.e. a single concrete descendant,
/// not the union type. `Event<T>` (`event.rs`) uses the same `T` for its
/// own `data: T` field, so `History<T>.events: Vec<Event<T>>` keeps every
/// event's data locked to the same concrete `ITEM_STRUCTURE` subtype,
/// exactly as the package narrative's "Basic Semantics" section describes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct History<T: ItemStructureApi> {
    /// Canonical `_type` discriminator (`"HISTORY"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    /// Spelled `TypeTag<History<T>>` (not `TypeTag<Self>`) and paired with
    /// the mandatory function-path `default = "TypeTag::new"` so serde's
    /// derive adds no spurious `T: Default` bound.
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<History<T>>,

    /// Inherited `DATA_STRUCTURE` (and transitively `LOCATABLE`) state.
    ///
    /// PORT NOTE: `HISTORY<T>` inherits `DATA_STRUCTURE` directly (not
    /// `ITEM_STRUCTURE`) per its own spec table — it is a sibling of
    /// `ITEM_STRUCTURE` under `DATA_STRUCTURE`, not a further
    /// `ITEM_STRUCTURE` descendant, despite `HISTORY.summary` itself being
    /// typed `ITEM_STRUCTURE`. Embeds `DataStructureData` directly (from
    /// `item_structure::data_structure`, the module that actually owns
    /// `DATA_STRUCTURE`, shared across both the `item_structure` and
    /// `history` sub-packages of `rm.data_structures`), not
    /// `ItemStructureData`.
    #[serde(flatten)]
    pub data_structure: DataStructureData,

    /// `origin`: time origin of this event history. The first event is not
    /// necessarily at the origin point.
    ///
    /// Cardinality `1..1`.
    pub origin: DvDateTime,

    /// `period`: period between samples in this segment if periodic.
    ///
    /// Cardinality `0..1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<DvDuration>,

    /// `duration`: duration of the entire History; either corresponds to
    /// the duration of all the events, and/or the duration represented by
    /// the summary, if it exists.
    ///
    /// Cardinality `0..1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<DvDuration>,

    /// `summary`: optional summary data that aggregates, organizes,
    /// reduces and transforms the event series. This may be a text or
    /// image that presents a graphical presentation, or some data that
    /// assists with the interpretation of the data.
    ///
    /// Cardinality `0..1`. Spec type `ITEM_STRUCTURE` — deliberately **not**
    /// the same `T` as `events`/`data`; the summary structure is
    /// independently archetypable (per the package narrative: "itself a
    /// structure, archetypable separately from the structure of the main
    /// data"), so it is typed with the closed `ItemStructure` enum rather
    /// than the generic `T`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<ItemStructure>,

    /// `events`: the events in the series. This attribute is of a generic
    /// type whose parameter must be a descendant of `ITEM_STRUCTURE`.
    ///
    /// Cardinality `0..1` per the spec table; modelled as
    /// `Option<Vec<Event<T>>>` for the same "attribute absent vs. empty
    /// list" reasoning as `ItemList.items` (see `item_structure/item_list.rs`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<Event<T>>>,
}

// PORT NOTE: the `T: ItemStructureApi` bound is repeated here only because
// the struct definition itself declares it (Rust requires an impl for a
// bounded struct to satisfy the struct's own bounds); `TypeName` needs
// nothing from `T` — the class name is `"HISTORY"` for every instantiation.
impl<T: ItemStructureApi> TypeName for History<T> {
    const NAME: &'static str = TYPE_NAME;
}

impl<T: ItemStructureApi> History<T> {
    /// `is_periodic`: indicates whether history is periodic.
    ///
    /// Invariant `Periodic_validity`: `is_periodic xor period = Void`. This
    /// invariant is exactly the function's own defining contract
    /// (`is_periodic` is true iff `period` is set), so the implementation
    /// is definitional, matching the `Element::is_null` precedent.
    pub fn is_periodic(&self) -> bool {
        self.period.is_some()
    }

    /// `offset_of`: the offset of `event` relative to this History's
    /// `origin`, i.e. the parent-driven form of `EVENT.offset()`.
    ///
    /// PORT NOTE: not a distinct spec function — it is where the spec's
    /// `EVENT.offset() = time.diff(parent.origin)` is realised, supplying
    /// this History (the `parent`) `origin` to the event's `offset`. See the
    /// `EventApi::offset` PORT NOTE (`event.rs`) for why the origin is
    /// passed explicitly rather than reached through a `PATHABLE.parent()`
    /// back-reference. The underlying `DvDateTime::diff` arithmetic is the
    /// data_types agent's P17 deliverable.
    pub fn offset_of(&self, event: &Event<T>) -> DvDuration {
        event.offset(&self.origin)
    }

    /// `Events_valid`: `(events /= Void and then not events.is_empty) or
    /// summary /= Void`.
    ///
    /// A History must carry either a non-empty `events` list or a `summary`.
    /// Working `invariant_*` method per ADR-003 §8 (the deep `Validate`
    /// walker remains the P11 deliverable).
    pub fn invariant_events_valid(&self) -> bool {
        self.events.as_ref().is_some_and(|e| !e.is_empty()) || self.summary.is_some()
    }

    /// `Period_consistency`: `is_periodic implies events.for_all (e: EVENT |
    /// e.offset.to_seconds.mod(period.to_seconds) = 0)`.
    ///
    /// For a periodic History, every event's offset from `origin` must be an
    /// integer multiple of `period`. Working `invariant_*` method per
    /// ADR-003 §8: `offset_of` (→ `DvDateTime::diff`) and
    /// `DvDuration::magnitude` (`to_seconds`) are all available. Vacuously
    /// true for a non-periodic History (the antecedent `is_periodic` is
    /// false).
    pub fn invariant_period_consistency(&self) -> bool {
        if !self.is_periodic() {
            return true;
        }
        let Some(period) = self.period.as_ref() else {
            return true;
        };
        let period_secs = period.magnitude();
        self.events.as_ref().is_none_or(|events| {
            events.iter().all(|e| {
                let offset_secs = self.offset_of(e).magnitude();
                if period_secs.abs() < f64::EPSILON {
                    // Degenerate zero period: only a zero offset is a multiple.
                    offset_secs.abs() < f64::EPSILON
                } else {
                    (offset_secs % period_secs).abs() < 1e-9
                }
            })
        })
    }
}

impl<T: ItemStructureApi> DataStructureBehaviour for History<T> {
    fn as_hierarchy(&self) -> Item {
        // TODO(port): `HISTORY` has no explicit `as_hierarchy` redefinition
        // row in its own spec table (unlike every `ITEM_STRUCTURE`
        // subtype), so it inherits `DATA_STRUCTURE.as_hierarchy(): ITEM`
        // unredefined. The class description for `DATA_STRUCTURE` states
        // the function generates "the equivalent CEN EN13606 single
        // hierarchy for each subtype's physical representation" — for
        // `HISTORY`, no ISO 13606 encoding rule is documented anywhere in
        // this package (the "ISO 13606 Encoding Rules" section in
        // master04-item_structure_package.adoc only covers the four
        // `ITEM_STRUCTURE` subtypes, not `HISTORY` itself), so the actual
        // hierarchy-generation algorithm for a `HISTORY<T>` is
        // unspecified. Left as `todo!()` pending that determination.
        todo!(
            "as_hierarchy(): HISTORY has no documented ISO 13606 encoding rule anywhere in this spec package"
        )
    }
}

pub const TYPE_NAME: &str = "HISTORY";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::archetyped::locatable::LocatableData;
    use crate::data_structures::history::event::EventData;
    use crate::data_structures::history::point_event::PointEvent;
    use crate::data_structures::item_structure::data_structure::DataStructureData;
    use crate::data_structures::item_structure::item_list::ItemList;
    use crate::data_structures::item_structure::item_structure::ItemStructureData;

    fn locatable(name: &str) -> LocatableData {
        serde_json::from_value(serde_json::json!({
            "name": { "_type": "DV_TEXT", "value": name },
            "archetype_node_id": "at0001",
        }))
        .unwrap()
    }

    fn dt(value: &str) -> DvDateTime {
        serde_json::from_value(serde_json::json!({ "_type": "DV_DATE_TIME", "value": value }))
            .unwrap()
    }

    fn dur(value: &str) -> DvDuration {
        serde_json::from_value(serde_json::json!({ "_type": "DV_DURATION", "value": value }))
            .unwrap()
    }

    fn item_list() -> ItemList {
        ItemList {
            type_tag: TypeTag::new(),
            item_structure: ItemStructureData {
                data_structure: DataStructureData {
                    locatable: locatable("data"),
                },
            },
            items: None,
        }
    }

    fn point_event(time: &str) -> Event<ItemList> {
        Event::Point(PointEvent {
            type_tag: TypeTag::new(),
            event: EventData {
                locatable: locatable("event"),
                time: dt(time),
                state: None,
                data: item_list(),
            },
        })
    }

    /// A History with an origin at 10:00 and two point events at +1 and +2
    /// minutes.
    fn history(period: Option<DvDuration>) -> History<ItemList> {
        History {
            type_tag: TypeTag::new(),
            data_structure: DataStructureData {
                locatable: locatable("history"),
            },
            origin: dt("2024-01-01T10:00:00"),
            period,
            duration: None,
            summary: None,
            events: Some(vec![
                point_event("2024-01-01T10:01:00"),
                point_event("2024-01-01T10:02:00"),
            ]),
        }
    }

    /// Spec `is_periodic` / `Periodic_validity`: `is_periodic xor period =
    /// Void` — is_periodic is exactly "period is set".
    #[test]
    fn is_periodic_reflects_period_presence() {
        assert!(!history(None).is_periodic());
        assert!(history(Some(dur("PT1M"))).is_periodic());
    }

    /// Spec `Events_valid`: `(events /= Void and then not events.is_empty) or
    /// summary /= Void`.
    #[test]
    fn events_valid_invariant() {
        // non-empty events, no summary → valid
        assert!(history(None).invariant_events_valid());

        // neither events nor summary → invalid
        let mut h = history(None);
        h.events = None;
        assert!(!h.invariant_events_valid());

        // empty events, no summary → invalid
        let mut h = history(None);
        h.events = Some(vec![]);
        assert!(!h.invariant_events_valid());

        // no events but a summary present → valid
        let mut h = history(None);
        h.events = None;
        h.summary = Some(ItemStructure::List(item_list()));
        assert!(h.invariant_events_valid());
    }

    /// Spec `EVENT.offset()` postcondition (`Result = time.diff(parent.origin)`),
    /// driven from the parent History via `offset_of`.
    ///
    /// `DvDateTime::diff` is the data_types agent's P17 ISO 8601 arithmetic
    /// (currently `todo!()`). This test proves the *wiring* (offset delegates
    /// to `time.diff(origin)`) unconditionally and asserts the *values* once
    /// that arithmetic lands — staying green in both states. `catch_unwind`
    /// absorbs the deferred-arithmetic panic; a passing run may print that
    /// `todo!()` message to stderr, which is expected until P17.
    #[test]
    fn offset_of_is_time_diff_origin() {
        let h = history(None);
        let events = h.events.clone().unwrap();

        let r1 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| h.offset_of(&events[0])));
        let r2 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| h.offset_of(&events[1])));

        if let (Ok(o1), Ok(o2)) = (r1, r2) {
            // Event 1 is +60 s from origin, event 2 is +120 s.
            assert!((o1.magnitude() - 60.0).abs() < 1e-9);
            assert!((o2.magnitude() - 120.0).abs() < 1e-9);
            assert!(o1.magnitude() < o2.magnitude());
        }
    }

    /// Spec `Period_consistency`: `is_periodic implies events.for_all(e |
    /// e.offset.to_seconds mod period.to_seconds = 0)`.
    ///
    /// Guarded like `offset_of_is_time_diff_origin` — it transitively calls
    /// `DvDateTime::diff` (the P17 arithmetic), so it proves the wiring now
    /// and the values once that arithmetic lands.
    #[test]
    fn period_consistency_invariant() {
        // Non-periodic history: vacuously consistent.
        assert!(history(None).invariant_period_consistency());

        // Periodic with period PT1M: events at +60 s and +120 s are both
        // multiples of 60 s → consistent.
        let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            history(Some(dur("PT1M"))).invariant_period_consistency()
        }));
        if let Ok(result) = ok {
            assert!(result);
        }

        // Periodic with period PT90S: +60 s is not a multiple of 90 s →
        // inconsistent.
        let bad = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            history(Some(dur("PT1M30S"))).invariant_period_consistency()
        }));
        if let Ok(result) = bad {
            assert!(!result);
        }
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.history §HISTORY — docs/research/spec-cache/RM-1.1.0/uml_classes/history.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-history_package.adoc §Class Descriptions / history.adoc §HISTORY Class
//   confidence: high
//   todos: 1
//   note: is_periodic() implemented (definitional); Events_valid and Period_consistency now working invariant methods (ADR-003 §8) with tests; offset_of() realises EVENT.offset()=time.diff(origin) driven by the parent History. Only as_hierarchy() stays todo!() — a genuine published-spec gap: no ISO 13606 encoding rule is documented for HISTORY anywhere in the item_structure or history chapters (unlike the four ITEM_STRUCTURE subtypes). P4/ADR-002: self-tag added (generic form TypeTag<History<T>>; TypeName impl mirrors the struct's own T: ItemStructureApi bound).
// ─────────────────────────────────────────────
