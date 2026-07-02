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
use crate::data_structures::representation::item::Item;

/// `ITEM_TREE` class.
///
/// Embeds the shared `ITEM_STRUCTURE` state (per ADR-001 §3) plus its own
/// `items` attribute.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemTree {
    /// Inherited `ITEM_STRUCTURE` (and transitively `DATA_STRUCTURE`,
    /// `LOCATABLE`) state.
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
    pub items: Option<Vec<Item>>,
}

impl ItemStructureApi for ItemTree {
    fn item_structure_data(&self) -> &ItemStructureData {
        &self.item_structure
    }
}

impl ItemTree {
    /// `has_element_path`: `True` if path `a_path` is a valid leaf path.
    pub fn has_element_path(&self, a_path: &str) -> bool {
        // TODO(port): path resolution against archetype/runtime paths
        // depends on `PATHABLE`/`LOCATABLE` path-building machinery (the
        // `name`/`archetype_node_id`-derived path segments), owned by the
        // concurrently-transcribed `common` package — see
        // `representation/item.rs` for the same forward-reference
        // dependency.
        let _ = a_path;
        todo!(
            "has_element_path(a_path): needs PATHABLE/LOCATABLE path-building machinery from the common package"
        )
    }

    /// `element_at_path`: return the leaf element at the path `a_path`.
    pub fn element_at_path(&self, a_path: &str) -> Element {
        // TODO(port): same path-resolution dependency as
        // `has_element_path`. Spec signature declares this returning
        // `ELEMENT` (not `Option<ELEMENT>`); no-such-path behaviour is not
        // specified in the table.
        let _ = a_path;
        todo!(
            "element_at_path(a_path): needs PATHABLE/LOCATABLE path-building machinery from the common package; not-found behaviour also unspecified"
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
    /// its `List<ELEMENT>`. See that TODO's rationale in `item_list.rs` for
    /// why constructing this wrapper is blocked on the same `common`
    /// package `LOCATABLE` fields.
    pub fn as_hierarchy(&self) -> Cluster {
        // TODO(port): constructing the wrapper `Cluster` requires
        // `common::archetyped::locatable` fields (LOCATABLE state for the
        // synthesized node) — same dependency as `ItemList::as_hierarchy`.
        todo!(
            "as_hierarchy(): needs a LOCATABLE-state policy for the synthesized wrapper CLUSTER, same as ItemList"
        )
    }
}

impl DataStructureBehaviour for ItemTree {
    fn as_hierarchy(&self) -> Item {
        Item::Cluster(self.as_hierarchy())
    }
}

pub const TYPE_NAME: &str = "ITEM_TREE";

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.item_structure §ITEM_TREE — docs/research/spec-cache/RM-1.1.0/uml_classes/item_tree.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-item_structure_package.adoc §Class Descriptions / item_tree.adoc §ITEM_TREE Class
//   confidence: medium
//   todos: 3
//   note: has_element_path()/element_at_path() block on PATHABLE/LOCATABLE path-building from the common package; as_hierarchy()'s "synthesized wrapper CLUSTER" reading (analogous to ITEM_LIST) is a judgment call since the spec text does not literally spell out the wrapper mechanism for a tree whose items are already List<ITEM>.
// ─────────────────────────────────────────────
