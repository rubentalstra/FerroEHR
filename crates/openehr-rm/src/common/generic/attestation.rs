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
use openehr_foundation::serde_support::{TypeName, TypeTag};
use openehr_term::{
    OpenehrTerminologyGroupIdentifiers, TerminologyAccess, TerminologyCode, TerminologyService,
};
use serde::{Deserialize, Serialize};

use super::audit_details::AuditDetailsData;

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attestation {
    /// Canonical `_type` discriminator (`"ATTESTATION"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `AUDIT_DETAILS` state (`system_id`, `time_committed`,
    /// `change_type`, `description`, `committer`).
    ///
    /// PORT NOTE: flattens the untagged [`AuditDetailsData`] (not the
    /// self-tagged `AuditDetails` wrapper) so the parent's `_type` never
    /// leaks into `ATTESTATION` output (ADR-002).
    #[serde(flatten)]
    pub audit_details: AuditDetailsData,

    /// `attested_view`: `DV_MULTIMEDIA`, cardinality `0..1`.
    ///
    /// Optional visual representation of content attested, e.g. screen
    /// image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attested_view: Option<DvMultimedia>,

    /// `proof`: `String`, cardinality `0..1`.
    ///
    /// Proof of attestation.
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
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
    /// Checked by [`Attestation::is_reason_valid`] (ADR-003 d.8), which
    /// discriminates the [`DvText::Coded`] runtime case; wiring into the P11
    /// Validate framework is pending.
    pub reason: DvText,

    /// `is_pending`: `Boolean`, cardinality `1..1`.
    ///
    /// True if this attestation is outstanding; False means it has been
    /// completed.
    pub is_pending: bool,
}

impl TypeName for Attestation {
    const NAME: &'static str = TYPE_NAME;
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
    /// Working method per ADR-003 decision 8. `reason` is typed `DV_TEXT`
    /// (the wider supertype), but the invariant is an *implication* whose
    /// antecedent is `reason.generating_type.is_equal("DV_CODED_TEXT")` —
    /// so a plain (non-coded) `DV_TEXT` value satisfies it vacuously. The
    /// runtime-type test is the [`DvText::Coded`] discriminant of the closed
    /// enum (ADR-001 §4); a coded reason's `defining_code` is then checked
    /// against the openEHR "attestation reason" group.
    pub fn is_reason_valid(&self, terminology: &TerminologyService) -> bool {
        match &self.reason {
            // Antecedent true: the reason is a DV_CODED_TEXT, so its code
            // must be in the "attestation reason" group.
            DvText::Coded(coded) => {
                let defining_code = &coded.defining_code;
                terminology
                    .terminology(OpenehrTerminologyGroupIdentifiers::TERMINOLOGY_ID_OPENEHR)
                    .is_some_and(|access| {
                        access.has_code_for_group_id(
                            OpenehrTerminologyGroupIdentifiers::GROUP_ID_ATTESTATION_REASON,
                            &TerminologyCode::new(
                                defining_code.terminology_id.value(),
                                defining_code.code_string.clone(),
                            ),
                        )
                    })
            }
            // Antecedent false: a plain DV_TEXT reason is vacuously valid.
            DvText::Text { .. } => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::change_control::versioned_object::test_support::{audit, coded};
    use crate::data_types::text::dv_text::DvTextData;

    /// An attestation with the given `reason`, otherwise minimal.
    fn attestation(reason: DvText) -> Attestation {
        Attestation {
            type_tag: TypeTag::new(),
            audit_details: audit("2020-01-01T00:00:00").data,
            attested_view: None,
            proof: None,
            items: None,
            reason,
            is_pending: false,
        }
    }

    #[test]
    fn reason_valid_accepts_a_coded_attestation_reason() {
        let service = TerminologyService::bundled().expect("bundled terminology parses");
        // 240 = "signed" in the openEHR "attestation reason" group.
        let a = attestation(DvText::Coded(coded("240", "signed")));
        assert!(a.is_reason_valid(service));
    }

    #[test]
    fn reason_valid_rejects_a_bogus_coded_reason() {
        let service = TerminologyService::bundled().expect("bundled terminology parses");
        let a = attestation(DvText::Coded(coded("999999", "nonsense")));
        assert!(!a.is_reason_valid(service));
    }

    #[test]
    fn reason_valid_is_vacuously_true_for_plain_text() {
        let service = TerminologyService::bundled().expect("bundled terminology parses");
        // Antecedent (generating_type = DV_CODED_TEXT) is false for a bare
        // DV_TEXT, so the implication holds regardless of terminology.
        let a = attestation(DvText::Text {
            type_tag: TypeTag::new(),
            data: DvTextData {
                value: "free-text reason".to_string(),
                hyperlink: None,
                formatting: None,
                mappings: None,
                language: None,
                encoding: None,
            },
        });
        assert!(a.is_reason_valid(service));
    }

    #[test]
    fn items_valid_rejects_an_empty_but_present_list() {
        let mut a = attestation(DvText::Coded(coded("240", "signed")));
        assert!(a.are_items_valid()); // None → valid
        a.items = Some(Vec::new());
        assert!(!a.are_items_valid()); // Some(empty) → invalid
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.generic — docs/research/spec-cache/RM-1.1.0/uml_classes/attestation.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master04-generic_package.adoc §Attestation / uml_classes/attestation.adoc §ATTESTATION Class
//   confidence: high
//   todos: 0
//   note: Both invariants now working methods (ADR-003 d.8) with spec-derived tests: Items_valid a self-contained boolean; Reason_valid is the conditional DV_CODED_TEXT case — a DvText::Coded reason is checked against the openEHR "attestation reason" group via &TerminologyService, a plain DvText::Text reason is vacuously valid (antecedent false). Only remaining deferral is P11 Validate-framework wiring. Digital-signature generation (openPGP/RFC 4880) is prose-only with no function signature to transcribe — proof stays a plain String.
// ─────────────────────────────────────────────
