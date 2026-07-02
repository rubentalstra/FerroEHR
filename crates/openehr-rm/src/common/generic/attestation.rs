//! `ATTESTATION` — an explicit signing of record content by a healthcare
//! agent.
//!
//! openEHR class: `ATTESTATION` (concrete), package `common.generic`.
//! Inherits: `AUDIT_DETAILS`.
//!
//! Record an attestation of a party (the committer) to item(s) of record
//! content. An attestation is an explicit signing by one healthcare agent
//! of particular content for various particular purposes, including:
//!
//! * authorisation of a controlled substance or procedure (e.g.
//!   sectioning of patient under mental health act);
//! * witnessing of content by senior clinical professional;
//! * indicating acknowledgement of content by intended recipient, e.g. GP
//!   who ordered a test result.
//!
//! Here it is modelled as a subtype of `AUDIT_DETAILS`, meaning that it is
//! logically a kind of audit, with additional information pertinent to
//! the act of signing. The contents of an `ATTESTATION` are:
//!
//! * the identity of the attesting party (`AUDIT_DETAILS.committer`);
//! * the date and time of the action of attestation
//!   (`AUDIT_DETAILS.time_committed`);
//! * references to items in the record being attested to
//!   (`ATTESTATION.items`); if this list is empty, the attestation is for
//!   the entire object (usually the content of an `ORIGINAL_VERSION`) to
//!   which the attestation is attached, otherwise the list must contain a
//!   set of paths to items within the item to which the attestation is
//!   attached;
//! * an optionally coded reason for attestation (`ATTESTATION.reason`);
//! * an optional literal view of the content attested, e.g. a binary
//!   screen image;
//! * a proof of attestation in the form of a digital signature by the
//!   attesting party.
//!
//! The digital signature, if present, is generated using the openPGP
//! standard (IETF RFC 4880): the attestation object is serialised into a
//! canonical text form, hashed to create a digest, and the digest signed
//! with the user's private key; the result is radix-64 encoded and written
//! back into the `proof` attribute. The exact serialisation is not yet
//! defined by openEHR (marked "To Be Determined" in the spec).
//!
//! The `is_pending` attribute marks the attestation as either having been
//! done or awaiting completion. When an attestation is required, the
//! common scenario is that a Composition Version is committed with a
//! `commit_audit` of type `ATTESTATION` with `is_pending` set to `True`;
//! when signing occurs, a new `ATTESTATION` is added to
//! `VERSION.attestations`, this time with `is_pending` set to `False` and
//! the appropriate proof supplied.

// TODO(port): `DV_MULTIMEDIA`, `DV_EHR_URI`, `DV_TEXT` are RM 1.1.0
// `data_types.encapsulated`/`data_types.uri`/`data_types.text`,
// transcribed by a sibling agent in this same phase but not yet landed in
// this worktree. Forward-references to their eventual module paths.
use crate::data_types::encapsulated::dv_multimedia::DvMultimedia;
use crate::data_types::text::dv_text::DvText;
use crate::data_types::uri::dv_ehr_uri::DvEhrUri;

use super::audit_details::AuditDetails;

/// Canonical `_type` discriminator string for this class in serialized
/// form. Per ADR-001 refinements ("serde derives wait until P4"), a
/// `const` stands in for `#[serde(rename = ...)]` until serde lands as a
/// dependency of this crate.
pub const TYPE_NAME: &str = "ATTESTATION";

/// `ATTESTATION inherits AUDIT_DETAILS` per its `Inherit` row. Per
/// ADR-001 §3, the entire inherited attribute set (`system_id`,
/// `time_committed`, `change_type`, `description`, `committer`) is carried
/// via an embedded [`AuditDetails`] field, with the five new attributes
/// declared directly on this struct.
#[derive(Debug, Clone, PartialEq)]
pub struct Attestation {
    /// Embedded `AUDIT_DETAILS` state (`system_id`, `time_committed`,
    /// `change_type`, `description`, `committer`).
    pub audit_details: AuditDetails,

