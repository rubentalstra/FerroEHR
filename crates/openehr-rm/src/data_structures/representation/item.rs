//! `ITEM` — abstract parent of the `representation` package's grouping and
//! leaf classes.
//!
//! openEHR class: `ITEM` (abstract), package `rm.data_structures.representation`.
//!
//! The abstract parent of `CLUSTER` and `ELEMENT` representation classes.
//! `ITEM` declares no attributes or functions of its own beyond what it
//! inherits from `LOCATABLE`.

use super::cluster::Cluster;
use super::element::Element;
// PORT NOTE: `LOCATABLE` lives in the `common` package cluster
// (`common::archetyped::locatable`), now landed. `ItemData` embeds its
// `LocatableData` and `ItemApi` exposes the two `LOCATABLE` accessors the
// `item_structure` package's own functions need (`name`,
// `archetype_node_id`) by delegating to that embedded state. The wider
// `LOCATABLE`/`PATHABLE` behaviour battery (`concept()`,
// `is_archetype_root()`, path resolution) is exposed by
// `common::archetyped::locatable::LocatableApi` and reached by wiring
// `Cluster`/`Element` into `PathableApi`, which is a P11 (validation +
// path) deliverable, not required by the data-structure functions here.
use crate::common::archetyped::locatable::LocatableData;
use crate::data_types::text::dv_text::DvText;
use serde::{Deserialize, Serialize};

/// Shared attribute state of `ITEM` and its descendants.
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct + marker
/// trait), every concrete `ITEM` subtype (`CLUSTER`, `ELEMENT`) embeds this
/// struct rather than inheriting from it, since Rust has no class
/// inheritance. `ITEM` itself declares no attributes beyond the `LOCATABLE`
/// state it inherits, so this struct is presently just the `LOCATABLE`
/// embedding — it exists as a named type (rather than inlining
/// `LocatableData` directly into `Cluster`/`Element`) so a later
/// `#[serde(flatten)]` at P4/P5 has a natural, ITEM-shaped target, and so
/// any future `ITEM`-level attribute the spec adds has somewhere to land
/// without re-touching every concrete descendant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemData {
    /// Inherited `LOCATABLE` state (`name`, `archetype_node_id`, `uid`,
    /// `links`, `archetype_details`, `feeder_audit`).
    ///
    /// PORT NOTE: reconciled with `common::archetyped::locatable::LocatableData`
    /// (now landed) — no longer a forward reference.
    #[serde(flatten)]
    pub locatable: LocatableData,
}

/// `ITEM` is abstract in the spec and is used polymorphically wherever an
/// attribute or return type is declared `ITEM` (e.g.
/// `CLUSTER.items: List<ITEM>`, `ITEM_TREE.items: List<ITEM>`,
/// `DATA_STRUCTURE.as_hierarchy(): ITEM`). Per ADR-001 §4 (closed subtype
/// set → enum), the two concrete subtypes `CLUSTER` and `ELEMENT` are
/// collected into this closed `enum` so a field or return type can be
/// declared `Item` exactly where the spec declares it `ITEM`.
///
/// Recursion note: `CLUSTER.items: List<ITEM>` is the recursive-containment
/// edge in this package (a `CLUSTER` can contain further `CLUSTER`s). The
/// `Item` enum wraps its `Cluster` variant *by value*, not `Box<Cluster>` —
/// this still satisfies the "recursive containment is boxed" rule
/// (`.claude/rules/rm-transcription.md`) because `Cluster.items` is a
/// `Vec<Item>`, and `Vec<T>`'s backing storage is heap-allocated
/// independently of `T`'s size. The cycle `Item -> Cluster -> Vec<Item>` has
/// no unboxed, unindirected self-reference (unlike, say, a hypothetical
/// `Cluster { parent: Cluster }` field), so no `Item::Cluster(Box<Cluster>)`
/// indirection is needed for `Item`/`Cluster` to have a finite size. This is
/// analogous to how `Vec<Item>` itself, not `Item`, is what breaks the
/// otherwise-infinite recursion — see the `CLUSTER` transcription
/// (`cluster.rs`) for the corresponding note on the `items` field.
// PORT NOTE: `#[serde(untagged)]` per ADR-002 — dispatch is driven by each
// variant payload's own `TypeTag` (`Cluster`/`Element` self-tag with
// `_type`), whose `Deserialize` fails on a mismatched `_type` string, so
// untagged probing is tag-driven rather than structure-driven. A struct-
// level `#[serde(tag = "_type")]` here would duplicate the payloads' own
// `_type` keys on the wire. Variant order still lists the structurally
// richer payload first (`Cluster` requires `items`; `Element` requires
// nothing an unknown-field-tolerant probe couldn't satisfy) so tag-less
// input resolves correctly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Item {
    /// `CLUSTER`.
    Cluster(Cluster),
    /// `ELEMENT`.
    Element(Element),
}

