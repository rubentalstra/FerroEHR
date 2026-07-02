//! `REVISION_HISTORY_ITEM` — one entry in a revision history.
//!
//! openEHR class: `REVISION_HISTORY_ITEM` (concrete), package
//! `common.generic`.
//!
//! An entry in a revision history, corresponding to a version from a
//! versioned container. Consists of `AUDIT_DETAILS` instances with the
//! revision identifier of the revision to which the `AUDIT_DETAILS`
//! instance belongs.
use openehr_base::identification::object_version_id::ObjectVersionId;

use super::audit_details::AuditDetails;

/// Canonical `_type` discriminator string for this class in serialized
/// form. Per ADR-001 refinements ("serde derives wait until P4"), a
/// `const` stands in for `#[serde(rename = ...)]` until serde lands as a
/// dependency of this crate.
pub const TYPE_NAME: &str = "REVISION_HISTORY_ITEM";

/// `REVISION_HISTORY_ITEM` declares no `Inherit` row in the spec table.
#[derive(Debug, Clone, PartialEq)]
pub struct RevisionHistoryItem {
    /// `version_id`: `OBJECT_VERSION_ID`, cardinality `1..1`.
    ///
    /// Version identifier for this revision.
    pub version_id: ObjectVersionId,

    /// `audits`: `List<AUDIT_DETAILS>`, cardinality `1..1`.
    ///
    /// The audits for this revision; there will always be at least one
    /// commit audit (which may itself be an `ATTESTATION`), there may
    /// also be further attestations.
    ///
    /// PORT NOTE: the spec table types this attribute `List<AUDIT_DETAILS>`
    /// even though its own description says an entry "may itself be an
    /// `ATTESTATION`" — since `ATTESTATION inherits AUDIT_DETAILS`
    /// (embedding it, per ADR-001 §3, in `attestation.rs`), a
    /// `Vec<AuditDetails>` cannot literally hold an `Attestation` value
    /// without either up-casting (losing the `Attestation`-specific
    /// fields) or widening the element type to an enum the spec does not
    /// itself declare here. Transcribed literally as `Vec<AuditDetails>`
    /// per the table's stated type; a caller needing to represent a mixed
    /// list of plain audits and attestations must currently choose the
    /// `Attestation` element and access its embedded `audit_details` field
    /// when only the base information is needed — capturing an
    /// `Attestation` losslessly inside this list is not yet representable
    /// and is flagged here rather than silently narrowed.
    pub audits: Vec<AuditDetails>,
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.generic — docs/research/spec-cache/RM-1.1.0/uml_classes/revision_history_item.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master04-generic_package.adoc §Revision History / uml_classes/revision_history_item.adoc §REVISION_HISTORY_ITEM Class
//   confidence: medium
//   todos: 0
//   note: audits: Vec<AuditDetails> transcribed literally per the table's stated element type, even though the class description says an entry "may itself be an ATTESTATION" — representing a lossless mix of AuditDetails/Attestation in one Vec is not possible without an enum the spec table does not declare at this attribute; flagged in the field doc rather than silently introducing one. No invariants published for this class.
// ─────────────────────────────────────────────
