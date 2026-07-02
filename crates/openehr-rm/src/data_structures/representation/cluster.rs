//! `CLUSTER` — the grouping variant of `ITEM`.
//!
//! openEHR class: `CLUSTER`, package `rm.data_structures.representation`.
//!
//! The grouping variant of `ITEM`, which may contain further instances of
//! `ITEM`, in an ordered list.

use super::item::{Item, ItemApi, ItemData};

/// `CLUSTER` class.
///
/// Embeds the shared `ITEM` state (per ADR-001 §3) plus its own `items`
/// attribute.
#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    /// Inherited `ITEM` (and transitively `LOCATABLE`) state.
    pub item: ItemData,

    /// `items`: ordered list of items — `CLUSTER` or `ELEMENT` objects —
    /// under this `CLUSTER`.
    ///
    /// Cardinality `1..1` per the spec table (not optional).
    ///
    /// Recursion note: this is the recursive-containment edge for `ITEM`
    /// (a `CLUSTER` can nest further `CLUSTER`s via `Item::Cluster`). See
    /// the doc comment on `Item` (`item.rs`) for why `Vec<Item>` alone
    /// (without an additional `Box`) already satisfies the boxing rule for
    /// recursive containment — the `Vec`'s heap-allocated backing storage
    /// is the indirection that gives `Cluster` (and `Item`) a finite,
    /// statically-known size.
    pub items: Vec<Item>,
}

impl ItemApi for Cluster {
    fn item_data(&self) -> &ItemData {
        &self.item
    }
}

pub const TYPE_NAME: &str = "CLUSTER";

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.representation §CLUSTER — docs/research/spec-cache/RM-1.1.0/uml_classes/cluster.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-representation_package.adoc §Class Descriptions / cluster.adoc §CLUSTER Class
//   confidence: high
//   todos: 0
//   note: no invariants declared for CLUSTER in the spec table; item_structure package narrative (master04) documents the ISO 13606 encoding rules that produce CLUSTERs but imposes no additional CLUSTER-level constraint.
// ─────────────────────────────────────────────
