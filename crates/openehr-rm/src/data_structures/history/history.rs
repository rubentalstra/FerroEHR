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

use super::event::Event;
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

    // TODO(port): invariant `Events_valid`:
    // `(events /= Void and then not events.is_empty) or summary /= Void` —
    // requires either a non-empty `events` or a present `summary`;
    // recorded pending the RM `Validate` trait framework
    // (`.claude/rules/rm-transcription.md` "Invariants").

    // TODO(port): invariant `Period_consistency`:
    // `is_periodic implies events.for_all (e: EVENT | e.offset.to_seconds.mod(
    // period.to_seconds) = 0)` — requires both `Event::offset()` (itself
    // blocked on the `PATHABLE.parent()` back-reference, see `event.rs`)
    // and `DvDuration::to_seconds()` (a `data_types` dependency), so this
    // is transitively blocked on two not-yet-landed pieces.
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

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.history §HISTORY — docs/research/spec-cache/RM-1.1.0/uml_classes/history.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-history_package.adoc §Class Descriptions / history.adoc §HISTORY Class
//   confidence: medium
//   todos: 3
//   note: is_periodic() is implemented (definitional); Events_valid and Period_consistency invariants deferred to the Validate framework, the latter transitively blocked on Event::offset() and DvDuration::to_seconds(); as_hierarchy() is genuinely underspecified for HISTORY (no ISO 13606 encoding rule documented for it anywhere in the item_structure or history package chapters). P4/ADR-002: self-tag added (generic form TypeTag<History<T>>; TypeName impl mirrors the struct's own T: ItemStructureApi bound).
// ─────────────────────────────────────────────
