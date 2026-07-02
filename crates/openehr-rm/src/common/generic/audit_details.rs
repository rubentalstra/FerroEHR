//! `AUDIT_DETAILS` — the set of attributes documenting the committal of an
//! information item.
//!
//! openEHR class: `AUDIT_DETAILS` (concrete), package `common.generic`.
//!
//! The set of attributes required to document the committal of an
//! information item to a repository.
//!
//! Three classes are provided to represent audit information. The first,
//! `AUDIT_DETAILS`, expresses the details that would be captured about a
//! user when committing some information to a repository of some kind,
//! which may be version controlled. It records the `system_id`,
//! `committer`, `time_committed`, `change_type` and an optional
//! `description`.
//!
//! The `system_id` attribute is used to record the identifier of the
//! logical EHR repository to which the data containing the audit are
//! committed.
//!
//! Committer is recorded using a `PARTY_PROXY`, allowing for `PARTY_SELF`
//! to be used when the committer is the record subject, and for other
//! identifying information to be included for other users, expressed
//! using `PARTY_IDENTIFIED`.
//!
//! [`super::attestation::Attestation`] is modelled as a subtype of
//! `AUDIT_DETAILS` (embedding this struct per ADR-001 §3), meaning it is
//! logically a kind of audit, with additional information pertinent to
//! the act of signing.

// TODO(port): `DV_DATE_TIME`, `DV_CODED_TEXT`, `DV_TEXT` are RM 1.1.0
// `data_types.date_time`/`data_types.text`, transcribed by a sibling agent
// in this same phase but not yet landed in this worktree.
// Forward-references to their eventual module paths.
use crate::data_types::date_time::dv_date_time::DvDateTime;
use crate::data_types::text::dv_coded_text::DvCodedText;
use crate::data_types::text::dv_text::DvText;

use super::party_proxy::PartyProxy;

/// Canonical `_type` discriminator string for this class in serialized
/// form. Per ADR-001 refinements ("serde derives wait until P4"), a
/// `const` stands in for `#[serde(rename = ...)]` until serde lands as a
/// dependency of this crate.
pub const TYPE_NAME: &str = "AUDIT_DETAILS";

/// `AUDIT_DETAILS` declares no `Inherit` row in the spec table.
///
/// [`super::attestation::Attestation`] embeds this struct rather than
/// referencing it, per that class's `Inherit: AUDIT_DETAILS` row and
/// ADR-001 §3.
#[derive(Debug, Clone, PartialEq)]
pub struct AuditDetails {
    /// `system_id`: `String`, cardinality `1..1`.
    ///
    /// Identifier of the logical EHR system where the change was
    /// committed. This is almost always owned by the organisation legally
    /// responsible for the EHR, and is distinct from any application, or
    /// any hosting infrastructure.
    ///
    /// Invariant `System_id_valid`: `not system_id.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl; recorded here as a doc note pending the RM invariant
    /// framework (`.claude/rules/rm-transcription.md` "Invariants").
    pub system_id: String,

    /// `time_committed`: `DV_DATE_TIME`, cardinality `1..1`.
    ///
    /// Time of committal of the item.
    pub time_committed: DvDateTime,

    /// `change_type`: `DV_CODED_TEXT`, cardinality `1..1`.
    ///
    /// Type of change. Coded using the openEHR Terminology "audit change
    /// type" group.
    ///
    /// Invariant `Change_type_valid`:
    /// `terminology(Terminology_id_openehr).has_code_for_group_id(
    /// Group_id_audit_change_type, change_type.defining_code)`.
    ///
    /// TODO(port): invariant requires a live `TerminologyService`; not yet
    /// enforced. See [`AuditDetails::is_change_type_valid`].
    pub change_type: DvCodedText,

    /// `description`: `DV_TEXT`, cardinality `0..1`.
    ///
    /// Reason for committal. This may be used to qualify the value in the
    /// `change_type` field. For example, if the change affects only the
    /// EHR directory, this field might be used to indicate 'Folder
    /// "episode 2018-02-16" added' or similar.
    pub description: Option<DvText>,

    /// `committer`: `PARTY_PROXY`, cardinality `1..1`.
    ///
    /// Identity and optional reference into identity management service,
    /// of user who committed the item.
    pub committer: PartyProxy,
}

impl AuditDetails {
    /// Invariant `System_id_valid`: `not system_id.is_empty`.
    ///
    /// TODO(port): not yet wired into a constructor or the RM `Validate`
    /// framework; this method lets a future `Validate` impl call the check
    /// directly once that framework lands.
    pub fn is_system_id_valid(&self) -> bool {
        !self.system_id.is_empty()
    }

    /// Invariant `Change_type_valid`:
    /// `terminology(Terminology_id_openehr).has_code_for_group_id(
    /// Group_id_audit_change_type, change_type.defining_code)`.
    ///
    /// TODO(port): requires a live `TerminologyService` to check
    /// `change_type.defining_code` against the "audit change type"
    /// openEHR Terminology group; left as `todo!()` rather than a bare
    /// boolean stub, since this invariant cannot be evaluated without
    /// external service state.
    pub fn is_change_type_valid(
        &self,
        _terminology: &openehr_terminology::TerminologyService,
    ) -> bool {
        todo!(
            "AuditDetails::is_change_type_valid: needs TerminologyService.has_code_for_group_id against Group_id_audit_change_type"
        )
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.generic — docs/research/spec-cache/RM-1.1.0/uml_classes/audit_details.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master04-generic_package.adoc §Audit Information / uml_classes/audit_details.adoc §AUDIT_DETAILS Class
//   confidence: high
//   todos: 2
//   note: System_id_valid recorded as a self-contained boolean check; Change_type_valid left todo!()-bodied (needs live TerminologyService). Forward-refs DvDateTime, DvCodedText, DvText (data_types, sibling-agent territory, not yet landed). This struct is the embedded parent for Attestation per its Inherit row.
// ─────────────────────────────────────────────
