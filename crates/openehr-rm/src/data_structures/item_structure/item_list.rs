//! `ITEM_LIST` — logical list data structure.
//!
//! openEHR class: `ITEM_LIST`, package `rm.data_structures.item_structure`.
//!
//! Logical list data structure, where each item has a value and can be
//! referred to by a name and a positional index in the list. The list may
//! be empty.
//!
//! `ITEM_LIST` is used to represent any data which is logically a list of
//! values, such as blood pressure, most protocols, many blood tests etc.
//!
//! Misuse: not to be used for time-based lists, which should be
//! represented with the proper temporal class, i.e. `HISTORY`.

use super::data_structure::DataStructureBehaviour;
use super::item_structure::{ItemStructureApi, ItemStructureData};
use crate::data_structures::representation::cluster::Cluster;
use crate::data_structures::representation::element::Element;
use crate::data_structures::representation::item::Item;
// PORT NOTE: `DV_TEXT` belongs to `rm.data_types.text`, transcribed
// concurrently by a sibling agent; see `representation/element.rs` for the
// identical forward-reference rationale and assumed module path.
use crate::data_types::text::dv_text::DvText;

/// `ITEM_LIST` class.
///
/// Embeds the shared `ITEM_STRUCTURE` state (per ADR-001 §3) plus its own
/// `items` attribute.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemList {
    /// Inherited `ITEM_STRUCTURE` (and transitively `DATA_STRUCTURE`,
    /// `LOCATABLE`) state.
    pub item_structure: ItemStructureData,

    /// `items`: physical representation of the list.
    ///
    /// Cardinality `0..1` per the spec table (the attribute itself may be
    /// absent, distinct from an empty list — see also the "list may be
    /// empty" note in the class description). Modelled as
    /// `Option<Vec<Element>>` rather than always defaulting an absent
    /// attribute to an empty `Vec`, to keep "attribute not set" and "list
    /// set but empty" distinguishable, matching the `0..1` cardinality
    /// literally.
    pub items: Option<Vec<Element>>,
}

impl ItemStructureApi for ItemList {
    fn item_structure_data(&self) -> &ItemStructureData {
        &self.item_structure
    }
}

impl ItemList {
    /// `item_count`: count of all items.
    pub fn item_count(&self) -> i32 {
        self.items.as_ref().map_or(0, |v| v.len() as i32)
    }

    /// `names`: retrieve the names of all items.
    ///
    /// Cardinality `0..1` on the function's own return (a `List<DV_TEXT>`
    /// that may itself be absent) — literally distinct from returning an
    /// empty list. Modelled as `Option<Vec<DvText>>` to preserve that
    /// distinction; the actual name-extraction from each `Element`'s
    /// inherited `LOCATABLE.name` is deferred to the `common` package
    /// landing (see forward-reference notes elsewhere in this file set).
    pub fn names(&self) -> Option<Vec<DvText>> {
        // TODO(port): requires `Element.item.locatable.name` accessor,
        // which depends on the not-yet-landed `common::archetyped::locatable`
        // module (see `representation/item.rs`).
        todo!("names(): needs LOCATABLE.name accessor on Element via common package")
    }

    /// `named_item`: retrieve the item with name `a_name`.
    pub fn named_item(&self, a_name: &str) -> Element {
        // TODO(port): requires `Element.item.locatable.name` accessor; see
        // `names()` above for the same forward-reference dependency. No
        // Void/error path is declared in the spec signature (returns
        // `ELEMENT`, not `ELEMENT` with an explicit failure case), so the
        // not-found behaviour is left unspecified pending that dependency.
        let _ = a_name;
        todo!("named_item(a_name): needs LOCATABLE.name accessor on Element via common package")
    }

    /// `ith_item`: retrieve the i-th item with name.
    pub fn ith_item(&self, i: i32) -> Element {
        // TODO(port): the spec signature declares this returning `ELEMENT`
        // (not `Option<ELEMENT>`), so an out-of-range `i` has no declared
        // Void/error path in the table; left as todo!() pending an
        // out-of-range policy decision (panic vs a wrapped Result, since
        // the RM proper avoids exceptions-as-control-flow).
        let _ = i;
        todo!("ith_item(i): out-of-range behaviour not specified by the spec table")
    }

    /// `as_hierarchy` (redefined): generate a CEN EN13606-compatible
    /// hierarchy consisting of a single `CLUSTER` containing the `ELEMENT`s
    /// of this list.
    ///
    /// Covariant redefinition (ADR-001 §6): narrows
    /// `DATA_STRUCTURE.as_hierarchy(): ITEM` to
    /// `ITEM_LIST.as_hierarchy(): CLUSTER`. See `data_structure.rs` for the
    /// shape rationale (widened trait method + narrowed inherent override).
    pub fn as_hierarchy(&self) -> Cluster {
        // TODO(port): constructing a `Cluster` requires the (not yet
        // landed) `common::archetyped::locatable` fields on `ItemData`
        // (`name`, `archetype_node_id`, ...) to be populated for the
        // synthesized CLUSTER wrapper — the spec does not specify what
        // name/archetype_node_id the generated wrapper CLUSTER should
        // carry, only that its `items` are "the ELEMENTs of this list".
        todo!("as_hierarchy(): needs a LOCATABLE-state policy for the synthesized wrapper CLUSTER")
    }
}

impl DataStructureBehaviour for ItemList {
    fn as_hierarchy(&self) -> Item {
        Item::Cluster(self.as_hierarchy())
    }
}

// TODO(port): invariant `Valid_structure`:
// `items.forall (i: ITEM | i.type = "ELEMENT")` — trivially true by
// construction here, since `items: Option<Vec<Element>>` is already typed
// to hold only `Element`s (Rust's static typing enforces this invariant
// structurally rather than requiring a runtime check). Recorded as a
// `Validate` no-op per `.claude/rules/rm-transcription.md` "Invariants"
// once the framework lands, rather than silently omitted.

pub const TYPE_NAME: &str = "ITEM_LIST";

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.item_structure §ITEM_LIST — docs/research/spec-cache/RM-1.1.0/uml_classes/item_list.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-item_structure_package.adoc §Class Descriptions / item_list.adoc §ITEM_LIST Class
//   confidence: medium
//   todos: 5
//   note: names()/named_item()/ith_item()/as_hierarchy() all block on the not-yet-landed common::archetyped::locatable module for LOCATABLE.name; Valid_structure invariant is structurally guaranteed by the Rust type system and needs only a Validate-framework no-op once that lands.
// ─────────────────────────────────────────────
