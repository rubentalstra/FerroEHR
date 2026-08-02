//! `AUDIT_DETAILS` — the commit provenance every version and contribution
//! carries, and the `audit_change_type` terminology codes it is
//! coded from.
//!
//! Spec: RM common `master04-generic_package.adoc` §Audit Details
//! (`AUDIT_DETAILS` = `system_id`, `committer: PARTY_PROXY`, `time_committed`,
//! `change_type`, `description?`) and RM common
//! `master06-change_control_package.adoc` §Contributions (the change-type
//! group). The wire form of the change type is its **numeric group code**
//! (`AUDIT_DETAILS.Change_type_valid` requires `change_type.defining_code` to
//! be a member of the openEHR `audit_change_type` group, e.g. `"249"`, not the
//! rubric `"creation"`); the numeric code is stored and the human rubric is
//! resolved from the `openehr-term` bundle at the render edge, never a
//! hardcoded rubric.

use openehr_base::prelude::TerminologyId;
use openehr_rm::prelude::{
    Attestation, AuditDetails, AuditDetailsData, CodePhrase, DvCodedText, DvDateTime, DvText,
    DvTextData, PartyProxy,
};
use openehr_term::bundle::openehr;
use serde_json::Value;

use crate::service::error::ServiceError;
use crate::versioning::attestation::AttestationParts;

/// The openEHR internal terminology id (`Terminology_id_openehr`).
pub(crate) const OPENEHR: &str = "openehr";

/// The `audit_change_type` openEHR terminology group id.
const AUDIT_CHANGE_TYPE: &str = "audit_change_type";

/// The COMPLETE `audit_change_type` group (TERM 3.1.0
/// `openehr_terminology.xml` group `audit_change_type`; RM common master06
/// §Contributions), one named constant per member. Membership checks still go
/// through [`change_type_code`] (the bundle is the authority); the constants
/// exist so no write path or compatibility rule ever spells a code as a bare
/// string literal.
pub(crate) mod change_type {
    /// `249|creation|` — first version of a versioned object.
    pub(crate) const CREATION: &str = "249";
    /// `250|amendment|` — a correcting content change to an existing object
    /// (a legal client choice on an update, alongside `251`).
    pub(crate) const AMENDMENT: &str = "250";
    /// `251|modification|` — a content change to an existing object.
    pub(crate) const MODIFICATION: &str = "251";
    /// `252|synthesis|` — a version synthesized from other sources.
    pub(crate) const SYNTHESIS: &str = "252";
    /// `253|unknown|` — provenance of the change is unknown.
    pub(crate) const UNKNOWN: &str = "253";
    /// `523|deleted|` — a logical deletion.
    pub(crate) const DELETED: &str = "523";
    /// `666|attestation|` — attaches an `ATTESTATION` to an existing
    /// `ORIGINAL_VERSION` (adds no new version — RM common master06
    /// §Contributions; the contribution path's `Action::Attest`).
    pub(crate) const ATTESTATION: &str = "666";
    /// `816|restoration|` — restores earlier content as a new version.
    pub(crate) const RESTORATION: &str = "816";
    /// `817|format conversion|` — the same content re-encoded in another
    /// format, committed as a new version.
    pub(crate) const FORMAT_CONVERSION: &str = "817";
}

/// Resolve an inbound audit `change_type` token — either a numeric group code
/// (`"249"`) or a rubric (`"creation"`) — to its canonical numeric group code.
/// `None` when the token is not a member of the `audit_change_type` group
/// (RM common master04 `AUDIT_DETAILS.Change_type_valid` — callers must reject,
/// never store, an out-of-group change type).
pub(crate) fn change_type_code(token: &str) -> Option<String> {
    let t = openehr();
    if t.is_valid_audit_change_type(token) {
        return Some(token.to_owned());
    }
    t.concepts_in_group(AUDIT_CHANGE_TYPE)
        .iter()
        .find(|c| c.rubric.eq_ignore_ascii_case(token))
        .map(|c| c.id.clone())
}

