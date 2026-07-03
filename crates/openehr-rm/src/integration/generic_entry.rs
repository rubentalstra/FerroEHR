//! `GENERIC_ENTRY` — an intermediate representation of non-openEHR-native
//! data.
//!
//! openEHR class: `GENERIC_ENTRY` (concrete), package `rm.integration`.
//!
//! This class is used to create intermediate representations of data from
//! sources not otherwise conforming to openEHR classes, such as HL7
//! messages, relational databases and so on.
//!
//! Unlike other classes in the openEHR reference model, `GENERIC_ENTRY`
//! contains no hard-wired attributes at all beyond `CONTENT_ITEM`'s
//! (transitively `LOCATABLE`'s), only one generic attribute, `data`. No
//! assumptions at all are made about the actual shape of such data.
//!
//! As a subtype of `CONTENT_ITEM`, `GENERIC_ENTRY` is a valid value for
//! `COMPOSITION.content`, and so participates as one of the variants in the
//! `ContentItem` closed enum owned by the `ehr` package cluster
//! (`SECTION`, `ENTRY`-and-descendants, `GENERIC_ENTRY`) — that enum is not
//! defined in this file; this file only supplies the `GenericEntry` struct
//! the `ehr` package's `ContentItem` enum is expected to wrap in a variant.
use crate::data_structures::item_structure::item_tree::ItemTree;
use crate::ehr::content_item::{ContentItemApi, ContentItemData};
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class. Single-sourced into the `TypeName` impl below
/// (ADR-002): the `TypeTag<Self>` first field on [`GenericEntry`] emits
/// `_type: "GENERIC_ENTRY"` itself, and the `ContentItem` enum
/// (`ehr/content_item.rs`) is `#[serde(untagged)]`, dispatching on this
/// payload's own tag.
pub const TYPE_NAME: &str = "GENERIC_ENTRY";

/// `GENERIC_ENTRY` inherits the attribute-less abstract `CONTENT_ITEM`
/// (`content_item.adoc`: `Inherit: LOCATABLE`, no `Attributes` section).
///
/// Per ADR-001 §3, it embeds [`ContentItemData`] (the `ehr` package's
/// embedded-`*Data` half of `CONTENT_ITEM`, in turn embedding
/// `LocatableData`) so it stays consistent with its siblings in the
/// `ContentItem` closed enum — every variant exposes the same
/// `content_item_data()` accessor. `#[serde(flatten)]` folds it (and,
/// transitively, `LocatableData`) into the enclosing
/// `ContentItem::GenericEntry` payload's JSON object; the flat wire shape is
/// unchanged from embedding `LocatableData` directly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericEntry {
    /// Canonical `_type` discriminator (`"GENERIC_ENTRY"`), always
    /// serialized first; tolerated-absent and validated-if-present on input
    /// (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `CONTENT_ITEM` (in turn `LOCATABLE`) state.
    #[serde(flatten)]
    pub content_item: ContentItemData,

    /// `data`: `ITEM_TREE` `[1..1]` — the data from the source message or
    /// record.
    pub data: ItemTree,
}

impl TypeName for GenericEntry {
    const NAME: &'static str = TYPE_NAME;
}

impl ContentItemApi for GenericEntry {
    fn content_item_data(&self) -> &ContentItemData {
        &self.content_item
    }
}

// No `Functions` or `Invariants` sections in this class's own spec table.

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 integration §Class Descriptions GENERIC_ENTRY — docs/research/spec-cache/RM-1.1.0/uml_classes/generic_entry.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-integration_package.adoc §Class Descriptions / uml_classes/generic_entry.adoc §GENERIC_ENTRY Class
//   confidence: medium
//   todos: 0
//   note: embeds LocatableData directly since CONTENT_ITEM adds no attributes of its own; will need to switch to a ContentItemData wrapper if the ehr package introduces one for its ContentItem enum's uniform accessor trait. P4/ADR-002: self-tagging TypeTag<Self> first field + TypeName impl — the ContentItem enum is now untagged and dispatches on this payload's own _type; ItemTree needs its own serde support (data_structures, sibling wave).
// ─────────────────────────────────────────────