    /// `attested_view`: `DV_MULTIMEDIA`, cardinality `0..1`.
    ///
    /// Optional visual representation of content attested, e.g. screen
    /// image.
    pub attested_view: Option<DvMultimedia>,

    /// `proof`: `String`, cardinality `0..1`.
    ///
    /// Proof of attestation.
    pub proof: Option<String>,

    /// `items`: `List<DV_EHR_URI>`, cardinality `0..1`.
    ///
    /// Items attested, expressed as fully qualified runtime paths to the
    /// items in question. Although not recommended, these may include
    /// fine-grained items which have been attested in some other system.
    /// Otherwise it is assumed to be for the entire `VERSION` with which
    /// it is associated.
    ///
    /// Invariant `Items_valid`: `items /= Void implies not
    /// items.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl; recorded here as a doc note pending the RM invariant
    /// framework (`.claude/rules/rm-transcription.md` "Invariants").
    pub items: Option<Vec<DvEhrUri>>,

    /// `reason`: `DV_TEXT`, cardinality `1..1`.
    ///
    /// Reason of this attestation. Optionally coded by the openEHR
    /// Terminology group "attestation reason"; includes values like
    /// "authorisation", "witness" etc.
    ///
    /// Invariant `Reason_valid`:
    /// `reason.generating_type.is_equal("DV_CODED_TEXT") implies
    /// terminology(Terminology_id_openehr).has_code_for_group_id(
    /// Group_id_attestation_reason, reason.defining_code)`.
    ///
    /// TODO(port): invariant requires runtime-type discrimination of
    /// `DV_TEXT` plus a live `TerminologyService`; not yet enforced. See
    /// [`Attestation::is_reason_valid`].
    pub reason: DvText,

    /// `is_pending`: `Boolean`, cardinality `1..1`.
    ///
    /// True if this attestation is outstanding; False means it has been
    /// completed.
    pub is_pending: bool,
}

impl Attestation {
    /// Invariant `Items_valid`: `items /= Void implies not
    /// items.is_empty`.
    ///
    /// TODO(port): not yet wired into a constructor or the RM `Validate`
    /// framework; this method lets a future `Validate` impl call the check
    /// directly once that framework lands.
    pub fn are_items_valid(&self) -> bool {
        match &self.items {
            Some(items) => !items.is_empty(),
            None => true,
        }
    }

    /// Invariant `Reason_valid`:
    /// `(reason.generating_type.is_equal("DV_CODED_TEXT") implies
    /// terminology(Terminology_id_openehr).has_code_for_group_id(
    /// Group_id_attestation_reason, reason.defining_code))`.
    ///
    /// TODO(port): `reason` is typed `DV_TEXT` (the wider supertype), but
    /// the invariant only constrains it when the runtime value happens to
    /// be a `DV_CODED_TEXT`. Requires runtime-type inspection of `DV_TEXT`
    /// (a closed enum per ADR-001 §4, once `data_types.text` is
    /// transcribed) plus a live `TerminologyService`; left as `todo!()`
    /// rather than a bare boolean stub since neither prerequisite exists
    /// yet.
    pub fn is_reason_valid(&self, _terminology: &openehr_terminology::TerminologyService) -> bool {
        todo!(
            "Attestation::is_reason_valid: needs DV_TEXT runtime-type discrimination plus TerminologyService.has_code_for_group_id against Group_id_attestation_reason"
        )
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.generic — docs/research/spec-cache/RM-1.1.0/uml_classes/attestation.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master04-generic_package.adoc §Attestation / uml_classes/attestation.adoc §ATTESTATION Class
//   confidence: high
//   todos: 2
//   note: Items_valid recorded as a self-contained boolean check; Reason_valid left todo!()-bodied (needs DV_TEXT runtime-type discrimination + live TerminologyService). Embeds AuditDetails per its Inherit row. Forward-refs DvMultimedia, DvEhrUri, DvText (data_types, sibling-agent territory, not yet landed). Digital-signature generation process (openPGP/RFC 4880) is described in prose only, no function signature to transcribe — proof stays a plain String.
// ─────────────────────────────────────────────