/// The effective `change_type` of a direct commit: the caller's supplied
/// token when it names a legal, operation-compatible `audit_change_type`
/// group member; the operation's default when the caller supplied none
/// (empty) or restated the default.
///
/// ITS-REST overview §"openehr-version and openehr-audit-details" lists
/// `change_type` among the attributes clients MAY supply and requires that
/// "whatever is provided it MUST be merged" — so a legal divergent value
/// (e.g. `250|amendment|` on an update) is honoured, not overwritten.
/// Operation compatibility mirrors the CONTRIBUTION path
/// (`versioning/contribution.rs` `classify`, RM common master06
/// §Contributions): a direct create commits a first version (`249` only), a
/// direct delete a logical deletion (`523` only), and a direct update a
/// content-carrying new version of an existing object (any group code except
/// `249`/`523`/`666`).
///
/// # Errors
/// [`ServiceError::Unprocessable`] — the token is not a member of the
/// `audit_change_type` group (`AUDIT_DETAILS.Change_type_valid`);
/// [`ServiceError::BadRequest`] — a group code that does not match the
/// operation (a change-control mismatch, not content validation — the same
/// split the CONTRIBUTION path draws).
fn merged_change_type(supplied: &str, operation: &str) -> Result<String, ServiceError> {
    let token = supplied.trim();
    if token.is_empty() || token == operation {
        return Ok(operation.to_owned());
    }
    let code = change_type_code(token).ok_or_else(|| {
        ServiceError::Unprocessable(format!(
            "change_type {token:?} is not a code in the openEHR audit_change_type group \
             (AUDIT_DETAILS.Change_type_valid)"
        ))
    })?;
    let compatible = match operation {
        change_type::CREATION => code == change_type::CREATION,
        change_type::DELETED => code == change_type::DELETED,
        // A direct update: a content-carrying new version of an existing
        // object — the full modification family, never
        // creation/deleted/attestation.
        _ => matches!(
            code.as_str(),
            change_type::AMENDMENT
                | change_type::MODIFICATION
                | change_type::SYNTHESIS
                | change_type::UNKNOWN
                | change_type::RESTORATION
                | change_type::FORMAT_CONVERSION
        ),
    };
    if compatible {
        Ok(code)
    } else {
        Err(ServiceError::BadRequest(format!(
            "change_type {code} does not match the operation (its change type is \
             {operation}) — the modification type must match the operation \
             (RM change_control §Contributions; ITS-REST overview \
             §\"openehr-version and openehr-audit-details\")"
        )))
    }
}

/// The rubric (English display text) for an `audit_change_type` code; falls
/// back to the code itself if the code is unknown to the bundle. The single
/// code→rubric mapping every presentation surface uses (the by-uid
/// CONTRIBUTION rendering and the contribution-list extension) — consumers
/// never hardcode a rubric table (RM common master04 §Audit Details; the
/// `audit_change_type` group, TERM 3.1.0).
pub(crate) fn change_type_rubric(code: &str) -> String {
    openehr()
        .rubric(AUDIT_CHANGE_TYPE, code, "en")
        .unwrap_or(code)
        .to_owned()
}

