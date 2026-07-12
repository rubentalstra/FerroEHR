//! CONTRIBUTION create + retrieval — the change-set envelope with its
//! `AUDIT_DETAILS` and the versions it produced.
//!
//! `contribution_create` applies a set of VERSIONs atomically under one
//! contribution (via `vobject::commit_contribution`). Each version's storage
//! action **and** its preserved audit change-type code come from
//! [`classify`]: the client-supplied `commit_audit.change_type` is validated
//! against the full openEHR `audit_change_type` group and stored **verbatim**
//! (never narrowed to creation/modification/deleted — RM `change_control`
//! §"Contributions"; finding F-06-06), while the storage branch collapses to
//! create / modify / delete. The object kind comes from the payload `_type`
//! (create) or the stored object (modify / delete); everything commits in one
//! transaction.

use ehrbase_rest::Page;
use ehrbase_sm::SmError;
use ehrbase_sm::{ResourceMeta, ServiceResponse};
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use super::codes::{self, change_type};
use super::vobject::{self, AuditInput, Change, Kind, PendingAttest};
use super::{EhrbaseService, ServiceError, version_id};

/// An optional `(lower, upper)` inclusive commit-time window — the simple
/// realization of the SM `Interval<Iso8601_date_time>` (either side open when
/// its bound is `None`; the whole `Option` `None` = unbounded). Both bounds are
/// already-parsed ISO 8601 timestamps.
type TimeRange = Option<(Option<jiff::Timestamp>, Option<jiff::Timestamp>)>;

/// Split a [`TimeRange`] into the two SQL bound params (ISO 8601 strings, bound
/// through the `::timestamptz` cast like every other timestamp in the service).
/// A `None` on either side (or a `None` window) becomes a `NULL` param, which
/// disables that side of the filter.
fn bounds(time_range: TimeRange) -> (Option<String>, Option<String>) {
    let (lower, upper) = time_range.unwrap_or((None, None));
    (lower.map(|t| t.to_string()), upper.map(|t| t.to_string()))
}

/// The storage branch an incoming VERSION maps to. This is deliberately
/// narrower than the `audit_change_type` group: many change kinds (amendment,
/// modification, synthesis, …) are all "commit a new content version"; the
/// audited change type is carried separately, verbatim (F-06-06).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Create,
    Modify,
    Delete,
    /// Attach an `ATTESTATION` to an existing `ORIGINAL_VERSION`
    /// (`666|attestation|`) — no new version (RM `change_control` §Contributions).
    Attest,
}

/// Classify one VERSION of a contribution: resolve (and validate) its
/// `commit_audit.change_type` to the canonical numeric `audit_change_type`
/// code, and derive the storage [`Action`], rejecting spec-invalid
/// combinations.
///
/// Spec (RM `docs/specs/openehr/RM/docs/common/master06-change_control_package.adoc`
/// §"Contributions"):
/// - *addition of new item* → a **new** `VERSIONED_OBJECT`, change type
///   `249|creation|` — so `249` with a `preceding_version_uid` is invalid,
///   and any non-`249` change type requires an existing object;
/// - *deletion* → a new version whose "data attribute is set to Void",
///   change type `523|deleted|` — so data alongside `523` is invalid;
/// - *modification of existing item* → `250|amendment|` (correction) or
///   `251|modification|` (content change); `252|synthesis|`, `253|unknown|`,
///   `816|restoration|`, `817|format conversion|` are likewise
///   content-carrying commits against an existing object;
/// - *attestation* → a new `ATTESTATION` is added to the attestations list of
///   an existing `ORIGINAL_VERSION`, change type `666|attestation|` — **not** a
///   new version (RM `change_control` §Contributions). It therefore requires a
///   `preceding_version_uid` (the target version) and must carry no `data`
///   ("attestation of an existing item adds no content").
fn classify(
    token: Option<&str>,
    has_preceding: bool,
    has_data: bool,
) -> Result<(Action, String), ServiceError> {
    let code = match token {
        Some(t) => codes::change_type_code(t).ok_or_else(|| {
            ServiceError::Unprocessable(format!(
                "change_type {t:?} is not a code in the openEHR audit_change_type group \
                 (AUDIT_DETAILS.Change_type_valid)"
            ))
        })?,
        // No client change type: infer creation vs modification from the
        // presence of preceding_version_uid.
        None if has_preceding => change_type::MODIFICATION.to_owned(),
        None => change_type::CREATION.to_owned(),
    };
    match code.as_str() {
        change_type::CREATION => {
            if has_preceding {
                return Err(ServiceError::Unprocessable(
                    "change_type 249|creation| is invalid for an existing object \
                     (preceding_version_uid present); creation commits a new \
                     VERSIONED_OBJECT (RM change_control §Contributions)"
                        .to_owned(),
                ));
            }
            if !has_data {
                return Err(ServiceError::Unprocessable(
                    "creation version needs data".to_owned(),
                ));
            }
            Ok((Action::Create, code))
        }
        change_type::DELETED => {
            if !has_preceding {
                return Err(ServiceError::Unprocessable(
                    "deleted (523) version requires preceding_version_uid".to_owned(),
                ));
            }
            if has_data {
                return Err(ServiceError::Unprocessable(
                    "deleted (523) version must not carry data — its data attribute is \
                     set to Void (RM change_control §Contributions)"
                        .to_owned(),
                ));
            }
            Ok((Action::Delete, code))
        }
        change_type::ATTESTATION => {
            // An attestation attaches to an existing ORIGINAL_VERSION identified
            // by preceding_version_uid (RM change_control §Contributions;
            // VERSIONED_OBJECT.commit_attestation pre `has_version_id`). Absent
            // → the request cannot name its target: a 400 (BadRequest), not a
            // 422, since it is a structural/addressing error.
            if !has_preceding {
                return Err(ServiceError::BadRequest(
                    "change_type 666|attestation| requires preceding_version_uid to \
                     identify the ORIGINAL_VERSION being attested (RM change_control \
                     §Contributions; VERSIONED_OBJECT.commit_attestation pre \
                     has_version_id)"
                        .to_owned(),
                ));
            }
            if has_data {
                return Err(ServiceError::Unprocessable(
                    "attestation of an existing item adds no content — a 666 version \
                     item must not carry data (RM change_control §Contributions)"
                        .to_owned(),
                ));
            }
            Ok((Action::Attest, code))
        }
        // amendment 250 / modification 251 / synthesis 252 / unknown 253 /
        // restoration 816 / format conversion 817: a content-carrying new
        // version of an existing object; the code is preserved verbatim.
        _ => {
            if !has_preceding {
                return Err(ServiceError::Unprocessable(format!(
                    "change_type {code} requires preceding_version_uid — a first \
                     version's change type is 249|creation| (RM change_control \
                     §Contributions)"
                )));
            }
            if !has_data {
                return Err(ServiceError::Unprocessable(format!(
                    "change_type {code} version needs data"
                )));
            }
            Ok((Action::Modify, code))
        }
    }
}

