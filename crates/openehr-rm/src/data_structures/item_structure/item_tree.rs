//! `ITEM_TREE` — logical tree data structure.
//!
//! openEHR class: `ITEM_TREE`, package `rm.data_structures.item_structure`.
//!
//! Logical tree data structure. The tree may be empty. Used for
//! representing data which are logically a tree such as audiology results,
//! microbiology results, biochemistry results.

use super::data_structure::DataStructureBehaviour;
use super::item_structure::{ItemStructureApi, ItemStructureData};
use crate::data_structures::representation::cluster::Cluster;
use crate::data_structures::representation::element::Element;
use crate::data_structures::representation::item::{Item, ItemData};
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `ITEM_TREE` class.
///
/// Embeds the shared `ITEM_STRUCTURE` state (per ADR-001 §3) plus its own
/// `items` attribute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemTree {
    /// Canonical `_type` discriminator (`"ITEM_TREE"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Inherited `ITEM_STRUCTURE` (and transitively `DATA_STRUCTURE`,
    /// `LOCATABLE`) state.
    #[serde(flatten)]
    pub item_structure: ItemStructureData,

    /// `items`: the items comprising the `ITEM_TREE`. Can include 0 or
    /// more `CLUSTER`s and/or 0 or more individual `ELEMENT`s.
    ///
    /// Cardinality `0..1` per the spec table; modelled as
    /// `Option<Vec<Item>>` for the same "attribute absent vs. empty list"
    /// reasoning as `ItemList.items` (see `item_list.rs`).
    ///
    /// Recursion note: `Item` already carries the recursive-containment
    /// edge (`Item::Cluster(Cluster)`, whose own `items: Vec<Item>` can
    /// nest further `Cluster`s) — see the doc comment on `Item`
    /// (`representation/item.rs`) for why no additional `Box` indirection
    /// is required here beyond what `Vec` already provides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<Item>>,
}

impl TypeName for ItemTree {
    const NAME: &'static str = TYPE_NAME;
}

impl ItemStructureApi for ItemTree {
    fn item_structure_data(&self) -> &ItemStructureData {
        &self.item_structure
    }
}

impl ItemTree {
    /// `has_element_path`: `True` if path `a_path` is a valid leaf path.
    pub fn has_element_path(&self, a_path: &str) -> bool {
        // TODO(port): the `common::archetyped::paths` evaluator (`RmPath::parse`,
        // `resolve`) has landed, but resolving a path against this tree
        // requires `ItemTree`/`Cluster`/`Element` to implement `PathableApi`'s
        // traversal hooks (`path_child_nodes`/`path_node_id`/`path_node_name`,
        // exposing the representation tree to the evaluator). Wiring the whole
        // representation subtree into the path evaluator is the P11 validation
        // + path deliverable, not part of the data-structure function set;
        // deferred to P11.
        let _ = a_path;
        todo!(
            "has_element_path(a_path): needs Cluster/Element/ItemTree wired into the common path evaluator (PathableApi traversal hooks) — P11"
        )
    }

    /// `element_at_path`: return the leaf element at the path `a_path`.
    pub fn element_at_path(&self, a_path: &str) -> Element {
        // TODO(port): same P11 path-evaluator wiring dependency as
        // `has_element_path`. Spec signature declares this returning
        // `ELEMENT` (not `Option<ELEMENT>`); no-such-path behaviour is not
        // specified in the table, to be widened to Option when wired at P11.
        let _ = a_path;
        todo!(
            "element_at_path(a_path): needs Cluster/Element/ItemTree wired into the common path evaluator (PathableApi traversal hooks) — P11; not-found behaviour also unspecified"
        )
    }

    /// `as_hierarchy` (redefined): generate a CEN EN13606-compatible
    /// hierarchy, which is the same as the tree's physical representation.
    ///
    /// Covariant redefinition (ADR-001 §6): narrows
    /// `DATA_STRUCTURE.as_hierarchy(): ITEM` to
    /// `ITEM_TREE.as_hierarchy(): CLUSTER`. See `data_structure.rs` for the
    /// shape rationale.
    ///
    /// PORT NOTE: `ITEM_TREE`'s own `items: List<ITEM>` attribute is
    /// already a list of `ITEM`s (mixed `CLUSTER`/`ELEMENT`), not itself a
    /// single `CLUSTER` — but the spec declares the *function's* return
    /// type as `CLUSTER` (matching every other `ITEM_STRUCTURE` subtype's
    /// redefinition) and describes it as "the same as the tree's physical
    /// representation". Read together, this means a synthesized wrapper
    /// `CLUSTER` whose own `items` are this tree's `items` — analogous to
    /// how `ItemList.as_hierarchy()` synthesizes a wrapper `CLUSTER` around
    /// its `List<ELEMENT>`. The wrapper carries this tree's own `LOCATABLE`
    /// identity (name, archetype_node_id, ...); its `items` are this tree's
    /// `items` verbatim (already a mixed `List<ITEM>`).
    pub fn as_hierarchy(&self) -> Cluster {
        Cluster {
            type_tag: TypeTag::new(),
            item: ItemData {
                locatable: self.item_structure.data_structure.locatable.clone(),
            },
            items: self.items.clone().unwrap_or_default(),
        }
    }
}

impl DataStructureBehaviour for ItemTree {
    fn as_hierarchy(&self) -> Item {
        Item::Cluster(self.as_hierarchy())
    }
}