/// What an audit row records about a committed change — the `AUDIT_DETAILS`
/// attributes the service owns at write time (RM common master04 §Audit
/// Details), plus the `ATTESTATION`-declared attributes when the commit audit
/// is of that subtype (master06 §Attestation: "`ORIGINAL_VERSION._commit_audit_`
/// is of type `ATTESTATION` rather than `AUDIT_DETAILS`").
///
/// This is the ONE commit-audit carrier: the write paths build it, storage
/// persists it ([`Self::row`]), and the read paths rebuild the served
/// `AUDIT_DETAILS` from it ([`Self::typed`] / [`Self::canonical`]) — so what a
/// client commits is what a reader gets back.
#[derive(Debug, Clone)]
pub(crate) struct AuditInput {
    /// `AUDIT_DETAILS.system_id` (1..1, non-empty — `System_id_valid`).
    pub(crate) system_id: String,
    /// The numeric `audit_change_type` group code (`249`/`251`/`523`/…) — never
    /// a rubric string (`AUDIT_DETAILS.Change_type_valid`).
    pub(crate) change_type: String,
    /// `AUDIT_DETAILS.description` (0..1) as its canonical `DV_TEXT` fragment —
    /// whole, because the attribute's `DV_CODED_TEXT` subtype carries a
    /// `defining_code` a bare string would discard (RM common
    /// `UML/classes/org.openehr.rm.common.audit_details.adoc` §Attributes).
    pub(crate) description: Option<Value>,
    /// Canonical `PARTY_PROXY` of the committer (`AUDIT_DETAILS.committer`, 1..1).
    pub(crate) committer: Value,
    /// The canonical fragment of the `ATTESTATION`-declared attributes
    /// ([`crate::versioning::attestation::AttestationParts::fragment`]) when
    /// this commit audit is an `ATTESTATION`, else `None`. `ATTESTATION` is the
    /// only `AUDIT_DETAILS` subtype RM 1.2.0 declares, so presence IS the
    /// concrete class.
    pub(crate) attestation: Option<Value>,
}

