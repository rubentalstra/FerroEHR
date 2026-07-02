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
use crate::common::archetyped::locatable::LocatableData;
use crate::data_structures::item_structure::item_tree::ItemTree;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// `pub const TYPE_NAME`: the canonical `_type` discriminator string for
/// this concrete class. Single-sourced into the `TypeName` impl below
/// (ADR-002): the `TypeTag<Self>` first field on [`GenericEntry`] emits
/// `_type: "GENERIC_ENTRY"` itself, and the `ContentItem` enum
/// (`ehr/content_item.rs`) is `#[serde(untagged)]`, dispatching on this
/// payload's own tag.
pub const TYPE_NAME: &str = "GENERIC_ENTRY";

/// Shared attribute state of `CONTENT_ITEM` — an abstract class with no
/// attributes of its own beyond `LOCATABLE`'s (see
/// `content_item.adoc`: `Inherit: LOCATABLE`, no `Attributes` section).
///
/// Per ADR-001 §3, this would ordinarily be `ContentItemData` embedding
/// `LocatableData`, but since `CONTENT_ITEM` adds nothing over `LOCATABLE`,
/// `GENERIC_ENTRY` embeds `LocatableData` directly rather than introducing
/// a pass-through wrapper struct with no fields of its own. `#[serde(flatten)]`
/// folds `LocatableData` into the enclosing `ContentItem::GenericEntry`
/// payload's JSON object.
///
/// TODO(port): if a later transcription of `CONTENT_ITEM` (owned by the
/// `ehr` package cluster) introduces a `ContentItemData`/`ContentItemApi`
/// pair for other reasons (e.g. to give the `ContentItem` enum a uniform
/// accessor trait matching `SECTION`/`ENTRY`), `GenericEntry` should switch
/// to embedding that struct instead of `LocatableData` directly, to stay
/// consistent with its siblings in the `ContentItem` enum.
///
/// TODO(port): P4 — the flatten below requires `LocatableData` to itself
/// derive `Serialize`/`Deserialize` (sibling P4 wave over `common/`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericEntry {
    /// Canonical `_type` discriminator (`"GENERIC_ENTRY"`), always
    /// serialized first; tolerated-absent and validated-if-present on input
    /// (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Inherited `LOCATABLE` state (via the attribute-less `CONTENT_ITEM`).
    #[serde(flatten)]
    pub locatable: LocatableData,

    /// `data`: `ITEM_TREE` `[1..1]` — the data from the source message or
    /// record.
    pub data: ItemTree,
}

impl TypeName for GenericEntry {
    const NAME: &'static str = TYPE_NAME;
}

// No `Functions` or `Invariants` sections in this class's own spec table.

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 integration §Class Descriptions GENERIC_ENTRY — docs/research/spec-cache/RM-1.1.0/uml_classes/generic_entry.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-integration_package.adoc §Class Descriptions / uml_classes/generic_entry.adoc §GENERIC_ENTRY Class
//   confidence: medium
//   todos: 2
//   note: embeds LocatableData directly since CONTENT_ITEM adds no attributes of its own; will need to switch to a ContentItemData wrapper if the ehr package introduces one for its ContentItem enum's uniform accessor trait. P4/ADR-002: self-tagging TypeTag<Self> first field + TypeName impl — the ContentItem enum is now untagged and dispatches on this payload's own _type; ItemTree needs its own serde support (data_structures, sibling wave).
// ─────────────────────────────────────────────
