//! `ITEM_STRUCTURE` — abstract parent of all spatial data types.
//!
//! openEHR class: `ITEM_STRUCTURE` (abstract), package
//! `rm.data_structures.item_structure`.
//!
//! Abstract parent class of all spatial data types. Declares no attributes
//! or functions of its own beyond what it inherits from `DATA_STRUCTURE`.

use super::data_structure::{DataStructureBehaviour, DataStructureData};
use super::item_list::ItemList;
use super::item_single::ItemSingle;
use super::item_table::ItemTable;
use super::item_tree::ItemTree;
use crate::data_structures::representation::item::Item;
use serde::{Deserialize, Serialize};

/// Shared attribute state of `ITEM_STRUCTURE` and its descendants.
///
/// Per ADR-001 §3 and the Refinements note (abstract class with attributes
/// that is also used polymorphically as a declared field type combines §3
/// and §4). `ITEM_STRUCTURE` itself adds no attribute beyond the inherited
/// `DATA_STRUCTURE` state, so `ItemStructureData` is presently just that
/// embedding — named so `ItemSingle`/`ItemList`/`ItemTable`/`ItemTree` each
/// hold one common field rather than re-embedding `DataStructureData`
/// (and transitively `LocatableData`) directly, and so a future
/// `ITEM_STRUCTURE`-level attribute has one place to land.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemStructureData {
    /// Inherited `DATA_STRUCTURE` (and transitively `LOCATABLE`) state.
    #[serde(flatten)]
    pub data_structure: DataStructureData,
}

/// `ITEM_STRUCTURE` is abstract in the spec and is used polymorphically
/// wherever an attribute or return type is declared `ITEM_STRUCTURE` (e.g.
/// `HISTORY<T: ITEM_STRUCTURE>`'s type parameter bound,
/// `EVENT.state: ITEM_STRUCTURE`, `HISTORY.summary: ITEM_STRUCTURE`). Per
/// ADR-001 §4 (closed subtype set → enum), the four concrete subtypes
/// `ITEM_SINGLE`, `ITEM_LIST`, `ITEM_TABLE`, and `ITEM_TREE` are collected
/// into this closed `enum` so a field, return type, or generic bound target
/// can be declared `ItemStructure` exactly where the spec declares it
/// `ITEM_STRUCTURE`.
// PORT NOTE: `#[serde(untagged)]` per ADR-002 — dispatch is driven by each
// variant payload's own `TypeTag` (`ItemSingle`/`ItemList`/`ItemTable`/
// `ItemTree` self-tag with `_type`), whose `Deserialize` fails on a
// mismatched `_type` string, so untagged probing is tag-driven rather than
// structure-driven. A struct-level `#[serde(tag = "_type")]` here would
// duplicate the payloads' own `_type` keys on the wire. `Single` (the only
// variant with a required own attribute, `item`) stays first; `List`/
// `Table`/`Tree` carry only optional own attributes, so tag-less input is
// structurally ambiguous among them — harmless, since ITS-JSON requires
// `_type` in abstract-declared slots such as this enum's use sites
// (`HISTORY.summary`, `EVENT.state`), and all tagged input dispatches by
// tag regardless of order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ItemStructure {
    /// `ITEM_SINGLE`.
    Single(ItemSingle),
    /// `ITEM_LIST`.
    List(ItemList),
    /// `ITEM_TABLE`.
    Table(ItemTable),
    /// `ITEM_TREE`.
    Tree(ItemTree),
}

/// Marker/accessor trait shared by every `ITEM_STRUCTURE` descendant,
/// exposing the abstract class's inherited state uniformly whether the
/// caller holds a concrete type or an `ItemStructure` enum value, plus the
/// `DATA_STRUCTURE`-level `as_hierarchy()` behaviour widened to `Item` (see
/// `DataStructureBehaviour`'s doc comment on the covariant-redefinition
/// shape used throughout this package).
pub trait ItemStructureApi: DataStructureBehaviour {
    /// Access the shared `ITEM_STRUCTURE` (i.e. inherited `DATA_STRUCTURE`)
    /// state.
    fn item_structure_data(&self) -> &ItemStructureData;
}

impl DataStructureBehaviour for ItemStructure {
    fn as_hierarchy(&self) -> Item {
        match self {
            ItemStructure::Single(v) => v.as_hierarchy().into(),
            ItemStructure::List(v) => v.as_hierarchy().into(),
            ItemStructure::Table(v) => v.as_hierarchy().into(),
            ItemStructure::Tree(v) => v.as_hierarchy().into(),
        }
    }
}

impl ItemStructureApi for ItemStructure {
    fn item_structure_data(&self) -> &ItemStructureData {
        match self {
            ItemStructure::Single(v) => v.item_structure_data(),
            ItemStructure::List(v) => v.item_structure_data(),
            ItemStructure::Table(v) => v.item_structure_data(),
            ItemStructure::Tree(v) => v.item_structure_data(),
        }
    }
}

pub const TYPE_NAME: &str = "ITEM_STRUCTURE";

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.item_structure §ITEM_STRUCTURE — docs/research/spec-cache/RM-1.1.0/uml_classes/item_structure.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-item_structure_package.adoc §Class Descriptions / item_structure.adoc §ITEM_STRUCTURE Class
//   confidence: medium
//   todos: 0
//   note: `impl DataStructureBehaviour for ItemStructure::as_hierarchy` dispatches to each concrete type's narrowed inherent as_hierarchy() and widens the result into Item via a `From` conversion each concrete type is expected to provide (Element -> Item, Cluster -> Item) — those From impls are declared where Item is transcribed (representation/item.rs) as ordinary enum-variant constructors, not written here. P4/ADR-002: ItemStructure enum is #[serde(untagged)] — dispatch via the payload TypeTags on the four concretes; ItemStructureData stays untagged (abstract).
// ─────────────────────────────────────────────