/// Validate a client-supplied commit `AUDIT_DETAILS`' non-terminology RM
/// invariants before it is persisted (a CONTRIBUTION audit or a version
/// `commit_audit`). Two invariants are enforced here as a service-layer `422`:
///
/// - `AUDIT_DETAILS.System_id_valid`: `not system_id.is_empty`
///   (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.audit_details.adoc`
///   invariant `System_id_valid`). Without this guard an empty client-supplied
///   `system_id` reaches the `ck_audit_system_id_nonempty` DB CHECK and surfaces
///   as a `500` (a validation failure must be `422`, not an internal error).
/// - the committer `PARTY_PROXY`'s own `PARTY_IDENTIFIED`/`PARTY_RELATED`
///   invariants `Basic_validity` + `Name_valid`
///   (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.party_identified.adoc`).
///   A PARTY that appears as *content* is validated by the RM-invariant pass,
///   but the audit committer is stored verbatim, so it is checked here.
///
/// `change_type` is validated separately ([`classify`] / [`codes::change_type_code`]).
fn validate_commit_audit(audit: &vobject::AuditInput) -> Result<(), ServiceError> {
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
/// (`party_identified.adoc`): a `PARTY_IDENTIFIED`/`PARTY_RELATED` committer must
/// carry at least one of `name` / `identifiers` / `external_ref`, and a present
/// `name` must be non-empty. A `PARTY_RELATED` committer additionally requires
/// its `relationship` (1..1) with `Relationship_valid`:
/// `terminology(openehr).has_code_for_group_id(subject_relationship, …)`
/// (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.party_related.adoc`).
/// `PARTY_SELF` (the subject-of-record proxy) has no such invariant and is
/// accepted unconditionally.
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
/// relationship.defining_code)` (`party_related.adoc`) — the code must BE an
/// openEHR `subject_relationship` group member, so a `defining_code` from any
/// other terminology fails the invariant too (the spec formula has no
/// terminology escape hatch; openEHR specs are leading). A PARTY that appears
/// as *content* gets the group check from the validation walker's terminology
/// pass (`openehr-flat` `terminology.rs`, F-11-05).
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
    if terminology != "openehr"
        || !openehr_term::bundle::openehr().is_valid_subject_relationship(code)
    {
        return Err(ServiceError::Unprocessable(format!(
            "PARTY_RELATED.relationship code {code:?} (terminology {terminology:?}) is not \
             in the openEHR subject relationship group (Relationship_valid)"
        )));
    }
    Ok(())
}

impl EhrbaseService {
    /// Commit a raw-wire EHR CONTRIBUTION body atomically, returning the stored
    /// `CONTRIBUTION` with its resource metadata (the `contribution_uid` for the
    /// `201` `ETag`/`Location`). The EHR-scoped analogue of
    /// [`create_demographic_contribution`](Self::create_demographic_contribution)
    /// (`ehr_id = Some`, `party_only = false`), over the shared
    /// [`commit_version_set`](Self::commit_version_set).
    ///
    /// PORT NOTE: the SM native `commit_contribution(Vec<UpdateVersion>,
    /// UpdateAudit)` ([`EhrContributionService`](ehrbase_sm::EhrContributionService))
    /// is a *typed subset* of the wire CONTRIBUTION — `UpdateVersion` mandates
    /// `data` + `lifecycle_state` (SM `update_version.adoc`, both `1..1`) and a
    /// committer, so it cannot represent an attestation-only (`666`) member (no
    /// `data`/`lifecycle_state`), a delete (`523`) member (no data), or a member
    /// that inherits its committer from the CONTRIBUTION audit (RM common
    /// `master06-change_control_package.adoc` §Committal m4). This raw-body seam
    /// restores the full-fidelity EHR CONTRIBUTION commit the pre-SM
    /// `contribution_create` provided; all RM `change_control` semantics stay in
    /// `commit_version_set`.
    ///
    /// # Errors
    ///
    /// Returns an [`SmError`] if the CONTRIBUTION fails validation or its commit
    /// (the `commit_version_set` error surface: bad version classification,
    /// content/terminology validation, optimistic-lock mismatch, storage faults).
    pub async fn create_ehr_contribution(
        &self,
        ehr_id: Uuid,
        body: Value,
    ) -> Result<ServiceResponse, SmError> {
        let contribution_id = self.commit_version_set(Some(ehr_id), &body, false).await?;
        let body = self.get_contribution(ehr_id, contribution_id).await?;
        let meta = ResourceMeta::new(ehr_id.to_string(), contribution_id.to_string());
        Ok(ServiceResponse::new(body, meta))
    }