/// Marker/accessor trait shared by every `ITEM` descendant, exposing the
/// abstract class's inherited `LOCATABLE` state uniformly whether the
/// caller holds a concrete type or an `Item` enum value.
///
/// The two `LOCATABLE` accessors the `item_structure` package's own
/// functions rely on (`ITEM_LIST.names`/`named_item`,
/// `ITEM_TABLE.row_names`/`column_names`/`has_row_with_name` etc.) are
/// exposed here as default methods delegating to the embedded
/// `LocatableData`. `LOCATABLE`'s wider function battery (`concept()`,
/// `is_archetype_root()`, path resolution) lives on
/// `common::archetyped::locatable::LocatableApi`; exposing it via `ItemApi`
/// would require `Cluster`/`Element` to implement `PathableApi`, which is a
/// P11 (validation + path) deliverable and is not needed by the data-
/// structure functions in this package.
pub trait ItemApi {
    /// Access the shared `ITEM` (i.e. inherited `LOCATABLE`) state.
    fn item_data(&self) -> &ItemData;

    /// `name`: inherited `LOCATABLE.name` (`DV_TEXT`, `1..1`). Delegates to
    /// the embedded [`LocatableData`].
    fn name(&self) -> &DvText {
        &self.item_data().locatable.name
    }

    /// `archetype_node_id`: inherited `LOCATABLE.archetype_node_id`
    /// (`String`, `1..1`). Delegates to the embedded [`LocatableData`].
    fn archetype_node_id(&self) -> &str {
        &self.item_data().locatable.archetype_node_id
    }
}

impl ItemApi for Item {
    fn item_data(&self) -> &ItemData {
        match self {
            Item::Cluster(v) => v.item_data(),
            Item::Element(v) => v.item_data(),
        }
    }
}

// PORT NOTE: not part of the spec table — plain Rust ergonomics so callers
// (and the `ITEM_STRUCTURE.as_hierarchy()` covariant-redefinition dispatch
// in `item_structure/item_structure.rs`) can widen a concrete `Cluster`/
// `Element` into the closed `Item` enum without a match arm at every call
// site.
impl From<Cluster> for Item {
    fn from(v: Cluster) -> Self {
        Item::Cluster(v)
    }
}

impl From<Element> for Item {
    fn from(v: Element) -> Self {
        Item::Element(v)
    }
}

pub const TYPE_NAME: &str = "ITEM";

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.representation §ITEM — docs/research/spec-cache/RM-1.1.0/uml_classes/item.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-representation_package.adoc §Class Descriptions / item.adoc §ITEM Class
//   confidence: high
//   todos: 0
//   note: common package landed — LocatableData reconciled and ItemApi now exposes name()/archetype_node_id() by delegating to it (used by the item_structure name/index functions). LOCATABLE's wider concept()/path battery is reached via LocatableApi/PathableApi, deferred to P11. P4/ADR-002: Item enum is #[serde(untagged)] — dispatch via the payload TypeTags on Cluster/Element; ItemData stays untagged (abstract).
// ─────────────────────────────────────────────