impl AuditInput {
    /// Build the commit audit from the caller's `UPDATE_VERSION.audit`
    /// envelope, merged with the server rules (ITS-REST overview
    /// §"openehr-version and openehr-audit-details": `change_type` is the
    /// FIRST attribute the clients "MAY supply values for", and "whatever is
    /// provided it MUST be merged"; RM common master06 §Committal m4
    /// defaults):
    ///
    /// - `change_type` — the caller's, when supplied: honoured verbatim after
    ///   [`merged_change_type`] validates it against the `audit_change_type`
    ///   group and the operation (an update legally carries `250|amendment|`
    ///   as well as `251|modification|` — the wire HAS legal divergent
    ///   values). Absent/empty → the operation's default;
    /// - `description` — the caller's when supplied, else the server default;
    /// - `committer` — the caller's `PARTY_PROXY` (the protocol adapter has
    ///   already defaulted an absent committer to the authenticated
    ///   principal / system identity);
    /// - `system_id` — the caller's when supplied, else this server's.
    ///
    /// # Errors
    /// [`ServiceError::Unprocessable`] for a `change_type` outside the
    /// `audit_change_type` group (`AUDIT_DETAILS.Change_type_valid`);
    /// [`ServiceError::BadRequest`] for a group code that contradicts the
    /// operation (mirroring the CONTRIBUTION path's change-control mismatch,
    /// `versioning/contribution.rs`).
    pub(crate) fn from_update(
        update: &crate::service::version_update::UpdateAudit,
        operation_change_type: &str,
        default_description: &str,
        fallback_system_id: &str,
    ) -> Result<Self, ServiceError> {
        let change_type =
            merged_change_type(&update.change_type.code_string, operation_change_type)?;
        // The native codec serializes a PARTY_PROXY infallibly.
        let committer = openehr_its::json::to_canonical_value(&update.committer);
        Ok(Self {
            system_id: update
                .system_id
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| fallback_system_id.to_owned()),
            change_type,
            description: Some(description_fragment(
                update
                    .description
                    .as_deref()
                    .filter(|d| !d.is_empty())
                    .unwrap_or(default_description),
            )),
            committer,
            // NOTE: an `UPDATE_AUDIT` cannot express an `ATTESTATION` commit
            // audit, and the routes that build one need no such expression: on
            // a direct `PUT`/`POST`/`DELETE` the committal metadata comes from
            // the `openehr-audit-details` header, whose released attribute list
            // is exactly `change_type`, `description`, `committer`, `system_id`
            // (ITS-REST overview `Requests_and_responses.md` §"openehr-version
            // and openehr-audit-details"). An `ATTESTATION` commit audit is a
            // CONTRIBUTION-body shape, parsed at the native commit seam
            // (`versioning::contribution`).
            attestation: None,
        })
    }

    /// The commit audit of a stored version-metadata row.
    pub(crate) fn from_meta(meta: &crate::storage::version_repo::meta::VersionMeta) -> Self {
        Self {
            system_id: meta.audit_system_id.clone(),
            change_type: meta.audit_change_type.clone(),
            description: meta.audit_description.clone(),
            committer: meta.audit_committer.clone(),
            attestation: meta.audit_attestation.clone(),
        }
    }

    /// The borrowed storage row shape ([`crate::storage::version_repo::commit::AuditRow`])
    /// this audit persists as.
    pub(crate) fn row(&self) -> crate::storage::version_repo::commit::AuditRow<'_> {
        crate::storage::version_repo::commit::AuditRow {
            system_id: &self.system_id,
            change_type: &self.change_type,
            description: self.description.as_ref(),
            committer: &self.committer,
            attestation: self.attestation.as_ref(),
        }
    }

    /// This commit audit as its typed RM value at the recorded commit instant:
    /// an [`AuditDetails::Attestation`] when the audit carries the
    /// `ATTESTATION`-declared attributes (RM common master06 §Attestation),
    /// else a plain [`AuditDetails::AuditDetails`]. `change_type` is the stored
    /// numeric `audit_change_type` group code; the `DV_CODED_TEXT` carries it as
    /// `defining_code.code_string` (`AUDIT_DETAILS.Change_type_valid`) with the
    /// group rubric — resolved from the `openehr-term` bundle — as its `value`.
    ///
    /// # Errors
    /// The [`party_proxy`] rejection when `committer` is not a canonical
    /// `PARTY_PROXY`; [`ServiceError::Unprocessable`] when the stored
    /// `description` is not a canonical `DV_TEXT` or the stored attestation
    /// attributes do not decode
    /// ([`crate::versioning::attestation::AttestationParts::decode`]).
    pub(crate) fn typed(
        &self,
        time_committed: &jiff::Timestamp,
    ) -> Result<AuditDetails, ServiceError> {
        let system_id = self.system_id.clone();
        let time_committed = dv_date_time(time_committed);
        let change_type =
            openehr_coded_text(&self.change_type, change_type_rubric(&self.change_type));
        let description = self
            .description
            .as_ref()
            .map(decode_description)
            .transpose()?;
        let committer = party_proxy(&self.committer)?;
        match &self.attestation {
            None => Ok(AuditDetails::AuditDetails(AuditDetailsData {
                system_id,
                time_committed,
                change_type,
                description,
                committer,
            })),
            Some(fragment) => {
                let parts = AttestationParts::decode(fragment)?;
                Ok(AuditDetails::Attestation(Attestation {
                    system_id,
                    time_committed,
                    change_type,
                    description,
                    committer,
                    attested_view: parts.attested_view,
                    proof: parts.proof,
                    items: openehr_base::containers::present(parts.items),
                    reason: parts.reason,
                    is_pending: parts.is_pending,
                }))
            }
        }
    }

    /// The canonical-JSON form of [`Self::typed`], serialized through the
    /// native codec so the wire body carries `_type` first — `ATTESTATION` when
    /// the version was committed with one — and the BMM's own attribute order.
    ///
    /// # Errors
    /// The [`Self::typed`] rejections.
    pub(crate) fn canonical(
        &self,
        time_committed: &jiff::Timestamp,
    ) -> Result<Value, ServiceError> {
        Ok(openehr_its::json::to_canonical_value(
            &self.typed(time_committed)?,
        ))
    }
}

/// A plain-string `AUDIT_DETAILS.description` as the canonical `DV_TEXT`
/// fragment stored for it — the shape a server default, an
/// `openehr-audit-details` header value, or an SM `String` description takes
/// (RM common master04 §Audit Details: `description` is a `DV_TEXT`).
pub(crate) fn description_fragment(value: &str) -> Value {
    openehr_its::json::to_canonical_value(&dv_text(value))
}

