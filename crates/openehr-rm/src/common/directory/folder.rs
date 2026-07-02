//! `FOLDER` — the concept of a named, versionable, hierarchical folder.
//!
//! openEHR class: `FOLDER`, package `common.directory`.
//! Inherits: `LOCATABLE`.
//!
//! A `FOLDER` instance contains more `FOLDER`s and/or items, which are
//! *references* to other (usually versioned) objects. A `FOLDER`
//! structure is therefore like a directory containing references to
//! objects. Since they are references, multiple references to the same
//! object are possible, allowing the structure to be used to multiply
//! classify other objects.
//!
//! It is strongly recommended that the inherited `uid` attribute be
//! populated in top-level (i.e. tree-root) `FOLDER` objects, using the UID
//! copied from the `object_id()` of the `uid` field of the enclosing
//! `VERSION` object.
use crate::common::archetyped::locatable::LocatableData;
use openehr_base::identification::object_ref::ObjectRef;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class in serialized
/// form. Per ADR-001 (Refinements), `serde` derives wait until P4.
pub const TYPE_NAME: &str = "FOLDER";

/// `FOLDER` — a named, archetypable, recursively-nested directory of item
/// references.
///
/// PORT NOTE (recursion hazard, per the invoking task and
/// `.claude/rules/rm-transcription.md` "Recursive containment"): the
/// `folders` field is self-referential (`FOLDER.folders: List<FOLDER>`).
/// Transcribed as `Option<Vec<Folder>>` rather than
/// `Option<Vec<Box<Folder>>>` — `Vec<T>`'s own heap allocation already
/// gives `Folder` a fixed, finite size regardless of `T`'s size (the
/// indirection the recursion needs is supplied by `Vec`'s internal
/// pointer, not by an additional `Box` per element), so no `Box` wrapper
/// is required to satisfy the compiler or to bound `Folder`'s size. This
/// matches the general P3 recursion convention: box only where the
/// recursive field is not already behind a `Vec`/`HashMap`/similar
/// heap-indirecting container (e.g. `DV_MULTIMEDIA.thumbnail: DV_MULTIMEDIA`
/// -- a bare, non-collection self-reference -- genuinely needs `Box`, while
/// `FOLDER.folders: List<FOLDER>` does not).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Folder {
    /// Canonical `_type` discriminator (`"FOLDER"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `LOCATABLE` state (`name`, `archetype_node_id`, `uid`,
    /// `links`, `archetype_details`, `feeder_audit`) per ADR-001 §3.
    #[serde(flatten)]
    pub locatable: LocatableData,

    /// `items`: the list of references to other (usually) versioned
    /// objects logically in this folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<ObjectRef>>,

    /// `folders`: sub-folders of this `FOLDER`.
    ///
    /// Invariant `Folders_valid`: `not folders.is_empty` — i.e. if
    /// present, the list is never empty (mirrored here as `Option<Vec<..>>`
    /// rather than an always-present possibly-empty `Vec<..>`, consistent
    /// with every other `0..1 List<T>` attribute elsewhere in this
    /// transcription pass).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folders: Option<Vec<Folder>>,

    /// `details`: archetypable meta-data for `FOLDER`.
    ///
    /// TODO(port): `ITEM_STRUCTURE` is transcribed in the
    /// `data_structures` package, out of scope for this
    /// change_control/directory transcription pass; forward-referenced.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<crate::data_structures::item_structure::item_structure::ItemStructure>,
}

// Invariants (spec `Invariants` table, not yet enforced by a
// constructor/`Validate` impl — see `.claude/rules/rm-transcription.md`
// "Invariants"):
//   Folders_valid: not folders.is_empty
//     (encoded structurally as Option<Vec<..>> per the doc comment above;
//     a Some(vec![]) value would violate this invariant but is not yet
//     rejected by a constructor.)

impl TypeName for Folder {
    const NAME: &'static str = TYPE_NAME;
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.directory §FOLDER — docs/research/spec-cache/RM-1.1.0/uml_classes/folder.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-directory_package.adoc §Class Descriptions / folder.adoc §FOLDER Class
//   confidence: medium
//   todos: 1
//   note: folders: Option<Vec<Folder>> deliberately NOT boxed (Vec already indirects); details forward-references data_structures::item_structure (P3, not yet transcribed in this pass).
// ─────────────────────────────────────────────
