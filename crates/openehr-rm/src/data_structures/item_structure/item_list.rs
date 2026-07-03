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
use crate::data_structures::representation::item::{Item, ItemData};
// PORT NOTE: `DV_TEXT` belongs to `rm.data_types.text` (now landed); its
// `DvTextApi::value` accessor is used to compare item names by string.
use crate::data_types::text::dv_text::{DvText, DvTextApi};
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `ITEM_LIST` class.
///
/// Embeds the shared `ITEM_STRUCTURE` state (per ADR-001 §3) plus its own
/// `items` attribute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemList {
    /// Canonical `_type` discriminator (`"ITEM_LIST"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Inherited `ITEM_STRUCTURE` (and transitively `DATA_STRUCTURE`,
    /// `LOCATABLE`) state.
    #[serde(flatten)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<Element>>,
}

impl TypeName for ItemList {
    const NAME: &'static str = TYPE_NAME;
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
    /// distinction: `None` when the `items` attribute itself is absent,
    /// `Some(..)` (possibly empty) otherwise, collecting each `ELEMENT`'s
    /// inherited `LOCATABLE.name`.
    pub fn names(&self) -> Option<Vec<DvText>> {
        self.items.as_ref().map(|items| {
            items
                .iter()
                .map(|el| el.item.locatable.name.clone())
                .collect()
        })
    }

    /// `named_item`: retrieve the item with name `a_name`.
    ///
    /// PORT NOTE: the spec signature returns `ELEMENT` (`1..1`) with no
    /// declared not-found path. Widened to `Option<Element>` per the
    /// precondition-widening precedent used across this port (e.g.
    /// `common::archetyped::locatable::LocatableApi::concept`): a name with
    /// no matching item yields `None` rather than a panic. Match is by the
    /// element's `LOCATABLE.name` value string.
    pub fn named_item(&self, a_name: &str) -> Option<Element> {
        self.items.as_ref().and_then(|items| {
            items
                .iter()
                .find(|el| el.item.locatable.name.value() == a_name)
                .cloned()
        })
    }

    /// `ith_item`: retrieve the i-th item.
    ///
    /// PORT NOTE: `i` is 1-based, matching openEHR's Eiffel-derived indexing
    /// convention used throughout the foundation `List<T>`/RM function set
    /// (`i = 1` is the first item). The spec signature returns `ELEMENT`
    /// (`1..1`) with no out-of-range path; widened to `Option<Element>` per
    /// the same precondition-widening precedent as `named_item`.
    pub fn ith_item(&self, i: i32) -> Option<Element> {
        let idx = usize::try_from(i - 1).ok()?;
        self.items
            .as_ref()
            .and_then(|items| items.get(idx).cloned())
    }

    /// `as_hierarchy` (redefined): generate a CEN EN13606-compatible
    /// hierarchy consisting of a single `CLUSTER` containing the `ELEMENT`s
    /// of this list.
    ///
    /// Covariant redefinition (ADR-001 §6): narrows
    /// `DATA_STRUCTURE.as_hierarchy(): ITEM` to
    /// `ITEM_LIST.as_hierarchy(): CLUSTER`. See `data_structure.rs` for the
    /// shape rationale (widened trait method + narrowed inherent override).
    ///
    /// PORT NOTE: the spec states the generated wrapper is "a single CLUSTER
    /// containing the ELEMENTs of this list" but does not specify the
    /// wrapper's own `LOCATABLE` state. Since `ITEM_LIST` is itself a
    /// `LOCATABLE`, the wrapper CLUSTER inherits this list's own
    /// `LOCATABLE` identity (`name`, `archetype_node_id`, `uid`, ...) — the
    /// coherent reading, matching the reference implementation's
    /// `ItemList.asHierarchy()`. Its `items` are this list's elements,
    /// widened `Element -> Item::Element`.
    pub fn as_hierarchy(&self) -> Cluster {
        Cluster {
            type_tag: TypeTag::new(),
            item: ItemData {
                locatable: self.item_structure.data_structure.locatable.clone(),
            },
            items: self.items.as_ref().map_or_else(Vec::new, |els| {
                els.iter().cloned().map(Item::Element).collect()
            }),
        }
    }

    /// `Valid_structure`: `items.forall (i: ITEM | i.type = "ELEMENT")`.
    ///
    /// PORT NOTE: statically guaranteed here — `items: Option<Vec<Element>>`
    /// can only hold `ELEMENT`s, so the invariant is enforced by the Rust
    /// type system rather than at runtime. Exposed as a working
    /// `invariant_*` method (ADR-003 §8) that always holds, so the P11
    /// `Validate` framework can call it uniformly across `ITEM_STRUCTURE`s.
    /// Contrast `ITEM_TABLE`, whose rows are `Vec<Item>` and therefore need
    /// a real runtime check (see `item_table.rs`).
    pub fn invariant_valid_structure(&self) -> bool {
        self.items
            .as_ref()
            .is_none_or(|items| items.iter().all(|el| el.type_tag.name() == Element::NAME))
    }
}