/// Decode a stored/wire `AUDIT_DETAILS.description` fragment into its RM type,
/// so a `DV_CODED_TEXT` description returns as one.
///
/// # Errors
/// [`ServiceError::Unprocessable`] when the value is not a canonical `DV_TEXT`
/// (RM common `UML/classes/org.openehr.rm.common.audit_details.adoc`
/// §Attributes).
pub(crate) fn decode_description(fragment: &Value) -> Result<DvText, ServiceError> {
    openehr_its::json::from_canonical_value::<DvText>(fragment).map_err(|e| {
        ServiceError::Unprocessable(format!(
            "AUDIT_DETAILS.description is not a canonical DV_TEXT: {e}"
        ))
    })
}

/// An openEHR-terminology `DV_CODED_TEXT`: the group `code` as
/// `defining_code.code_string` under the `openehr` terminology, with `rubric`
/// as its displayable `value`. The one shared constructor for the coded texts
/// this module and its callers mint (`AUDIT_DETAILS.change_type`,
/// `ORIGINAL_VERSION.lifecycle_state`), so no site rebuilds the `CODE_PHRASE`
/// by hand.
pub(crate) fn openehr_coded_text(code: &str, rubric: String) -> DvCodedText {
    DvCodedText {
        value: rubric,
        hyperlink: None,
        formatting: None,
        mappings: openehr_base::containers::present(Vec::new()),
        language: None,
        encoding: None,
        defining_code: CodePhrase {
            terminology_id: TerminologyId {
                value: OPENEHR.to_owned(),
            },
            code_string: code.to_owned(),
            preferred_term: None,
        },
    }
}

/// A `DV_DATE_TIME` carrying an instant's ISO 8601 form — the shape
/// `AUDIT_DETAILS.time_committed` (1..1) takes.
pub(crate) fn dv_date_time(instant: &jiff::Timestamp) -> DvDateTime {
    DvDateTime {
        normal_status: None,
        normal_range: None,
        other_reference_ranges: openehr_base::containers::present(Vec::new()),
        magnitude_status: None,
        accuracy: None,
        value: instant.to_string(),
    }
}

/// A plain `DV_TEXT` — the shape `AUDIT_DETAILS.description` (0..1) takes when
/// the caller supplied only a string.
pub(crate) fn dv_text(value: &str) -> DvText {
    DvText::DvText(DvTextData {
        value: value.to_owned(),
        hyperlink: None,
        formatting: None,
        mappings: openehr_base::containers::present(Vec::new()),
        language: None,
        encoding: None,
    })
}

/// Decode a stored/assembled canonical `PARTY_PROXY` value into its RM type.
///
/// The committer travels between the service layer and storage as canonical
/// JSON (an `audit.committer` jsonb column), so the typed builders below must
/// read it back to place it in an `AUDIT_DETAILS`.
///
/// # Errors
/// [`ServiceError::Unprocessable`] when the value is not a canonical
/// `PARTY_PROXY` (`AUDIT_DETAILS.committer` is 1..1 — RM common
/// `UML/classes/org.openehr.rm.common.audit_details.adoc` §Attributes).
pub(crate) fn party_proxy(committer: &Value) -> Result<PartyProxy, ServiceError> {
    openehr_its::json::from_canonical_value::<PartyProxy>(committer).map_err(|e| {
        ServiceError::Unprocessable(format!(
            "AUDIT_DETAILS.committer is not a canonical PARTY_PROXY: {e}"
        ))
    })
}

/// Validate a client-supplied commit `AUDIT_DETAILS`' non-terminology RM
/// invariants before it is persisted (a CONTRIBUTION audit or a version
/// `commit_audit`).
///
/// `change_type` is validated separately ([`change_type_code`]).
///
/// # Errors
/// [`ServiceError::Unprocessable`] when either enforced invariant fails:
///
/// - `AUDIT_DETAILS.System_id_valid`: `not system_id.is_empty` (RM
///   `UML/classes/org.openehr.rm.common.audit_details.adoc` §Invariants).
///   Without this guard an empty client-supplied
///   `system_id` reaches the DB `System_id_valid` CHECK and surfaces as a
///   `500` — a validation failure must be `422`, not an internal error.
/// - the committer `PARTY_PROXY`'s own `PARTY_IDENTIFIED`/`PARTY_RELATED`
///   invariants `Basic_validity` + `Name_valid` (RM
///   `UML/classes/org.openehr.rm.common.party_identified.adoc` §Invariants),
///   plus `Relationship_valid` for `PARTY_RELATED`
///   (`…org.openehr.rm.common.party_related.adoc` §Invariants). A PARTY that appears
///   as *content* is validated by the RM-invariant pass, but the audit
///   committer is stored verbatim, so it is checked here.
pub(crate) fn validate_commit_audit(audit: &AuditInput) -> Result<(), ServiceError> {
    if audit.system_id.is_empty() {
        return Err(ServiceError::Unprocessable(
            "AUDIT_DETAILS.system_id is mandatory and non-void \
             (AUDIT_DETAILS.System_id_valid)"
                .to_owned(),
        ));
    }
    validate_committer(&audit.committer)
}

