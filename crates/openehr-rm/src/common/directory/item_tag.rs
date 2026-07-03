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
use std::sync::LazyLock;

use openehr_base::identification::object_ref::ObjectRef;
use openehr_base::identification::uid_based_id::UidBasedId;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class in serialized
/// form. Per ADR-001 (Refinements), `serde` derives wait until P4.
pub const TYPE_NAME: &str = "ITEM_TAG";

/// A "justified", non-empty key: the string must begin and end with a
/// non-whitespace character, i.e. be non-empty and carry no leading or
/// trailing whitespace ("is_justified", per the `ITEM_TAG` class
/// description). Internal whitespace (including newlines, hence the `(?s)`
/// dot-matches-newline flag on the middle group) is permitted.
///
/// PORT NOTE: the spec states the constraint prose-only ("may not be empty
/// or contain leading or trailing whitespace"); this regex encodes exactly
/// that — `^\S$` for a single-char key, or `^\S ... \S$` where the middle
/// may be anything. `\S` is the regex crate's non-whitespace class.
static ITEM_TAG_KEY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\S(?s:.*\S)?$").expect("ITEM_TAG key regex is a valid pattern"));

/// `ITEM_TAG` — a lightweight, searchable annotation on a target entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

// Invariants (spec `Invariants` table): implemented as working
// `is_valid()`-family methods per ADR-003 decision 8; the P11
// walker/accumulator Validate framework will call these, they are not yet
// constructor-enforced.
//   Inv_key_valid: not key.is_empty and key.is_justified
//   Inv_value_valid: value /= Void implies not value.is_empty

impl TypeName for ItemTag {
    const NAME: &'static str = TYPE_NAME;
}

impl ItemTag {
    /// Invariant `Inv_key_valid`: `not key.is_empty and key.is_justified`.
    ///
    /// Working method per ADR-003 decision 8. The key must be non-empty and
    /// carry no leading or trailing whitespace ("is_justified"); the
    /// [`ITEM_TAG_KEY_RE`] static encodes exactly that syntax (see its doc
    /// comment).
    pub fn is_key_valid(&self) -> bool {
        ITEM_TAG_KEY_RE.is_match(&self.key)
    }

    /// Invariant `Inv_value_valid`: `value /= Void implies not
    /// value.is_empty`.
    ///
    /// Working method per ADR-003 decision 8. An absent `value` is
    /// vacuously valid; a present one must be non-empty.
    pub fn is_value_valid(&self) -> bool {
        self.value.as_ref().is_none_or(|v| !v.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::change_control::versioned_object::test_support::{hier, object_ref};

    fn tag(key: &str, value: Option<&str>) -> ItemTag {
        ItemTag {
            type_tag: TypeTag::new(),
            target: hier("87284370-2d4b-4e3d-a3f3-f303d2f4f34b").into(),
            target_path: None,
            key: key.to_string(),
            value: value.map(str::to_string),
            owner_id: object_ref("EHR", "b5a56f4c-4574-4759-9bd5-b09be2f0e532"),
        }
    }

    #[test]
    fn key_valid_accepts_a_justified_non_empty_key() {
        assert!(tag("priority", None).is_key_valid());
        // internal whitespace is allowed, only leading/trailing is not.
        assert!(tag("high priority", None).is_key_valid());
        // single non-whitespace character.
        assert!(tag("x", None).is_key_valid());
    }

    #[test]
    fn key_valid_rejects_empty_or_unjustified_keys() {
        assert!(!tag("", None).is_key_valid(), "empty");
        assert!(!tag(" leading", None).is_key_valid(), "leading space");
        assert!(!tag("trailing ", None).is_key_valid(), "trailing space");
        assert!(!tag("\tboth\n", None).is_key_valid(), "tab/newline edges");
        assert!(!tag("   ", None).is_key_valid(), "all whitespace");
    }

    #[test]
    fn value_valid_allows_absent_but_rejects_present_and_empty() {
        assert!(tag("k", None).is_value_valid(), "absent value is valid");
        assert!(tag("k", Some("v")).is_value_valid(), "non-empty value");
        assert!(
            !tag("k", Some("")).is_value_valid(),
            "present-but-empty value is invalid"
        );
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.tags §ITEM_TAG — docs/research/spec-cache/RM-1.1.0/uml_classes/item_tag.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master07-tags.adoc §Class Descriptions / item_tag.adoc §ITEM_TAG Class
//   confidence: high
//   todos: 0
//   note: transcribed from common.tags (master07-tags.adoc), colocated in common/directory/ per this pass's explicit scope instruction rather than its own common/tags/ module; not spec-flagged experimental (that caveat, if any, is EHRbase-server-specific, flagged for later-phase reviewers). Inv_key_valid (not-empty + is_justified) implemented via a LazyLock<Regex> (^\S(?s:.*\S)?$), Inv_value_valid as a working method (ADR-003 d.8); both pinned by accept/reject unit tests. Not yet constructor-enforced (P11 Validate framework).
// ─────────────────────────────────────────────
