//! `ITEM_SINGLE` — logical single value data structure.
//!
//! openEHR class: `ITEM_SINGLE`, package `rm.data_structures.item_structure`.
//!
//! Used to represent any data which is logically a single value, such as a
//! person's height or weight.

use super::data_structure::DataStructureBehaviour;
use super::item_structure::{ItemStructureApi, ItemStructureData};
use crate::data_structures::representation::element::Element;
use crate::data_structures::representation::item::Item;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `ITEM_SINGLE` class.
///
/// Embeds the shared `ITEM_STRUCTURE` state (per ADR-001 §3) plus its own
/// `item` attribute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemSingle {
    /// Canonical `_type` discriminator (`"ITEM_SINGLE"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Inherited `ITEM_STRUCTURE` (and transitively `DATA_STRUCTURE`,
    /// `LOCATABLE`) state.
    #[serde(flatten)]
    pub item_structure: ItemStructureData,

    /// `item`: the single element carried by this structure.
    ///
    /// Cardinality `1..1`.
    pub item: Element,
}

impl TypeName for ItemSingle {
    const NAME: &'static str = TYPE_NAME;
}

impl ItemStructureApi for ItemSingle {
    fn item_structure_data(&self) -> &ItemStructureData {
        &self.item_structure
    }
}

impl ItemSingle {
    /// `as_hierarchy` (redefined): generate a CEN EN13606-compatible
    /// hierarchy consisting of a single `ELEMENT`.
    ///
    /// Covariant redefinition (ADR-001 §6): the spec table marks this
    /// function `1..1 (redefined)`, narrowing `DATA_STRUCTURE.as_hierarchy():
    /// ITEM` to `ITEM_SINGLE.as_hierarchy(): ELEMENT`. Declared here as an
    /// inherent method with the narrowed return type directly, per the
    /// shape documented on `DataStructureBehaviour` in `data_structure.rs`.
    /// `DataStructureBehaviour::as_hierarchy` (the widened-to-`Item` trait
    /// method) is implemented separately below and simply widens this
    /// method's result.
    pub fn as_hierarchy(&self) -> Element {
        self.item.clone()
    }
}

impl DataStructureBehaviour for ItemSingle {
    fn as_hierarchy(&self) -> Item {
        self.item.clone().into()
    }
}

pub const TYPE_NAME: &str = "ITEM_SINGLE";

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.item_structure §ITEM_SINGLE — docs/research/spec-cache/RM-1.1.0/uml_classes/item_single.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-item_structure_package.adoc §Class Descriptions / item_single.adoc §ITEM_SINGLE Class
//   confidence: high
//   todos: 0
//   note: as_hierarchy() is a straight clone of the single element per the spec description ("consisting of a single ELEMENT"); revisit clone cost at P17/PERF pass if Element grows expensive to copy. P4: added #[serde(flatten)] on `item_structure` (was missing) so ITEM_STRUCTURE's fields sit flat on ITEM_SINGLE per the ITS-JSON schema; ADR-002 self-tag (TypeName + first-field TypeTag) added.
// ─────────────────────────────────────────────
