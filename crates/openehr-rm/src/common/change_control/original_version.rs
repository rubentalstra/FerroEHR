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
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OriginalVersion<T> {
    /// Canonical `_type` discriminator (`"ORIGINAL_VERSION"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<OriginalVersion<T>>,

    /// Embedded `VERSION<T>` state (`contribution`, `signature`,
    /// `commit_audit`) per ADR-001 §3.
    #[serde(flatten)]
    pub version: VersionData,

    /// `uid`: stored version of inheritance precursor
    /// (`VERSION.uid(): OBJECT_VERSION_ID`, abstract there).
    pub uid: ObjectVersionId,

    /// `preceding_version_uid`: stored version of inheritance precursor
    /// (`VERSION.preceding_version_uid(): OBJECT_VERSION_ID`, abstract
    /// there). `Void` if this is the first version.
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_input_version_uids: Option<Vec<ObjectVersionId>>,

    /// `lifecycle_state`: lifecycle state of the content item in this
    /// version; coded by openEHR vocabulary "version lifecycle state".
    pub lifecycle_state: DvCodedText,

    /// `attestations`: set of attestations relating to this version.
    ///
    /// Invariant `Attestations_valid`: `attestations /= Void implies not
    /// attestations.is_empty`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestations: Option<Vec<Attestation>>,

    /// `data`: data content of this Version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> TypeName for OriginalVersion<T> {
    const NAME: &'static str = TYPE_NAME;
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

    /// Invariant `Attestations_valid`: `attestations /= Void implies not
    /// attestations.is_empty`.
    ///
    /// Working method per ADR-003 decision 8. An absent `attestations` list
    /// is vacuously valid; a present one must be non-empty.
    pub fn are_attestations_valid(&self) -> bool {
        self.attestations.as_ref().is_none_or(|a| !a.is_empty())
    }

    /// Invariant `Other_input_version_uids_valid`:
    /// `other_input_version_uids /= Void implies not
    /// other_input_version_uids.is_empty`.
    ///
    /// Working method per ADR-003 decision 8. An absent list is vacuously
    /// valid; a present one must name at least one other input version.
    pub fn are_other_input_version_uids_valid(&self) -> bool {
        self.other_input_version_uids
            .as_ref()
            .is_none_or(|u| !u.is_empty())
    }

    /// Invariant `Is_merged_validity`: `other_input_version_ids = Void xor
    /// is_merged`.
    ///
    /// Working method per ADR-003 decision 8. Structurally guaranteed by
    /// [`OriginalVersion::is_merged`]'s own definition (`is_merged ==
    /// other_input_version_uids.is_some()`), so this always holds; kept as
    /// an explicit check for the P11 Validate framework to call.
    pub fn is_merged_validity_satisfied(&self) -> bool {
        self.other_input_version_uids.is_none() ^ self.is_merged()
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

// Invariants (spec `Invariants` table): implemented as working
// `is_valid()`-family methods per ADR-003 decision 8 —
// `are_attestations_valid()`, `are_other_input_version_uids_valid()`, and
// `is_merged_validity_satisfied()`. The P11 walker/accumulator Validate
// framework will call these; they are not yet constructor-enforced.
//   Attestations_valid: attestations /= Void implies not attestations.is_empty
//   Is_merged_validity: other_input_version_ids = Void xor is_merged
//   Other_input_version_uids_valid:
//     other_input_version_uids /= Void implies not
//     other_input_version_uids.is_empty

#[cfg(test)]
mod tests {
    use crate::common::change_control::version::VersionApi;
    use crate::common::change_control::versioned_object::test_support::{original_version, ovid};

    #[test]
    fn merge_invariants_track_other_input_version_uids() {
        // Non-merged version: no other inputs.
        let plain = original_version("c::sys::1", None, "2020-01-01T00:00:00", "532");
        assert!(!plain.is_merged());
        assert!(plain.are_other_input_version_uids_valid());
        assert!(plain.is_merged_validity_satisfied());
        assert!(plain.are_attestations_valid());

        // Merged version: at least one other input → valid, is_merged true.
        let mut merged =
            original_version("c::sys::2", Some("c::sys::1"), "2020-01-01T00:00:00", "532");
        merged.other_input_version_uids = Some(vec![ovid("c::sys::1")]);
        assert!(merged.is_merged());
        assert!(merged.are_other_input_version_uids_valid());
        assert!(merged.is_merged_validity_satisfied());

        // Present-but-empty other-inputs list violates the non-empty invariant.
        let mut empty_merge =
            original_version("c::sys::3", Some("c::sys::2"), "2020-01-01T00:00:00", "532");
        empty_merge.other_input_version_uids = Some(Vec::new());
        assert!(!empty_merge.are_other_input_version_uids_valid());

        // Present-but-empty attestations list violates Attestations_valid.
        let mut empty_att = original_version("c::sys::4", None, "2020-01-01T00:00:00", "532");
        empty_att.attestations = Some(Vec::new());
        assert!(!empty_att.are_attestations_valid());

        // sanity: lifecycle_state accessor still resolves via VersionApi.
        assert_eq!(plain.lifecycle_state().defining_code.code_string, "532");
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.change_control §ORIGINAL_VERSION — docs/research/spec-cache/RM-1.1.0/uml_classes/original_version.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-change_control_package.adoc §Class Descriptions / original_version.adoc §ORIGINAL_VERSION Class
//   confidence: high
//   todos: 0
//   note: is_merged() derived structurally from other_input_version_uids per the Is_merged_validity invariant, since the spec gives no separate body for it. All three spec invariants (Attestations_valid, Is_merged_validity, Other_input_version_uids_valid) now implemented as working is_valid()-family methods per ADR-003 d.8 with a spec-derived unit test; P11 Validate-framework wiring still pending.
// ─────────────────────────────────────────────
