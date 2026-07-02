//! `DATA_STRUCTURE` — abstract ancestor of all openEHR data structures.
//!
//! openEHR class: `DATA_STRUCTURE` (abstract), package `rm.data_structures`
//! (the package root, above `item_structure`/`history`/`representation`).
//!
//! Abstract parent class of all data structure types. Includes the
//! `as_hierarchy` function which can generate the equivalent CEN EN13606
//! single hierarchy for each subtype's physical representation. For
//! example, the physical representation of an `ITEM_LIST` is
//! `List<ELEMENT>`; its implementation of `as_hierarchy` will generate a
//! `CLUSTER` containing the set of `ELEMENT` nodes from the list.

use crate::data_structures::representation::item::Item;
// PORT NOTE: `LOCATABLE` is owned by the `common` package cluster (a
// sibling RM package transcribed concurrently). See `representation/item.rs`
// for the identical forward-reference rationale.
use crate::common::archetyped::locatable::LocatableData;

/// Shared attribute state of `DATA_STRUCTURE` and its descendants.
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct +
/// marker trait). `DATA_STRUCTURE` declares no attributes of its own beyond
/// the inherited `LOCATABLE` state, so this struct is presently just that
/// embedding, kept as a named type so both the `ITEM_STRUCTURE` subtree
/// (`item_structure/`) and the `HISTORY<T>` class (`history/history.rs`)
/// — the two direct descendants of `DATA_STRUCTURE` — have a common,
/// stable embedding point.
#[derive(Debug, Clone, PartialEq)]
pub struct DataStructureData {
    /// Inherited `LOCATABLE` state.
    ///
    /// TODO(port): forward reference; see `representation/item.rs`.
    pub locatable: LocatableData,
}

/// Behaviour shared by every `DATA_STRUCTURE` descendant.
///
/// `DATA_STRUCTURE` is abstract-with-a-function-but-no-declared-attributes
/// in the narrow sense (its own `Attributes` table is empty; the only row
/// in its spec table is the `Functions` row for `as_hierarchy`), so per
/// ADR-001 §1/§3 this trait is the natural place for that shared function
/// signature to live — every concrete `DATA_STRUCTURE` descendant across
/// `item_structure` (`ItemSingle`, `ItemList`, `ItemTable`, `ItemTree`) and
/// `history` (`History<T>`) must supply its own `as_hierarchy()`.
///
/// Covariant redefinition note (ADR-001 §6): the spec declares
/// `as_hierarchy(): ITEM` on `DATA_STRUCTURE` itself, then **redefines** it
/// on every concrete `ITEM_STRUCTURE` subtype to a narrower return type
/// (`ITEM_SINGLE.as_hierarchy(): ELEMENT`,
/// `ITEM_LIST/ITEM_TABLE/ITEM_TREE.as_hierarchy(): CLUSTER`). Rust traits
/// cannot express a per-implementor return-type covariant override of a
/// single trait method signature (the trait method's return type is fixed
/// for every impl). Two shapes were considered:
///
/// 1. Give the trait method the widest declared return type (`Item`), and
///    have each concrete type's inherent `as_hierarchy()` (declared outside
///    this trait, with its own narrower return type) be the "real",
///    spec-faithful redefinition, with the trait method itself unused by
///    concrete callers who know their concrete type statically.
/// 2. Drop the shared trait method entirely and let every concrete type
///    declare its own inherent `as_hierarchy()` with its spec-narrowed
///    return type, relying on `Item`'s enum variants only where genuine
///    `DATA_STRUCTURE`-level polymorphism is needed.
///
/// Shape 1 is used here: `DataStructureBehaviour::as_hierarchy` returns the
/// widest type (`Item`), and each concrete `ITEM_STRUCTURE` subtype
/// (`item_structure/item_single.rs`, `item_list.rs`, `item_table.rs`,
/// `item_tree.rs`) additionally declares its own narrower inherent
/// `as_hierarchy()` returning `Element`/`Cluster` directly, documented at
/// the point of the override — matching how ADR-001 §6 treats
/// `LOCATABLE_REF.id`. Callers holding a concrete type get the narrow
/// return; callers holding only `dyn DataStructureBehaviour` or working
/// generically get the widened `Item`.
pub trait DataStructureBehaviour {
    /// `as_hierarchy`: hierarchical equivalent of the physical
    /// representation of each subtype, compatible with CEN EN 13606
    /// structures.
    fn as_hierarchy(&self) -> Item;
}

pub const TYPE_NAME: &str = "DATA_STRUCTURE";

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures §DATA_STRUCTURE — docs/research/spec-cache/RM-1.1.0/uml_classes/data_structure.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master03-overview.adoc §Class Descriptions / data_structure.adoc §DATA_STRUCTURE Class
//   confidence: medium
//   todos: 1
//   note: the as_hierarchy() covariant-redefinition shape (widened trait method + narrowed inherent override per concrete type) is a judgment call documented on DataStructureBehaviour; LocatableData is a forward reference to the concurrently-transcribed common package.
// ─────────────────────────────────────────────
