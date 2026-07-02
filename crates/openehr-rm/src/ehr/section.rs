//! `SECTION` — a heading in a heading structure ("section tree").
//!
//! openEHR class: `SECTION`, package `rm.ehr.navigation`.
//! Inherits: `CONTENT_ITEM`.
//!
//! Represents a heading in a heading structure, or "section tree". Created
//! according to archetyped structures for typical headings such as SOAP,
//! physical examination, but also pathology result heading structures.
//! Should not be used instead of `ENTRY` hierarchical structures.
//!
//! # Recursion through the `ContentItem` enum (no extra `Box` needed)
//!
//! `SECTION.items` is `List<CONTENT_ITEM>`, and `CONTENT_ITEM` closes (per
//! [`super::content_item::ContentItem`], ADR-001 §4) over
//! [`super::content_item::ContentItem::Section`] itself as one of its
//! variants. This makes `Section` recursive: a `Section` can contain
//! `ContentItem::Section(Section { .. })` values in its own `items`. The
//! recursion is already broken by the `Vec` indirection alone — `Vec<T>`
//! is heap-allocated regardless of `T`'s size, so `Section` does not need
//! an *additional* `Box` around `items` or around the `ContentItem::Section`
//! variant's payload to have a finite size. This differs from the crate's
//! other boxed-recursion cases (`FOLDER`, `CLUSTER`, `ITEM_TREE`,
//! `DV_MULTIMEDIA.thumbnail`), which recurse through a bare `Option<Self>`
//! or `Option<Box<Self>>` field with no interposed collection type.
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class in serialized
/// form. Single-sourced into the `TypeName` impl below (ADR-002).
pub const TYPE_NAME: &str = "SECTION";

/// `SECTION` — a heading in a heading structure ("section tree").
///
/// `SECTION` inherits `CONTENT_ITEM` directly (not through `ENTRY`), so it
/// embeds [`super::content_item::ContentItemData`] rather than
/// [`super::entry::EntryData`]. `#[serde(flatten)]` folds `ContentItemData`
/// into `SECTION`'s own JSON object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Section {
    /// Canonical `_type` discriminator (`"SECTION"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `CONTENT_ITEM` (in turn `LOCATABLE`) state.
    #[serde(flatten)]
    pub content_item: super::content_item::ContentItemData,

    /// `items`: ordered list of content items under this section, which
    /// may include more `SECTION`s or `ENTRY`s.
    ///
    /// Invariant `Items_valid`: `items /= Void implies not
    /// items.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl.
    ///
    /// See the module-level doc comment for why no `Box` is needed here
    /// despite the recursion through `ContentItem::Section`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub items: Option<Vec<super::content_item::ContentItem>>,
}

impl TypeName for Section {
    const NAME: &'static str = TYPE_NAME;
}

impl super::content_item::ContentItemApi for Section {
    fn content_item_data(&self) -> &super::content_item::ContentItemData {
        &self.content_item
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.navigation — docs/research/spec-cache/RM-1.1.0/uml_classes/section.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master07-navigation_package.adoc §Class Descriptions / section.adoc §SECTION Class
//   confidence: high
//   todos: 1
//   note: recursion flows through the ContentItem enum + Vec indirection alone (documented at module level); Items_valid invariant left unimplemented. P4/ADR-002: self-tagging TypeTag<Self> first field + TypeName impl (no-op struct-level rename removed); flatten kept on content_item, items skip-if-none; the untagged ContentItem enum dispatches on this payload's own _type.
// ─────────────────────────────────────────────
