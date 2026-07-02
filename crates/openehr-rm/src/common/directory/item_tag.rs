//! `ITEM_TAG` — a tag with optional value, attached to a target entity.
//!
//! openEHR class: `ITEM_TAG`, package `common.tags`.
//!
//! PORT NOTE: `ITEM_TAG` belongs to the `common.tags` package
//! (`master07-tags.adoc`), not `common.directory` or
//! `common.change_control`. It is colocated in this `directory/` module
//! per the invoking transcription task's explicit instruction ("Include
//! any VERSIONED_FOLDER binding and the ITEM_TAG class if the tags chapter
//! declares one"), rather than creating a third, separate
//! `common/tags/` directory outside this pass's assigned scope. Move this
//! file to a dedicated `common/tags/` module if/when that package is
//! transcribed as its own unit.
//!
//! The `common.tags` package defines the structure and semantics of a
//! *tag* construct usable within openEHR, similar to 'tagging' facilities
//! in web-based email, forum platforms, and other content management
//! systems. A tag consists of a `key` and optional `value`; tags are
//! normally associated with an 'owner' object, identified by `owner_id`,
//! such that deletion or move of the owner results in deletion or move of
//! the associated tags.
//!
//! This class is **not** flagged as experimental or trial in the RM 1.1.0
//! spec text itself — the `common.tags` chapter carries no `[.tbd]` or
//! development/trial marker, unlike (for example) `rm.ehr_extract`, which
//! the master plan (`PORT_MASTER_PLAN.md` §7.1) explicitly calls out as
//! "experimental, defer". EHRbase's own Item Tags feature
//! (`PORT_MASTER_PLAN.md` §6, listed among "experimental Item Tags" REST
//! endpoints) is the *server-side implementation status* of this RM class,
//! not a property of the RM 1.1.0 specification text transcribed here —
//! flagged for the reviewer to weigh whether that EHRbase-level caveat
//! should propagate to how this type is wired up in later phases (P6+).
use openehr_base::identification::object_ref::ObjectRef;
use openehr_base::identification::uid_based_id::UidBasedId;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class in serialized
/// form. Per ADR-001 (Refinements), `serde` derives wait until P4.
pub const TYPE_NAME: &str = "ITEM_TAG";

/// `ITEM_TAG` — a lightweight, searchable annotation on a target entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemTag {
    /// Canonical `_type` discriminator (`"ITEM_TAG"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `target`: identifier of target, which may be a `VERSIONED_OBJECT<T>`
    /// or a `VERSION<T>`.
    pub target: UidBasedId,

    /// `target_path`: optional archetype (i.e. AQL) or RM path within
    /// `target`, in order to tag a fine-grained element.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_path: Option<String>,

    /// `key`: the tag key.
    ///
    /// Invariant `Inv_key_valid`: `not key.is_empty and key.is_justified`
    /// — i.e. may not be empty or contain leading or trailing whitespace
    /// ("is_justified" being the spec's term, per the class description,
    /// for having no leading/trailing whitespace).
    pub key: String,

    /// `value`: the value.
    ///
    /// Invariant `Inv_value_valid`: `value /= Void implies not
    /// value.is_empty` — if set, may not be empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// `owner_id`: identifier of owner object, such as EHR.
    pub owner_id: ObjectRef,
}

// Invariants (spec `Invariants` table, not yet enforced by a
// constructor/`Validate` impl — see `.claude/rules/rm-transcription.md`
// "Invariants"):
//   Inv_key_valid: not key.is_empty and key.is_justified
//     TODO(port): "is_justified" (no leading/trailing whitespace) needs a
//     runtime check; not yet wired into a constructor.
//   Inv_value_valid: value /= Void implies not value.is_empty

impl TypeName for ItemTag {
    const NAME: &'static str = TYPE_NAME;
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.tags §ITEM_TAG — docs/research/spec-cache/RM-1.1.0/uml_classes/item_tag.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master07-tags.adoc §Class Descriptions / item_tag.adoc §ITEM_TAG Class
//   confidence: medium
//   todos: 1
//   note: transcribed from common.tags (master07-tags.adoc), colocated in common/directory/ per this pass's explicit scope instruction rather than its own common/tags/ module; not spec-flagged experimental (that caveat, if any, is EHRbase-server-specific, flagged for later-phase reviewers).
// ─────────────────────────────────────────────