pub const TYPE_NAME: &str = "ITEM_TREE";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::archetyped::locatable::LocatableData;
    use crate::data_structures::item_structure::data_structure::DataStructureData;
    use crate::data_structures::representation::cluster::Cluster;
    use crate::data_structures::representation::element::Element;
    use crate::data_structures::representation::item::ItemData;

    /// Build a minimal `LocatableData` from the wire shape rather than a
    /// struct literal, so this test stays decoupled from the exact Rust
    /// constructor shape of `DvText` (converted by a sibling agent in the
    /// same P4 wave).
    fn locatable(name: &str, node_id: &str) -> LocatableData {
        serde_json::from_value(serde_json::json!({
            "name": { "_type": "DV_TEXT", "value": name },
            "archetype_node_id": node_id,
        }))
        .unwrap()
    }

    /// ADR-002 acceptance for this package: an `ITEM_TREE` self-tags first
    /// in key order, nested `Item`-typed slots carry each payload's own
    /// `_type`, the whole tree round-trips, and untagged `Item` dispatch is
    /// driven by `_type` (not declaration order / structure).
    #[test]
    fn item_tree_self_tags_and_item_slot_roundtrips_via_type() {
        let tree = ItemTree {
            type_tag: TypeTag::new(),
            item_structure: ItemStructureData {
                data_structure: DataStructureData {
                    locatable: locatable("tree", "at0001"),
                },
            },
            items: Some(vec![Item::Cluster(Cluster {
                type_tag: TypeTag::new(),
                item: ItemData {
                    locatable: locatable("cluster", "at0002"),
                },
                items: vec![Item::Element(Element {
                    type_tag: TypeTag::new(),
                    item: ItemData {
                        locatable: locatable("element", "at0003"),
                    },
                    null_flavour: None,
                    value: None,
                    null_reason: None,
                })],
            })]),
        };

        let json = serde_json::to_string(&tree).unwrap();
        assert!(
            json.starts_with(r#"{"_type":"ITEM_TREE","#),
            "ITEM_TREE tag must lead the object: {json}"
        );
        assert!(json.contains(r#""_type":"CLUSTER""#));
        assert!(json.contains(r#""_type":"ELEMENT""#));

        // Round-trip: the Item-typed slot re-dispatches through the
        // untagged enum via each payload's TypeTag.
        let back: ItemTree = serde_json::from_str(&json).unwrap();
        assert_eq!(back, tree);
        assert!(matches!(back.items.as_ref().unwrap()[0], Item::Cluster(_)));

        // Tag-driven (not structure-driven) dispatch: this payload
        // structurally satisfies Cluster (it has `items`), but the tag says
        // ELEMENT — the Cluster arm's TypeTag must reject it.
        let element_json = r#"{"_type":"ELEMENT","name":{"_type":"DV_TEXT","value":"n"},"archetype_node_id":"at0004","items":[]}"#;
        let item: Item = serde_json::from_str(element_json).unwrap();
        assert!(matches!(item, Item::Element(_)));
    }

    /// Spec `as_hierarchy` (redefined): "the same as the tree's physical
    /// representation." The wrapper CLUSTER carries the tree's own identity
    /// and holds the tree's items verbatim (mixed CLUSTER/ELEMENT).
    #[test]
    fn as_hierarchy_wraps_items_verbatim() {
        use crate::data_structures::representation::item::ItemApi;
        use crate::data_types::text::dv_text::DvTextApi;

        let tree = ItemTree {
            type_tag: TypeTag::new(),
            item_structure: ItemStructureData {
                data_structure: DataStructureData {
                    locatable: locatable("biochemistry", "at0001"),
                },
            },
            items: Some(vec![
                Item::Cluster(Cluster {
                    type_tag: TypeTag::new(),
                    item: ItemData {
                        locatable: locatable("panel", "at0002"),
                    },
                    items: vec![],
                }),
                Item::Element(Element {
                    type_tag: TypeTag::new(),
                    item: ItemData {
                        locatable: locatable("comment", "at0003"),
                    },
                    null_flavour: None,
                    value: None,
                    null_reason: None,
                }),
            ]),
        };

        let cluster = tree.as_hierarchy();
        assert_eq!(cluster.name().value(), "biochemistry");
        assert_eq!(cluster.items.len(), 2);
        assert!(matches!(cluster.items[0], Item::Cluster(_)));
        assert!(matches!(cluster.items[1], Item::Element(_)));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.item_structure §ITEM_TREE — docs/research/spec-cache/RM-1.1.0/uml_classes/item_tree.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-item_structure_package.adoc §Class Descriptions / item_tree.adoc §ITEM_TREE Class
//   confidence: high
//   todos: 2
//   note: as_hierarchy() implemented (common landed) — wrapper CLUSTER carries the tree's own LOCATABLE identity and holds its items verbatim, with a spec-derived test. has_element_path()/element_at_path() remain todo!() — the common path evaluator (RmPath/resolve) exists, but wiring Cluster/Element/ItemTree into PathableApi's traversal hooks is the P11 validation+path deliverable. P4/ADR-002: self-tag (TypeName + first-field TypeTag) added, plus the package's ADR-002 acceptance unit test (tag-first serialization, Item round-trip, tag-driven untagged dispatch).
// ─────────────────────────────────────────────