    /// Commit a CONTRIBUTION's version set atomically under one contribution +
    /// audit, returning the new contribution id. Shared by the EHR-scoped
    /// [`create_contribution`](Self::create_contribution) (`ehr_id = Some`,
    /// `party_only = false`) and the demographic contribution path
    /// (`ehr_id = None`, `party_only = true`). Each version's storage action and
    /// preserved audit change-type code come from [`classify`]; the object kind
    /// from the payload `_type` (create) or the stored object (modify/delete).
    ///
    /// `party_only` gates the version kinds: a demographic contribution may only
    /// carry party objects (PERSON/…/ROLE), and an EHR contribution may not —
    /// a mismatch is `422` (the analogue of the EHR-group contribution rejecting
    /// bad content).
    #[allow(clippy::too_many_lines)] // the per-version classify + change-build loop
    pub(super) async fn commit_version_set(
        &self,
        ehr_id: Option<Uuid>,
        body: &Value,
        party_only: bool,
    ) -> Result<Uuid, ServiceError> {
        let versions = body
            .get("versions")
            .and_then(Value::as_array)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                ServiceError::Unprocessable("contribution must contain versions".to_owned())
            })?;

        // A client-supplied CONTRIBUTION uid is honoured when unused and
        // rejected when malformed or already in use (ITS-REST
        // `contribution_create`; RM common master06 §CONTRIBUTION `uid`).
        let supplied_uid = match body
            .get("uid")
            .and_then(|u| u.get("value"))
            .and_then(Value::as_str)
        {
            None => None,
            Some(raw) => Some(raw.parse::<Uuid>().map_err(|_| {
                ServiceError::Unprocessable(format!(
                    "CONTRIBUTION uid {raw:?} is not a valid HIER_OBJECT_ID UUID"
                ))
            })?),
        };

        // master06 §Committal (m4): "these three attributes (system_id, committer,
        // time_committed of AUDIT_DETAILS) should be copied into the corresponding
        // attributes of the commit_audit of each VERSION included in the
        // CONTRIBUTION". We therefore default each version's committer/system_id
        // from the CONTRIBUTION's own audit (below), not merely from the request
        // principal. time_committed is always the server commit-act time (§Committal
        // "computed on the server"). The contribution audit committer/system_id
        // themselves default to the principal / service system id.
        let contrib_committer = body
            .get("audit")
            .and_then(|a| a.get("committer"))
            .cloned()
            .unwrap_or_else(super::ehr::committer);
        let contrib_system_id = body
            .get("audit")
            .and_then(|a| a.get("system_id"))
            .and_then(Value::as_str)
            .map_or_else(|| self.effective_system_id(), str::to_owned);

        let mut changes: Vec<(AuditInput, Change)> = Vec::with_capacity(versions.len());
        // 666 attestations of existing versions (added to the same CONTRIBUTION,
        // but committing no new version — RM change_control §Contributions).
        let mut attests: Vec<PendingAttest> = Vec::new();
        let mut version_codes: Vec<String> = Vec::with_capacity(versions.len());
        for version in versions {
            let token = version
                .get("commit_audit")
                .and_then(|a| a.get("change_type"))
                .and_then(coded_value);
            // A JSON `"data": null` is "no data" (the deleted-version shape).
            let data = version.get("data").cloned().filter(|d| !d.is_null());
            // PORT NOTE: a first version legitimately carries no
            // `preceding_version_uid` (SM `update_version.adoc:15` types it
            // `0..1`; `master03-common_package.adoc:25`: "must be specified,
            // except in the case where the version update is a first version").
            // The SM `commit_contribution` glue serializes the typed
            // `UpdateVersion.preceding_version_uid = None` to a JSON `null`, so a
            // bare `.is_some()` would misread that as "present" and classify a
            // spec-legal creation as a modify — treat a `null` as absent.
            let (action, code) = classify(
                token.as_deref(),
                version
                    .get("preceding_version_uid")
                    .is_some_and(|v| !v.is_null()),
                data.is_some(),
            )?;
            // Every member's change type feeds the CONTRIBUTION aggregate — incl.
            // `666`, so a 666-only contribution aggregates to `666` (below).
            version_codes.push(code.clone());

            // A 666 attestation adds no version (and no version audit row): its
            // content is the item's commit audit (an UPDATE_ATTESTATION),
            // completed into a full ATTESTATION at commit time.
            if action == Action::Attest {
                let (vo_id, expected) = parse_preceding(version)?;
                let kind = self.require_kind(vo_id).await?;
                check_kind_scope(kind, party_only)?;
                let partial = version.get("commit_audit").cloned().ok_or_else(|| {
                    ServiceError::Unprocessable(
                        "666 attestation version requires a commit_audit \
                         (the UPDATE_ATTESTATION)"
                            .to_owned(),
                    )
                })?;
                attests.push(PendingAttest {
                    vo_id,
                    kind,
                    expected,
                    partial,
                });
                continue;
            }

            // m4: default committer/system_id from the CONTRIBUTION audit when the
            // version item omits them (master06 §Committal). A version item that
            // *does* supply distinct committer/system_id keeps them verbatim —
            // the spec wording is "should be copied", so an explicit per-version
            // value is honoured (PORT NOTE: a deliberate SHOULD, not MUST).
            let version_audit = Self::parse_version_audit(
                version.get("commit_audit"),
                code,
                &contrib_committer,
                &contrib_system_id,
            );
            // AUDIT_DETAILS.System_id_valid + committer PARTY invariants
            // (audit_details.adoc / party_identified.adoc) — a client-supplied
            // version commit_audit must be a valid RM instance (422, not the
            // DB-CHECK 500).
            validate_commit_audit(&version_audit)?;
            // UPDATE_VERSION.lifecycle_state (RM common master06 §Version
            // Lifecycle) — a TerminologyCode {terminology_id, code_string};
            // absent → 532|complete| (default). Validated + resolved in the
            // shared commit path (`vobject::apply_change`).
            let lifecycle_state = lifecycle_of(version);
            // A `553|incomplete|` version gets relaxed content validation
            // (existence/cardinality lower limits treated as zero — RM common
            // master06 §"Incomplete Content"). Resolve the raw token (code or
            // rubric) to its canonical group code before comparing.
            let incomplete = lifecycle_state
                .as_deref()
                .and_then(codes::lifecycle_state_code)
                .is_some_and(|c| c == codes::lifecycle::INCOMPLETE);
            // A client-supplied UPDATE_VERSION.signature (RM common §"Digital
            // Signature") is stored verbatim; absent, the server signs (§3.3).
            let signature = version
                .get("signature")
                .and_then(Value::as_str)
                .map(str::to_owned);
            // UPDATE_VERSION.attestations: ATTESTATIONs committed together with a
            // NEW version (RM change_control §Attestation "Signing content at
            // committal"). Carried as raw partials; completed at commit time.
            let accompanying = attestations_of(version);
            let change = match action {
                Action::Create => {
                    let data = data.ok_or_else(|| {
                        ServiceError::Unprocessable("creation version needs data".to_owned())
                    })?;
                    let kind = data_kind(&data)?;
                    check_kind_scope(kind, party_only)?;
                    // A CONTRIBUTION commit is a full commit route: its versions
                    // are validated exactly as a direct create/update (F-07-01),
                    // relaxed for a `553|incomplete|` lifecycle (master06
                    // §"Incomplete Content").
                    self.validate_for_commit(kind, &data, incomplete).await?;
                    // An EHR holds exactly one EHR_STATUS / EHR_ACCESS
                    // (`ehr_status 1..1` — RM ehr, EHR class); a CONTRIBUTION that
                    // *creates* a second one is rejected (CNF master08
                    // `commit_contribution-ehr_status_invalid_change_type`). A
                    // FOLDER creation is NOT rejected: each new FOLDER hierarchy
                    // joins `EHR.folders` as a new member (RM ehr master04
                    // §Folders — "an entirely new Folder hierarchy may be added"),
                    // so folders are unbounded on the CONTRIBUTION path (like
                    // COMPOSITIONs).
                    if let Some(ehr_id) = ehr_id {
                        self.reject_duplicate_singleton(ehr_id, kind).await?;
                    }
                    Change::Create {
                        kind,
                        canonical: data,
                        template_id: None,
                        signature,
                        lifecycle_state,
                        attestations: accompanying,
                    }
                }
                Action::Modify => {
                    let data = data.ok_or_else(|| {
                        ServiceError::Unprocessable("modification version needs data".to_owned())
                    })?;
                    let (vo_id, expected) = parse_preceding(version)?;
                    let kind = self.require_kind(vo_id).await?;
                    check_kind_scope(kind, party_only)?;
                    self.validate_for_commit(kind, &data, incomplete).await?;
                    // ORIGINAL_VERSION.other_input_version_uids: merge provenance
                    // accepted on the wire (RM common master06 §Version Merging) —
                    // OBJECT_VERSION_ID values (string or {value}) preserved.
                    let other_input_version_uids = version
                        .get("other_input_version_uids")
                        .and_then(Value::as_array)
                        .map(|a| {
                            a.iter()
                                .filter_map(|u| {
                                    u.as_str()
                                        .or_else(|| u.get("value").and_then(Value::as_str))
                                        .map(str::to_owned)
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    Change::Modify {
                        vo_id,
                        kind,
                        canonical: data,
                        expected: Some(expected),
                        template_id: None,
                        signature,
                        lifecycle_state,
                        attestations: accompanying,
                        other_input_version_uids,
                    }
                }
                Action::Delete => {
                    let (vo_id, expected) = parse_preceding(version)?;
                    let kind = self.require_kind(vo_id).await?;
                    check_kind_scope(kind, party_only)?;
                    Change::Delete {
                        vo_id,
                        kind,
                        expected: Some(expected),
                        signature,
                    }
                }
                // Handled above (attestations commit no version).
                Action::Attest => unreachable!("Action::Attest handled before this match"),
            };
            changes.push((version_audit, change));
        }

        // EHR_STATUS.is_modifiable = False forbids content writes (ehr/master04
        // §"EHR Active Status"): a CONTRIBUTION that creates/modifies/deletes any
        // EHR content (COMPOSITION / FOLDER / EHR_ACCESS — everything other than
        // the EHR_STATUS object) is refused when the EHR is deactivated. An
        // EHR_STATUS-only CONTRIBUTION (incl. the one that flips is_modifiable
        // back to true) stays allowed, since the EHR_STATUS object "is always
        // modifiable". 666 attestations add no version and modify no content, so
        // they do not trip the guard.
        if let Some(ehr_id) = ehr_id
            && changes.iter().any(|(_, c)| c.kind() != Kind::EhrStatus)
        {
            self.ensure_content_writable(ehr_id).await?;
        }

        // The CONTRIBUTION's own audit: a client-supplied change_type is
        // validated against the group and preserved; otherwise the spec's
        // aggregate guidance applies (RM change_control §"Contributions":
        // "any code: when all member versions have the same change type, that
        // change type may be used for the Contribution as well", with
        // `251|modification|` accommodating a mixture).
        let contribution_code = match body
            .get("audit")
            .and_then(|a| a.get("change_type"))
            .and_then(coded_value)
        {
            Some(token) => codes::change_type_code(&token).ok_or_else(|| {
                ServiceError::Unprocessable(format!(
                    "contribution audit change_type {token:?} is not a code in the \
                     openEHR audit_change_type group (AUDIT_DETAILS.Change_type_valid)"
                ))
            })?,
            None => aggregate_change_type(&version_codes),
        };
        let contribution_audit = self.parse_audit(body.get("audit"), contribution_code);
        // The CONTRIBUTION's own AUDIT_DETAILS must likewise be a valid RM
        // instance (System_id_valid + committer PARTY invariants).
        validate_commit_audit(&contribution_audit)?;

        let mut tx = self.pool.begin().await?;
        let (contribution_id, committed) = vobject::commit_contribution(
            &mut tx,
            ehr_id,
            supplied_uid,
            &contribution_audit,
            changes,
            attests,
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);

        // An EHR_ACCESS version in this set changes the EHR's access-control
        // policy (the settings are change-controlled — RM ehr
        // `master04-ehr_package.adoc` §EHR Access), so drop the cached settings
        // the access gate consults per request.
        if let Some(ehr_id) = ehr_id
            && committed.iter().any(|c| c.kind == Kind::EhrAccess)
        {
            self.invalidate_ehr_access(ehr_id).await;
        }

        Ok(contribution_id)
    }

    /// Reject the *creation* of a second EHR-singleton versioned object. An EHR
    /// holds exactly one `EHR_STATUS` and one `EHR_ACCESS` (`EHR.ehr_status 1..1`,
    /// `ehr_access 1..1`) — RM ehr, EHR class. These are provisioned when the EHR
    /// is created; a CONTRIBUTION `creation` of another one is invalid (CNF
    /// master08 `-ehr_status_invalid_change_type`). COMPOSITIONs and FOLDERs are
    /// unbounded — each new FOLDER hierarchy is a new member of `EHR.folders` (RM
    /// ehr master04 §Folders) — so they pass through. A live singleton already
    /// present → `409 Conflict`.
    async fn reject_duplicate_singleton(
        &self,
        ehr_id: Uuid,
        kind: Kind,
    ) -> Result<(), ServiceError> {
        if !matches!(kind, Kind::EhrStatus | Kind::EhrAccess) {
            return Ok(());
        }
        if self.current_vo(ehr_id, kind).await?.is_some() {
            return Err(ServiceError::Conflict(format!(
                "EHR {ehr_id} already has a {}; only one is permitted (RM ehr, EHR class)",
                kind.as_str()
            )));
        }
        Ok(())
    }

    /// The stored kind of an existing object, or `NotFound`.
    async fn require_kind(&self, vo_id: Uuid) -> Result<Kind, ServiceError> {
        vobject::object_kind(&self.pool, vo_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("versioned object {vo_id}")))
    }

    /// Build an [`AuditInput`] from an ITS-REST audit object (`UpdateAudit`)
    /// and the already-resolved numeric `audit_change_type` code (validated by
    /// [`classify`] / the contribution-audit resolution), with the committer
    /// defaulting to the authenticated principal.
    fn parse_audit(&self, audit: Option<&Value>, change_type: String) -> AuditInput {
        let description = audit
            .and_then(|a| a.get("description"))
            .and_then(|d| d.get("value"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let committer = audit
            .and_then(|a| a.get("committer"))
            .cloned()
            .unwrap_or_else(super::ehr::committer);
        let system_id = audit
            .and_then(|a| a.get("system_id"))
            .and_then(Value::as_str)
            .map_or_else(|| self.effective_system_id(), str::to_owned);
        AuditInput {
            system_id,
            change_type,
            description,
            committer,
        }
    }

    /// Build an [`AuditInput`] for a VERSION's `commit_audit`. Unlike
    /// [`parse_audit`](Self::parse_audit) (which defaults to the request
    /// principal / service system id), the `committer` and `system_id` here
    /// default from the enclosing CONTRIBUTION's audit when the version item
    /// omits them — realizing the master06 §Committal copy rule (m4). A version
    /// item that supplies its own `committer`/`system_id` keeps them verbatim.
    fn parse_version_audit(
        audit: Option<&Value>,
        change_type: String,
        fallback_committer: &Value,
        fallback_system_id: &str,
    ) -> AuditInput {
        let description = audit
            .and_then(|a| a.get("description"))
            .and_then(|d| d.get("value"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let committer = audit
            .and_then(|a| a.get("committer"))
            .cloned()
            .unwrap_or_else(|| fallback_committer.clone());
        let system_id = audit
            .and_then(|a| a.get("system_id"))
            .and_then(Value::as_str)
            .map_or_else(|| fallback_system_id.to_owned(), str::to_owned);
        AuditInput {
            system_id,
            change_type,
            description,
            committer,
        }
    }

    /// Retrieve a CONTRIBUTION by id (scoped to the EHR), with its audit and the
    /// `OBJECT_REFs` of the versions it committed.
    pub(super) async fn get_contribution(
        &self,
        ehr_id: Uuid,
        contribution_id: Uuid,
    ) -> Result<Value, ServiceError> {
        self.get_contribution_inner(ehr_id, contribution_id, false)
            .await
    }

    /// `get_contribution` with `Prefer: resolve_refs` honoured: the
    /// `versions` list carries the resolved `ORIGINAL_VERSION` objects instead
    /// of `OBJECT_REF`s (ITS-REST `Requests_and_responses` §Representation
    /// details negotiation).
    pub(super) async fn get_contribution_resolved(
        &self,
        ehr_id: Uuid,
        contribution_id: Uuid,
    ) -> Result<Value, ServiceError> {
        self.get_contribution_inner(ehr_id, contribution_id, true)
            .await
    }

    async fn get_contribution_inner(
        &self,
        ehr_id: Uuid,
        contribution_id: Uuid,
        resolve_refs: bool,
    ) -> Result<Value, ServiceError> {
        let meta = sqlx::query(
            "SELECT a.system_id, a.change_type, a.description, a.committer, a.time_committed \
             FROM contribution c JOIN audit a ON a.id = c.audit_id \
             WHERE c.id = $1 AND c.ehr_id = $2",
        )
        .bind(contribution_id)
        .bind(ehr_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("CONTRIBUTION {contribution_id}")))?;

        let system_id: String = meta.try_get("system_id")?;
        let change_type: String = meta.try_get("change_type")?;
        let description: Option<String> = meta.try_get("description")?;
        let committer: Value = meta.try_get("committer")?;
        let time_committed: jiff::Timestamp = meta
            .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
            .to_jiff();

        // CONTRIBUTION.versions lists the affected VERSION objects (RM
        // change_control §Contributions). A 666 attestation commits no new
        // version but still affects an existing one, so union the versions
        // referenced by this contribution's `vo_attestation` rows (dedup, no
        // duplicate OBJECT_REF for a version that was both written and attested
        // in the same contribution).
        let version_rows = sqlx::query(
            "SELECT vo_id, trunk_version, branch_number, branch_version, creating_system_id, \
             kind FROM vo_version \
             WHERE contribution_id = $1 \
             UNION \
             SELECT v.vo_id, v.trunk_version, v.branch_number, v.branch_version, \
             v.creating_system_id, v.kind FROM vo_version v \
             JOIN vo_attestation att ON att.vo_id = v.vo_id AND att.sys_version = v.sys_version \
             WHERE att.contribution_id = $1 \
             ORDER BY vo_id",
        )
        .bind(contribution_id)
        .fetch_all(&self.pool)
        .await?;

        let referenced: Vec<(Uuid, super::version_id::TreeId, Value)> = version_rows
            .iter()
            .map(
                |row| -> Result<(Uuid, super::version_id::TreeId, Value), ServiceError> {
                    let vo_id: Uuid = row.try_get("vo_id")?;
                    let tree = super::version_id::TreeId::from_columns(
                        row.try_get("trunk_version")?,
                        row.try_get("branch_number")?,
                        row.try_get("branch_version")?,
                    );
                    let creating_system_id: String = row.try_get("creating_system_id")?;
                    let kind: String = row.try_get("kind")?;
                    Ok((
                        vo_id,
                        tree,
                        json!({
                            "_type": "OBJECT_REF",
                            "namespace": "local",
                            "type": kind,
                            "id": {
                                "_type": "OBJECT_VERSION_ID",
                                "value": self.object_version_id(vo_id, &creating_system_id, tree)
                            }
                        }),
                    ))
                },
            )
            .collect::<Result<Vec<_>, _>>()?;

        let mut versions = Vec::with_capacity(referenced.len());
        for (vo_id, tree, obj_ref) in referenced {
            if resolve_refs {
                // Resolve the ref to the full ORIGINAL_VERSION (Prefer:
                // resolve_refs).
                let read = super::vobject::read_version(&self.pool, vo_id, tree)
                    .await?
                    .ok_or_else(|| ServiceError::NotFound(format!("VERSION {vo_id}::{tree}")))?;
                versions.push(self.original_version(&read)?);
            } else {
                versions.push(obj_ref);
            }
        }

        Ok(json!({
            "_type": "CONTRIBUTION",
            "uid": { "_type": "HIER_OBJECT_ID", "value": contribution_id.to_string() },
            "audit": Self::audit_details(&system_id, &change_type, description.as_deref(), &committer, &time_committed),
            "versions": versions
        }))
    }

    /// SM `I_EHR_CONTRIBUTION.list_contributions` — the ids of the EHR's
    /// CONTRIBUTIONs, oldest-first (audit `time_committed`, then id), within the
    /// optional `(lower, upper)` inclusive commit-time window, paged by `page`
    /// (`docs/specs/openehr/SM/docs/UML/classes/i_ehr_contribution.adoc`;
    /// "Obtain a list of identifiers of Contributions in EHR"). A missing EHR is
    /// the SM `ehr_does_not_exist` → `NotFound`.
    pub(super) async fn list_contributions(
        &self,
        ehr_id: Uuid,
        time_range: TimeRange,
        page: Page,
    ) -> Result<Vec<Uuid>, ServiceError> {
        self.ensure_ehr_exists(ehr_id).await?;
        let (lower, upper) = bounds(time_range);
        // A NULL bound param disables that side (`$n IS NULL OR …`); a NULL LIMIT
        // returns all rows (Postgres `LIMIT NULL`); OFFSET defaults to 0.
        let rows = sqlx::query(
            "SELECT c.id FROM contribution c JOIN audit a ON a.id = c.audit_id \
             WHERE c.ehr_id = $1 \
               AND ($2::timestamptz IS NULL OR a.time_committed >= $2::timestamptz) \
               AND ($3::timestamptz IS NULL OR a.time_committed <= $3::timestamptz) \
             ORDER BY a.time_committed, c.id \
             OFFSET $4 LIMIT $5",
        )
        .bind(ehr_id)
        .bind(lower)
        .bind(upper)
        .bind(i64::try_from(page.offset()).unwrap_or(i64::MAX))
        .bind(page.limit().map(|l| i64::try_from(l).unwrap_or(i64::MAX)))
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| Ok(row.try_get("id")?))
            .collect::<Result<Vec<Uuid>, ServiceError>>()
    }

    /// SM `I_EHR_CONTRIBUTION.contribution_count` — the number of CONTRIBUTIONs
    /// in the EHR within the optional `(lower, upper)` inclusive commit-time
    /// window (`docs/specs/openehr/SM/docs/UML/classes/i_ehr_contribution.adoc`).
    /// A missing EHR is the SM `ehr_does_not_exist` → `NotFound`.
    pub(super) async fn count_contributions(
        &self,
        ehr_id: Uuid,
        time_range: TimeRange,
    ) -> Result<i64, ServiceError> {
        self.ensure_ehr_exists(ehr_id).await?;
        let (lower, upper) = bounds(time_range);
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM contribution c JOIN audit a ON a.id = c.audit_id \
             WHERE c.ehr_id = $1 \
               AND ($2::timestamptz IS NULL OR a.time_committed >= $2::timestamptz) \
               AND ($3::timestamptz IS NULL OR a.time_committed <= $3::timestamptz)",
        )
        .bind(ehr_id)
        .bind(lower)
        .bind(upper)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    /// Build an `AUDIT_DETAILS` from stored audit columns. `change_type` is the
    /// numeric `audit_change_type` group code (`249`/`251`/`523`/…) stored in the
    /// `audit` row; the emitted `DV_CODED_TEXT` carries the code as
    /// `defining_code.code_string` (RM `AUDIT_DETAILS.Change_type_valid`) and the
    /// group rubric — resolved from the `openehr-term` bundle — as `value`
    /// (findings F-06-02, F-11-01, F-01-06, F-02-06).
    pub(super) fn audit_details(
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
                "value": super::codes::change_type_rubric(change_type),
                "defining_code": {
                    "_type": "CODE_PHRASE",
                    "terminology_id": { "_type": "TERMINOLOGY_ID", "value": super::codes::OPENEHR },
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
}

/// The CONTRIBUTION-level aggregate change type when the client supplied none
/// (RM `change_control` §"Contributions"): the shared code when every member
/// version has the same change type, else `251|modification|` ("accommodates
/// cases where there is a mixture of creation, deletion, modification").
fn aggregate_change_type(version_codes: &[String]) -> String {
    match version_codes.split_first() {
        Some((first, rest)) if rest.iter().all(|c| c == first) => first.clone(),
        _ => change_type::MODIFICATION.to_owned(),
    }
}

/// Enforce that a version's object kind matches the contribution's scope: a
/// demographic contribution (`party_only`) may carry only party roots, and an
/// EHR contribution may carry only clinical versioned objects. A mismatch is
/// `422` — the analogue of the EHR-group contribution rejecting bad content.
fn check_kind_scope(kind: Kind, party_only: bool) -> Result<(), ServiceError> {
    // A demographic CONTRIBUTION carries the ehr-less demographic kinds (the five
    // party roots + PARTY_RELATIONSHIP); an EHR CONTRIBUTION carries neither.
    if party_only && !kind.is_demographic() {
        return Err(ServiceError::Unprocessable(format!(
            "a demographic CONTRIBUTION may only contain demographic versions, got {}",
            kind.as_str()
        )));
    }
    if !party_only && kind.is_demographic() {
        return Err(ServiceError::Unprocessable(format!(
            "an EHR CONTRIBUTION may not contain demographic versions, got {}",
            kind.as_str()
        )));
    }
    Ok(())
}

/// The change-type code of a `DV_CODED_TEXT`: its `defining_code.code_string`
/// if present, else its `value`.
fn coded_value(dv: &Value) -> Option<String> {
    dv.get("defining_code")
        .and_then(|c| c.get("code_string"))
        .and_then(Value::as_str)
        // PORT NOTE: `UPDATE_AUDIT.change_type` is a `Terminology_code`
        // (`{terminology_id, code_string}`), not a `DV_CODED_TEXT`
        // (SM `update_audit.adoc:16`). The SM `commit_contribution` glue
        // serializes the typed `UpdateAudit.change_type` to exactly that shape,
        // so read a top-level `code_string` in addition to the `DV_CODED_TEXT`
        // `defining_code.code_string`/`value` forms; otherwise the client's
        // change type is lost and defaults to creation/modification.
        .or_else(|| dv.get("code_string").and_then(Value::as_str))
        .or_else(|| dv.get("value").and_then(Value::as_str))
        .map(str::to_owned)
}

/// Extract a VERSION item's `lifecycle_state` token (RM common master06
/// §Version Lifecycle), if present. `UPDATE_VERSION.lifecycle_state` is a
/// `TerminologyCode {terminology_id, code_string}` on the wire, so the code is
/// `lifecycle_state.code_string`; we also accept a `DV_CODED_TEXT`
/// (`defining_code.code_string`), a bare `{value}`, or a plain string, matching
/// the leniency of [`coded_value`]. `None` → the shared commit path defaults to
/// `532|complete|`.
fn lifecycle_of(version: &Value) -> Option<String> {
    let ls = version.get("lifecycle_state")?;
    if let Some(s) = ls.as_str() {
        return Some(s.to_owned());
    }
    ls.get("code_string")
        .and_then(Value::as_str)
        .or_else(|| {
            ls.get("defining_code")
                .and_then(|c| c.get("code_string"))
                .and_then(Value::as_str)
        })
        .or_else(|| ls.get("value").and_then(Value::as_str))
        .map(str::to_owned)
}

/// The versioned-object kind of a VERSION's `data`, from its `_type`.
fn data_kind(data: &Value) -> Result<Kind, ServiceError> {
    let rm_type = data
        .get("_type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    Kind::from_type(rm_type).ok_or_else(|| {
        ServiceError::Unprocessable(format!("not a versioned root type: {rm_type:?}"))
    })
}

/// Parse a VERSION's `preceding_version_uid` (`OBJECT_VERSION_ID`, as a string or
/// `{value}`) into the object id and the version it must currently be at —
/// through the strict BASE three-part parse (`version_id`; F-13-01).
fn parse_preceding(version: &Value) -> Result<(Uuid, version_id::TreeId), ServiceError> {
    let raw = version
        .get("preceding_version_uid")
        // PORT NOTE: treat a JSON `null` as absent (the SM glue serializes a
        // `None` preceding uid to `null`) — consistent with the create-vs-modify
        // classification above; a genuine modify/delete with no preceding still
        // errors here.
        .filter(|p| !p.is_null())
        .and_then(|p| {
            p.as_str()
                .or_else(|| p.get("value").and_then(Value::as_str))
        })
        .ok_or_else(|| {
            ServiceError::Unprocessable(
                "preceding_version_uid required for modify/delete".to_owned(),
            )
        })?;
    Ok(version_id::parse_version_uid(raw)?)
}

/// The raw wire `UPDATE_VERSION.attestations` array of a version item (partial
/// `UPDATE_ATTESTATION`s), empty when absent.
fn attestations_of(version: &Value) -> Vec<Value> {
    version
        .get("attestations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// Complete a wire `UPDATE_ATTESTATION` partial into a full canonical RM
/// `ATTESTATION` (RM common `attestation.adoc`; ITS-REST `UpdateAttestation.yaml`).
/// The server supplies the inherited `AUDIT_DETAILS` fields it owns —
/// `system_id`, `time_committed`, and the `666|attestation|` `change_type`
/// `DV_CODED_TEXT` — exactly as `UPDATE_AUDIT` → `AUDIT_DETAILS`
/// (master03 §Version Update Semantics), then adds the `ATTESTATION`-specific
/// attributes.
///
/// Validates the RM invariants: `reason` is mandatory (1..1) and, when coded
/// (`DV_CODED_TEXT`), its `defining_code` must be in the openEHR
/// `attestation reason` group (`ATTESTATION.Reason_valid`); `is_pending` is a
/// mandatory `Boolean` (1..1); `items`, if present, must be non-empty
/// (`ATTESTATION.Items_valid`). `committer` comes from the partial when present,
/// else the CONTRIBUTION's committer (master06 §Committal).
pub(super) fn complete_attestation(
    partial: &Value,
    system_id: &str,
    committer_fallback: &Value,
    now: jiff::Timestamp,
) -> Result<Value, ServiceError> {
    // reason (1..1)
    let reason = partial.get("reason").cloned().ok_or_else(|| {
        ServiceError::Unprocessable("ATTESTATION.reason is required (1..1)".to_owned())
    })?;
    // Reason_valid: if the reason is a DV_CODED_TEXT, its defining_code must be
    // a member of the openEHR `attestation reason` group.
    if reason.get("_type").and_then(Value::as_str) == Some("DV_CODED_TEXT") {
        let code = reason
            .pointer("/defining_code/code_string")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !openehr_term::bundle::openehr().is_valid_attestation_reason(code) {
            return Err(ServiceError::Unprocessable(format!(
                "ATTESTATION.reason.defining_code {code:?} is not in the openEHR \
                 `attestation reason` group (ATTESTATION.Reason_valid)"
            )));
        }
    }
    // is_pending (1..1, Boolean)
    let is_pending = partial
        .get("is_pending")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            ServiceError::Unprocessable(
                "ATTESTATION.is_pending is required (1..1 Boolean)".to_owned(),
            )
        })?;
    // items (0..1); Items_valid: non-empty when present.
    let items = partial.get("items");
    if let Some(items) = items
        && items.as_array().is_none_or(Vec::is_empty)
    {
        return Err(ServiceError::Unprocessable(
            "ATTESTATION.items must be a non-empty list when present \
             (ATTESTATION.Items_valid)"
                .to_owned(),
        ));
    }
    // committer: from the partial if present, else the CONTRIBUTION committer.
    let committer = partial
        .get("committer")
        .cloned()
        .unwrap_or_else(|| committer_fallback.clone());
    // description: UpdateAudit.description is UDvText (plain string or DV_TEXT).
    let description = partial.get("description").and_then(|d| {
        d.as_str()
            .or_else(|| d.get("value").and_then(Value::as_str))
    });
    // The inherited AUDIT_DETAILS fields, built exactly like any audit, then
    // retyped to ATTESTATION with its own attributes appended.
    let mut att = EhrbaseService::audit_details(
        system_id,
        change_type::ATTESTATION,
        description,
        &committer,
        &now,
    );
    if let Value::Object(map) = &mut att {
        map.insert("_type".to_owned(), Value::String("ATTESTATION".to_owned()));
        map.insert("reason".to_owned(), reason);
        map.insert("is_pending".to_owned(), Value::Bool(is_pending));
        if let Some(v) = partial.get("attested_view") {
            map.insert("attested_view".to_owned(), v.clone());
        }
        if let Some(v) = partial.get("proof") {
            map.insert("proof".to_owned(), v.clone());
        }
        if let Some(v) = items {
            map.insert("items".to_owned(), v.clone());
        }
    }
    Ok(att)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classify_err(token: Option<&str>, has_preceding: bool, has_data: bool) -> String {
        match classify(token, has_preceding, has_data) {
            Err(ServiceError::Unprocessable(msg)) => msg,
            other => panic!("expected Unprocessable, got {other:?}"),
        }
    }

    fn classify_bad_request(token: Option<&str>, has_preceding: bool, has_data: bool) -> String {
        match classify(token, has_preceding, has_data) {
            Err(ServiceError::BadRequest(msg)) => msg,
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }

    #[test]
    fn classify_preserves_the_full_change_type_set() {
        // F-06-06: amendment / synthesis / unknown round-trip verbatim as
        // content commits; nothing is narrowed to "modification".
        for code in ["250", "251", "252", "253", "816", "817"] {
            let (action, kept) = classify(Some(code), true, true).expect(code);
            assert_eq!(action, Action::Modify);
            assert_eq!(kept, code);
        }
        // Rubric tokens resolve to their codes (and are preserved as codes).
        let (action, kept) = classify(Some("amendment"), true, true).unwrap();
        assert_eq!((action, kept.as_str()), (Action::Modify, "250"));
        let (action, kept) = classify(Some("creation"), false, true).unwrap();
        assert_eq!((action, kept.as_str()), (Action::Create, "249"));
        let (action, kept) = classify(Some("523"), true, false).unwrap();
        assert_eq!((action, kept.as_str()), (Action::Delete, "523"));
    }

    #[test]
    fn classify_rejects_spec_invalid_combinations() {
        // creation on an existing object (RM change_control §Contributions:
        // creation commits a *new* VERSIONED_OBJECT).
        assert!(classify_err(Some("249"), true, true).contains("249|creation|"));
        // a non-creation change type on a first version.
        assert!(classify_err(Some("250"), false, true).contains("preceding_version_uid"));
        // deleted with data (spec: "data attribute is set to Void").
        assert!(classify_err(Some("523"), true, true).contains("must not carry data"));
        // deleted without a preceding version.
        assert!(classify_err(Some("523"), false, false).contains("preceding_version_uid"));
        // out-of-group token (AUDIT_DETAILS.Change_type_valid).
        assert!(classify_err(Some("999"), true, true).contains("audit_change_type"));
        // content change types need data.
        assert!(classify_err(Some("251"), true, false).contains("needs data"));
    }

    #[test]
    fn classify_attestation_of_existing_version() {
        // RM change_control §Contributions: 666 attaches an ATTESTATION to an
        // existing ORIGINAL_VERSION — a preceding_version_uid, no data.
        let (action, kept) = classify(Some("666"), true, false).unwrap();
        assert_eq!((action, kept.as_str()), (Action::Attest, "666"));
        let (action, kept) = classify(Some("attestation"), true, false).unwrap();
        assert_eq!((action, kept.as_str()), (Action::Attest, "666"));

        // No preceding_version_uid → cannot name the target → 400 (BadRequest).
        assert!(classify_bad_request(Some("666"), false, false).contains("preceding_version_uid"));
        // Carries data → 422 (attestation adds no content).
        assert!(classify_err(Some("666"), true, true).contains("adds no content"));
    }

    #[test]
    fn classify_defaults_without_a_change_type() {
        let (action, code) = classify(None, false, true).unwrap();
        assert_eq!((action, code.as_str()), (Action::Create, "249"));
        let (action, code) = classify(None, true, true).unwrap();
        assert_eq!((action, code.as_str()), (Action::Modify, "251"));
    }

    fn audit_input(system_id: &str, committer: Value) -> vobject::AuditInput {
        vobject::AuditInput {
            system_id: system_id.to_owned(),
            change_type: change_type::CREATION.to_owned(),
            description: None,
            committer,
        }
    }

    #[test]
    fn commit_audit_rejects_empty_system_id() {
        // AUDIT_DETAILS.System_id_valid: `not system_id.is_empty` — a client
        // CONTRIBUTION audit with system_id "" must be a 422, not the DB-CHECK 500.
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
        // PARTY_IDENTIFIED.Basic_validity: a committer with none of
        // name/identifiers/external_ref is invalid.
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
        // PARTY_IDENTIFIED.Name_valid: a present name must be non-empty.
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
        // PARTY_RELATED.relationship: mandatory (1..1), coded, and in the
        // openEHR `subject_relationship` group (Relationship_valid,
        // party_related.adoc). Group member 10 = "mother".
        let related = |relationship: Value| {
            let mut c = json!({ "_type": "PARTY_RELATED", "name": "Mum" });
            if !relationship.is_null() {
                c.as_object_mut()
                    .unwrap()
                    .insert("relationship".into(), relationship);
            }
            audit_input("sys", c)
        };
        // Missing relationship → 422 naming the invariant.
        match validate_commit_audit(&related(Value::Null)) {
            Err(ServiceError::Unprocessable(msg)) => {
                assert!(msg.contains("relationship"), "got {msg}");
            }
            other => panic!("missing relationship must be Unprocessable, got {other:?}"),
        }
        // Uncoded relationship → 422.
        assert!(
            validate_commit_audit(&related(json!({ "_type": "DV_TEXT", "value": "mother" })))
                .is_err(),
            "an uncoded relationship must be rejected (1..1 DV_CODED_TEXT)"
        );
        // A non-member openehr code → 422 citing Relationship_valid.
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
        // A group member (10 = mother) is accepted.
        validate_commit_audit(&related(json!({
            "_type": "DV_CODED_TEXT", "value": "mother",
            "defining_code": { "_type": "CODE_PHRASE", "code_string": "10",
                               "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" } }
        })))
        .expect("subject_relationship group member accepted");
    }

    #[test]
    fn commit_audit_accepts_valid_committers() {
        // A named PARTY_IDENTIFIED, an identifier-only PARTY_IDENTIFIED, and a
        // PARTY_SELF committer are all valid (Basic_validity satisfied / no
        // invariant on PARTY_SELF).
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

    #[test]
    fn contribution_aggregate_change_type() {
        // All members share a code → that code; a mixture → 251|modification|.
        let same = vec!["250".to_owned(), "250".to_owned()];
        assert_eq!(aggregate_change_type(&same), "250");
        let mixed = vec!["249".to_owned(), "523".to_owned()];
        assert_eq!(aggregate_change_type(&mixed), "251");
        assert_eq!(aggregate_change_type(&[]), "251");
    }
}