/// Run the committer `PARTY_PROXY`'s OWN RM class invariants + terminology
/// bindings — `PARTY_IDENTIFIED.Basic_validity` / `.Name_valid`
/// (`RM/docs/UML/classes/org.openehr.rm.common.party_identified.adoc`
/// §Invariants), `PARTY_RELATED.Relationship_valid`
/// (`…org.openehr.rm.common.party_related.adoc` §Invariants), and the
/// structural conformance of the concrete `PARTY_PROXY` subtype.
///
/// The rules are NOT restated here: the value goes through the unified
/// dispatcher [`openehr_its::wire_validate::validate_rm_value`], the same one the
/// whole-instance commit pass runs on every node of a committed RM document
/// (`openehr_its::rm_instance::validate_rm_and_terminology_as`). That
/// dispatcher is what runs the generated structural check, the typed invariant
/// cores, the model-driven mandatory-container bounds and the
/// terminology-backed invariants (all defined in `openehr_rm::validate`).
///
/// It has to be invoked EXPLICITLY here because a commit audit is not part of
/// any committed RM document: the `AUDIT_DETAILS` is written to its own row and
/// never walked by the content pass, so without this call the committer would
/// be the one `PARTY_PROXY` in the system that reaches storage unvalidated.
///
/// `AUDIT_DETAILS.committer` is typed `PARTY_PROXY` — an ABSTRACT class — so
/// canonical JSON must tag it; an untagged value carries no class to judge and
/// is left to the structural decode at the storage boundary, exactly as the
/// dispatcher treats an untagged node anywhere else.
fn validate_committer(committer: &Value) -> Result<(), ServiceError> {
    let mut violations = Vec::new();
    openehr_its::wire_validate::validate_rm_value(committer, &mut violations);
    if violations.is_empty() {
        return Ok(());
    }
    let detail = violations
        .iter()
        .map(|v| v.message.as_str())
        .collect::<Vec<_>>()
        .join("; ");
    Err(ServiceError::Unprocessable(format!(
        "AUDIT_DETAILS.committer is not a valid PARTY_PROXY: {detail}"
    )))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// ITS-REST overview §"openehr-version and openehr-audit-details": a
    /// caller-supplied `change_type` "MUST be merged" — legal divergent
    /// values are honoured; out-of-group tokens are 422; operation
    /// mismatches are 400 (mirroring the CONTRIBUTION path).
    #[test]
    fn merged_change_type_honours_legal_client_codes() {
        // Absent/empty and restated defaults resolve to the operation's code.
        assert_eq!(
            merged_change_type("", change_type::MODIFICATION).unwrap(),
            change_type::MODIFICATION
        );
        assert_eq!(
            merged_change_type(change_type::MODIFICATION, change_type::MODIFICATION).unwrap(),
            change_type::MODIFICATION
        );
        // 250|amendment| is a legal divergent value on an update.
        assert_eq!(
            merged_change_type("250", change_type::MODIFICATION).unwrap(),
            "250"
        );
        // Rubric tokens resolve to their numeric group code.
        assert_eq!(
            merged_change_type("amendment", change_type::MODIFICATION).unwrap(),
            "250"
        );
        // Creates carry 249 only; deletes carry 523 only.
        assert_eq!(
            merged_change_type(change_type::CREATION, change_type::CREATION).unwrap(),
            change_type::CREATION
        );
        assert_eq!(
            merged_change_type(change_type::DELETED, change_type::DELETED).unwrap(),
            change_type::DELETED
        );
    }

    #[test]
    fn merged_change_type_rejects_mismatch_as_bad_request() {
        for (supplied, operation) in [
            (change_type::CREATION, change_type::MODIFICATION),
            (change_type::DELETED, change_type::MODIFICATION),
            (change_type::ATTESTATION, change_type::MODIFICATION),
            ("250", change_type::CREATION),
            ("250", change_type::DELETED),
            (change_type::CREATION, change_type::DELETED),
        ] {
            let err = merged_change_type(supplied, operation).unwrap_err();
            assert!(
                matches!(err, ServiceError::BadRequest(_)),
                "{supplied} on op {operation}: {err:?}"
            );
        }
    }

    #[test]
    fn merged_change_type_rejects_out_of_group_as_unprocessable() {
        for token in ["999", "banana", "creation-x"] {
            let err = merged_change_type(token, change_type::MODIFICATION).unwrap_err();
            assert!(
                matches!(err, ServiceError::Unprocessable(_)),
                "{token}: {err:?}"
            );
        }
    }

    /// Every named constant is a real group member, and the constants cover
    /// the COMPLETE `audit_change_type` group — a code added to the TERM
    /// bundle without a constant here fails this test.
    #[test]
    fn change_type_constants_are_the_complete_group() {
        let all = [
            change_type::CREATION,
            change_type::AMENDMENT,
            change_type::MODIFICATION,
            change_type::SYNTHESIS,
            change_type::UNKNOWN,
            change_type::DELETED,
            change_type::ATTESTATION,
            change_type::RESTORATION,
            change_type::FORMAT_CONVERSION,
        ];
        let t = openehr();
        for code in all {
            assert!(t.is_valid_audit_change_type(code), "code {code}");
            // code_string must be numeric (AUDIT_DETAILS.Change_type_valid).
            assert!(code.chars().all(|c| c.is_ascii_digit()));
        }
        let mut group: Vec<String> = t
            .concepts_in_group(AUDIT_CHANGE_TYPE)
            .iter()
            .map(|c| c.id.clone())
            .collect();
        group.sort();
        let mut named: Vec<String> = all.iter().map(|c| (*c).to_owned()).collect();
        named.sort();
        assert_eq!(group, named, "constants must mirror the full TERM group");
    }

    #[test]
    fn change_type_code_accepts_code_or_rubric_and_rejects_non_members() {
        assert_eq!(change_type_code("249").as_deref(), Some("249"));
        assert_eq!(change_type_code("creation").as_deref(), Some("249"));
        assert_eq!(change_type_code("Deleted").as_deref(), Some("523"));
        assert_eq!(change_type_code("amendment").as_deref(), Some("250"));
        assert_eq!(change_type_code("synthesis").as_deref(), Some("252"));
        assert_eq!(change_type_code("unknown").as_deref(), Some("253"));
        assert_eq!(change_type_code("666").as_deref(), Some("666"));
        // Out-of-group tokens are rejected, not passed through
        // (AUDIT_DETAILS.Change_type_valid).
        assert_eq!(change_type_code("not-a-change-type"), None);
        assert_eq!(change_type_code("532"), None); // lifecycle code, wrong group
    }

    #[test]
    fn rubric_round_trips() {
        assert_eq!(change_type_rubric(change_type::CREATION), "creation");
        assert_eq!(
            change_type_rubric(change_type::MODIFICATION),
            "modification"
        );
        assert_eq!(change_type_rubric(change_type::DELETED), "deleted");
    }

    fn audit_input(system_id: &str, committer: Value) -> AuditInput {
        AuditInput {
            system_id: system_id.to_owned(),
            change_type: change_type::CREATION.to_owned(),
            description: None,
            committer,
            attestation: None,
        }
    }

    #[test]
    fn commit_audit_rejects_empty_system_id() {
        let audit = audit_input(
            "",
            json!({ "_type": "PARTY_IDENTIFIED", "name": "Dr Jones" }),
        );
        match validate_commit_audit(&audit) {
            Err(ServiceError::Unprocessable(msg)) => assert!(
                msg.contains("System_id_valid"),
                "should cite System_id_valid, got {msg}"
            ),
            other => panic!("expected Unprocessable(System_id_valid), got {other:?}"),
        }
    }

    #[test]
    fn commit_audit_rejects_committer_without_identity() {
        let audit = audit_input("ferroehr.local", json!({ "_type": "PARTY_IDENTIFIED" }));
        match validate_commit_audit(&audit) {
            Err(ServiceError::Unprocessable(msg)) => assert!(
                msg.contains("Basic_validity"),
                "should cite Basic_validity, got {msg}"
            ),
            other => panic!("expected Unprocessable(Basic_validity), got {other:?}"),
        }
    }

    #[test]
    fn commit_audit_rejects_empty_committer_name() {
        let audit = audit_input(
            "ferroehr.local",
            json!({ "_type": "PARTY_IDENTIFIED", "name": "" }),
        );
        match validate_commit_audit(&audit) {
            Err(ServiceError::Unprocessable(msg)) => assert!(
                msg.contains("Name_valid"),
                "should cite Name_valid, got {msg}"
            ),
            other => panic!("expected Unprocessable(Name_valid), got {other:?}"),
        }
    }

    #[test]
    fn commit_audit_party_related_relationship_is_enforced() {
        // Group member 10 = "mother"; a non-member / uncoded / missing
        // relationship is a 422 (Relationship_valid, party proxies).
        let related = |relationship: Value| {
            let mut c = json!({ "_type": "PARTY_RELATED", "name": "Mum" });
            if !relationship.is_null() {
                c.as_object_mut()
                    .unwrap()
                    .insert("relationship".into(), relationship);
            }
            audit_input("sys", c)
        };
        match validate_commit_audit(&related(Value::Null)) {
            Err(ServiceError::Unprocessable(msg)) => {
                assert!(msg.contains("relationship"), "got {msg}");
            }
            other => panic!("missing relationship must be Unprocessable, got {other:?}"),
        }
        assert!(
            validate_commit_audit(&related(json!({ "_type": "DV_TEXT", "value": "mother" })))
                .is_err(),
            "an uncoded relationship must be rejected (1..1 DV_CODED_TEXT)"
        );
        let bad = related(json!({
            "_type": "DV_CODED_TEXT", "value": "colleague",
            "defining_code": { "_type": "CODE_PHRASE", "code_string": "99999",
                               "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" } }
        }));
        // The refusal is produced by the generated `PARTY_RELATED` core, whose
        // violation message names the invariant, not the rejected code.
        match validate_commit_audit(&bad) {
            Err(ServiceError::Unprocessable(msg)) => {
                assert!(msg.contains("Relationship_valid"), "got {msg}");
            }
            other => panic!("non-member relationship code must be 422, got {other:?}"),
        }
        validate_commit_audit(&related(json!({
            "_type": "DV_CODED_TEXT", "value": "mother",
            "defining_code": { "_type": "CODE_PHRASE", "code_string": "10",
                               "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" } }
        })))
        .expect("subject_relationship group member accepted");
    }

    #[test]
    fn commit_audit_accepts_valid_committers() {
        validate_commit_audit(&audit_input(
            "sys",
            json!({ "_type": "PARTY_IDENTIFIED", "name": "Dr Jones" }),
        ))
        .expect("named committer");
        validate_commit_audit(&audit_input(
            "sys",
            json!({ "_type": "PARTY_IDENTIFIED", "identifiers": [
                { "_type": "DV_IDENTIFIER", "id": "42", "issuer": "x", "type": "id" }
            ] }),
        ))
        .expect("identifier-only committer");
        validate_commit_audit(&audit_input("sys", json!({ "_type": "PARTY_SELF" })))
            .expect("PARTY_SELF committer");
    }
}
