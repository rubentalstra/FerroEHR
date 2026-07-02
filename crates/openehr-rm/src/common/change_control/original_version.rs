//! `ORIGINAL_VERSION<T>` — a Version containing locally created content.
//!
//! openEHR class: `ORIGINAL_VERSION<T>`, package
//! `common.change_control`.
//! Inherits: `VERSION<T>`.
//!
//! A Version containing locally created content and optional attestations.
//! Represents a Version created with original content (stored form of data
//! property) at the time of creation (including from non-openEHR local
//! feeder systems), and potentially attested (signed). All instances of
//! `VERSION<T>` in non-distributed openEHR systems will be instances of
//! `ORIGINAL_VERSION<T>`; it is also the unit of copying in a distributed
//! environment.
use openehr_base::identification::object_version_id::ObjectVersionId;

use crate::common::change_control::version::{VersionApi, VersionData};
use crate::common::generic::attestation::Attestation;
use crate::data_types::text::dv_coded_text::DvCodedText;

/// Canonical `_type` discriminator string for this class in serialized
/// form. Per ADR-001 (Refinements), `serde` derives wait until P4.
pub const TYPE_NAME: &str = "ORIGINAL_VERSION";

/// `ORIGINAL_VERSION<T>` — locally created, optionally attested content.
///
/// Stores (rather than computes) the three functions the parent
/// `VERSION<T>` declares abstract (`uid`, `preceding_version_uid`,
/// `lifecycle_state`, `data`) as its own attributes — the spec's own
/// per-class table lists these as "Stored version of inheritance
/// precursor." for `uid`/`preceding_version_uid`, and declares
/// `lifecycle_state`/`data` as ordinary attributes directly. Compare
/// `ImportedVersion`, which computes the same four by delegating to its
/// wrapped `item` instead of storing them.
#[derive(Debug, Clone)]
pub struct OriginalVersion<T> {
    /// Embedded `VERSION<T>` state (`contribution`, `signature`,
    /// `commit_audit`) per ADR-001 §3.
    pub version: VersionData,

    /// `uid`: stored version of inheritance precursor
    /// (`VERSION.uid(): OBJECT_VERSION_ID`, abstract there).
    pub uid: ObjectVersionId,

    /// `preceding_version_uid`: stored version of inheritance precursor
    /// (`VERSION.preceding_version_uid(): OBJECT_VERSION_ID`, abstract
    /// there). `Void` if this is the first version.
    pub preceding_version_uid: Option<ObjectVersionId>,

    /// `other_input_version_uids`: identifiers of other versions whose
    /// content was merged into this version, if any.
    ///
    /// Invariant `Is_merged_validity`: `other_input_version_ids = Void xor
    /// is_merged` — see [`OriginalVersion::is_merged`].
    ///
    /// Invariant `Other_input_version_uids_valid`:
    /// `other_input_version_uids /= Void implies not
    /// other_input_version_uids.is_empty`.
    pub other_input_version_uids: Option<Vec<ObjectVersionId>>,

    /// `lifecycle_state`: lifecycle state of the content item in this
    /// version; coded by openEHR vocabulary "version lifecycle state".
    pub lifecycle_state: DvCodedText,

    /// `attestations`: set of attestations relating to this version.
    ///
    /// Invariant `Attestations_valid`: `attestations /= Void implies not
    /// attestations.is_empty`.
    pub attestations: Option<Vec<Attestation>>,

    /// `data`: data content of this Version.
    pub data: Option<T>,
}

impl<T> OriginalVersion<T> {
    /// `is_merged(): Boolean`.
    ///
    /// True if this Version was created from more than just the preceding
    /// (checked out) version.
    ///
    /// PORT NOTE: transcribed from `other_input_version_uids`'s presence,
    /// matching the `Is_merged_validity` invariant
    /// (`other_input_version_ids = Void xor is_merged`) — the spec table
    /// does not otherwise define `is_merged`'s body, only its invariant
    /// relationship to `other_input_version_uids`.
    pub fn is_merged(&self) -> bool {
        self.other_input_version_uids.is_some()
    }
}

impl<T> VersionApi<T> for OriginalVersion<T> {
    fn uid(&self) -> &ObjectVersionId {
        &self.uid
    }

    fn preceding_version_uid(&self) -> Option<&ObjectVersionId> {
        self.preceding_version_uid.as_ref()
    }

    fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    fn lifecycle_state(&self) -> &DvCodedText {
        &self.lifecycle_state
    }
}

// Invariants (spec `Invariants` table, not yet enforced by a
// constructor/`Validate` impl — see `.claude/rules/rm-transcription.md`
// "Invariants"):
//   Attestations_valid: attestations /= Void implies not attestations.is_empty
//   Is_merged_validity: other_input_version_ids = Void xor is_merged
//     (structurally guaranteed by is_merged()'s own definition above, but
//     documented as the spec invariant it satisfies rather than enforced
//     by a separate Validate check.)
//   Other_input_version_uids_valid:
//     other_input_version_uids /= Void implies not
//     other_input_version_uids.is_empty

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.change_control §ORIGINAL_VERSION — docs/research/spec-cache/RM-1.1.0/uml_classes/original_version.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-change_control_package.adoc §Class Descriptions / original_version.adoc §ORIGINAL_VERSION Class
//   confidence: high
//   todos: 0
//   note: is_merged() derived structurally from other_input_version_uids per the Is_merged_validity invariant, since the spec gives no separate body for it.
// ─────────────────────────────────────────────
