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

use openehr_term::bundle::openehr;
use serde_json::{Value, json};

use crate::service::error::ServiceError;

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
/// Details).
#[derive(Debug, Clone)]
pub(crate) struct AuditInput {
    /// `AUDIT_DETAILS.system_id` (1..1, non-empty — `System_id_valid`).
    pub(crate) system_id: String,
    /// The numeric `audit_change_type` group code (`249`/`251`/`523`/…) — never
    /// a rubric string (`AUDIT_DETAILS.Change_type_valid`).
    pub(crate) change_type: String,
    /// `AUDIT_DETAILS.description` (0..1).
    pub(crate) description: Option<String>,
    /// Canonical `PARTY_PROXY` of the committer (`AUDIT_DETAILS.committer`, 1..1).
    pub(crate) committer: Value,
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
            description: Some(
                update
                    .description
                    .clone()
                    .filter(|d| !d.is_empty())
                    .unwrap_or_else(|| default_description.to_owned()),
            ),
            committer,
        })
    }

    /// The borrowed storage row shape ([`crate::storage::version_repo::commit::AuditRow`])
    /// this audit persists as.
    pub(crate) fn row(&self) -> crate::storage::version_repo::commit::AuditRow<'_> {
        crate::storage::version_repo::commit::AuditRow {
            system_id: &self.system_id,
            change_type: &self.change_type,
            description: self.description.as_deref(),
            committer: &self.committer,
        }
    }
}

/// Build an `AUDIT_DETAILS` from stored audit columns. `change_type` is the
/// numeric `audit_change_type` group code stored in the `audit` row; the
/// emitted `DV_CODED_TEXT` carries the code as `defining_code.code_string`
/// (RM common master04 `AUDIT_DETAILS.Change_type_valid`) and the group rubric
/// — resolved from the `openehr-term` bundle — as its `value`.
pub(crate) fn audit_details(
    system_id: &str,
    change_type: &str,
    description: Option<&str>,
    committer: &Value,
    time_committed: &jiff::Timestamp,
) -> Value {
    let mut audit = json!({
        "_type": "AUDIT_DETAILS",
        "system_id": system_id,
        "time_committed": { "_type": "DV_DATE_TIME", "value": time_committed.to_string() },
        "change_type": {
            "_type": "DV_CODED_TEXT",
            "value": change_type_rubric(change_type),
            "defining_code": {
                "_type": "CODE_PHRASE",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": OPENEHR },
                "code_string": change_type
            }
        },
        "committer": committer
    });
    if let (Some(desc), Value::Object(map)) = (description, &mut audit) {
        map.insert(
            "description".to_owned(),
            json!({ "_type": "DV_TEXT", "value": desc }),
        );
    }
    audit
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
/// - `AUDIT_DETAILS.System_id_valid`: `not system_id.is_empty` (RM common
///   master04 §Audit Details). Without this guard an empty client-supplied
///   `system_id` reaches the DB `System_id_valid` CHECK and surfaces as a
///   `500` — a validation failure must be `422`, not an internal error.
/// - the committer `PARTY_PROXY`'s own `PARTY_IDENTIFIED`/`PARTY_RELATED`
///   invariants `Basic_validity` + `Name_valid` (+ `Relationship_valid` for
///   `PARTY_RELATED`; RM common master04 §Party Proxies). A PARTY that appears
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

/// Enforce the committer `PARTY_PROXY`'s `Basic_validity` + `Name_valid`
/// (RM common master04 §Party Proxies, `PARTY_IDENTIFIED`): a
/// `PARTY_IDENTIFIED`/`PARTY_RELATED` committer must carry at least one of
/// `name` / `identifiers` / `external_ref`, and a present `name` must be
/// non-empty. A `PARTY_RELATED` committer additionally requires its
/// `relationship` (1..1) with `Relationship_valid`. `PARTY_SELF` has no such
/// invariant and is accepted unconditionally.
fn validate_committer(committer: &Value) -> Result<(), ServiceError> {
    let party_type = committer.get("_type").and_then(Value::as_str);
    if !matches!(party_type, Some("PARTY_IDENTIFIED" | "PARTY_RELATED")) {
        return Ok(());
    }
    let name = committer.get("name").filter(|v| !v.is_null());
    let has_identifiers = committer
        .get("identifiers")
        .and_then(Value::as_array)
        .is_some_and(|a| !a.is_empty());
    let has_external_ref = committer.get("external_ref").is_some_and(|v| !v.is_null());
    // Basic_validity: at least one of name / identifiers / external_ref.
    if name.is_none() && !has_identifiers && !has_external_ref {
        return Err(ServiceError::Unprocessable(
            "AUDIT_DETAILS.committer (PARTY_IDENTIFIED) requires at least one of \
             name, identifiers, external_ref (PARTY_IDENTIFIED.Basic_validity)"
                .to_owned(),
        ));
    }
    // Name_valid: a present name must be non-empty.
    if name.and_then(Value::as_str) == Some("") {
        return Err(ServiceError::Unprocessable(
            "AUDIT_DETAILS.committer name must be non-empty when present \
             (PARTY_IDENTIFIED.Name_valid)"
                .to_owned(),
        ));
    }
    if party_type == Some("PARTY_RELATED") {
        validate_party_related_relationship(committer)?;
    }
    Ok(())
}

/// `PARTY_RELATED.relationship` (1..1 `DV_CODED_TEXT`) + `Relationship_valid`
/// for an audit committer. The invariant is
/// `terminology(openehr).has_code_for_group_id(subject_relationship,
/// relationship.defining_code)` (RM common master04 §Party Proxies,
/// `PARTY_RELATED`) — the code must BE an openEHR `subject_relationship` group
/// member, so a `defining_code` from any other terminology fails the invariant
/// too (the spec formula has no terminology escape hatch; openEHR specs are
/// leading).
fn validate_party_related_relationship(committer: &Value) -> Result<(), ServiceError> {
    let Some(relationship) = committer.get("relationship").filter(|v| !v.is_null()) else {
        return Err(ServiceError::Unprocessable(
            "PARTY_RELATED.relationship is mandatory (1..1 DV_CODED_TEXT)".to_owned(),
        ));
    };
    let code = relationship
        .pointer("/defining_code/code_string")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ServiceError::Unprocessable(
                "PARTY_RELATED.relationship must be a DV_CODED_TEXT with a defining_code"
                    .to_owned(),
            )
        })?;
    let terminology = relationship
        .pointer("/defining_code/terminology_id/value")
        .and_then(Value::as_str)
        .unwrap_or("");
    if terminology != OPENEHR || !openehr().is_valid_subject_relationship(code) {
        return Err(ServiceError::Unprocessable(format!(
            "PARTY_RELATED.relationship code {code:?} (terminology {terminology:?}) is not \
             in the openEHR subject relationship group (Relationship_valid)"
        )));
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
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
        let audit = audit_input("ehrbase-rs.local", json!({ "_type": "PARTY_IDENTIFIED" }));
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
            "ehrbase-rs.local",
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
        match validate_commit_audit(&bad) {
            Err(ServiceError::Unprocessable(msg)) => assert!(
                msg.contains("Relationship_valid") && msg.contains("99999"),
                "got {msg}"
            ),
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