impl DataStructureBehaviour for ItemList {
    fn as_hierarchy(&self) -> Item {
        Item::Cluster(self.as_hierarchy())
    }
}

pub const TYPE_NAME: &str = "ITEM_LIST";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::archetyped::locatable::LocatableData;
    use crate::data_structures::item_structure::data_structure::DataStructureData;
    use crate::data_structures::representation::item::ItemApi;

    fn locatable(name: &str, node_id: &str) -> LocatableData {
        serde_json::from_value(serde_json::json!({
            "name": { "_type": "DV_TEXT", "value": name },
            "archetype_node_id": node_id,
        }))
        .unwrap()
    }

    fn element(name: &str, node_id: &str) -> Element {
        Element {
            type_tag: TypeTag::new(),
            item: ItemData {
                locatable: locatable(name, node_id),
            },
            null_flavour: None,
            value: None,
            null_reason: None,
        }
    }

    /// A three-item list: systolic, diastolic, mean.
    fn bp_list() -> ItemList {
        ItemList {
            type_tag: TypeTag::new(),
            item_structure: ItemStructureData {
                data_structure: DataStructureData {
                    locatable: locatable("blood pressure", "at0001"),
                },
            },
            items: Some(vec![
                element("systolic", "at0004"),
                element("diastolic", "at0005"),
                element("mean arterial pressure", "at0006"),
            ]),
        }
    }

    /// Spec `item_count`: "Count of all items."
    #[test]
    fn item_count_counts_items() {
        assert_eq!(bp_list().item_count(), 3);
        let empty = ItemList {
            type_tag: TypeTag::new(),
            item_structure: ItemStructureData {
                data_structure: DataStructureData {
                    locatable: locatable("empty", "at0001"),
                },
            },
            items: None,
        };
        assert_eq!(empty.item_count(), 0);
    }

    /// Spec `names`: "Retrieve the names of all items."
    #[test]
    fn names_returns_each_item_name() {
        let names = bp_list().names().expect("items present");
        let values: Vec<&str> = names.iter().map(DvTextApi::value).collect();
        assert_eq!(values, ["systolic", "diastolic", "mean arterial pressure"]);
    }

    /// Spec `named_item`: "Retrieve the item with name 'a_name'." (widened
    /// to Option for the undeclared not-found path).
    #[test]
    fn named_item_finds_by_name() {
        let list = bp_list();
        let found = list.named_item("diastolic").expect("present");
        assert_eq!(found.item.locatable.name.value(), "diastolic");
        assert!(list.named_item("temperature").is_none());
    }

    /// Spec `ith_item`: "Retrieve the i-th item." (1-based, widened to
    /// Option for out-of-range).
    #[test]
    fn ith_item_is_one_based() {
        let list = bp_list();
        assert_eq!(
            list.ith_item(1).unwrap().item.locatable.name.value(),
            "systolic"
        );
        assert_eq!(
            list.ith_item(3).unwrap().item.locatable.name.value(),
            "mean arterial pressure"
        );
        assert!(list.ith_item(0).is_none());
        assert!(list.ith_item(4).is_none());
    }

    /// Spec `as_hierarchy` (redefined): "a single CLUSTER containing the
    /// ELEMENTs of this list." The wrapper carries the list's own LOCATABLE
    /// identity (PORT NOTE on the method).
    #[test]
    fn as_hierarchy_wraps_elements_in_a_cluster() {
        let list = bp_list();
        let cluster = list.as_hierarchy();
        // Wrapper CLUSTER inherits the list's own name.
        assert_eq!(cluster.name().value(), "blood pressure");
        // ...and contains exactly the list's three ELEMENTs.
        assert_eq!(cluster.items.len(), 3);
        assert!(
            cluster
                .items
                .iter()
                .all(|it| matches!(it, Item::Element(_)))
        );
    }

    /// Spec `Valid_structure`: statically guaranteed; the working method
    /// always holds for a well-typed `ItemList`.
    #[test]
    fn valid_structure_holds() {
        assert!(bp_list().invariant_valid_structure());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.item_structure §ITEM_LIST — docs/research/spec-cache/RM-1.1.0/uml_classes/item_list.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-item_structure_package.adoc §Class Descriptions / item_list.adoc §ITEM_LIST Class
//   confidence: high
//   todos: 0
//   note: common package landed — item_count/names/named_item/ith_item/as_hierarchy all implemented with spec-derived tests. named_item/ith_item widened to Option (undeclared not-found/out-of-range paths); ith_item is 1-based (Eiffel convention); as_hierarchy wraps the ELEMENTs in a CLUSTER carrying the list's own LOCATABLE identity (PORT NOTE). Valid_structure is a working invariant method, statically guaranteed by items: Option<Vec<Element>>. P4/ADR-002: self-tag (TypeName + first-field TypeTag) added.
// ─────────────────────────────────────────────
