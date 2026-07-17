//! CONTRIBUTION classify + commit orchestration + retrieval (S-16..S-19).
//!
//! Spec: RM common `master06-change_control_package.adoc` §Contributions,
//! §Committal and Audits + `master08-versioning.adoc` §Managing Changes. A
//! CONTRIBUTION is the change-set unit — `uid` + a `versions` list + an
//! `audit`; every change is a CONTRIBUTION, committed as a nested transaction
//! (all versions/attestations commit or none). Each version's storage action
//! **and** its preserved audit change-type code come from [`classify`]: the
//! client `commit_audit.change_type` is validated against the full openEHR
//! `audit_change_type` group and stored **verbatim** (never narrowed), while
//! the storage branch collapses to create / modify / delete / attest.

use serde_json::{Value, json};
use uuid::Uuid;

use crate::ids::{EhrId, VoId};
use crate::service::error::ServiceError;
use crate::service::list::Page;
use crate::versioning::attestation::PendingAttest;
use crate::versioning::audit::{
    AuditInput, audit_details, change_type, change_type_code, validate_commit_audit,
};
use crate::versioning::change::Change;
use crate::versioning::lifecycle::{lifecycle_state_code, state};
use crate::versioning::object_version_id::{self, TreeId};
use crate::versioning::signature::signer::Signer;
use crate::versioning::wire::original_version;
use crate::versioning::{CommitEnv, Kind, change, read};

/// An optional `(lower, upper)` inclusive commit-time window — the simple
/// realization of the SM `Interval<Iso8601_date_time>` (either side open when
/// its bound is `None`; the whole `Option` `None` = unbounded).
pub(crate) type TimeRange = Option<(Option<jiff::Timestamp>, Option<jiff::Timestamp>)>;

/// The storage branch an incoming VERSION maps to. Deliberately narrower than
/// the `audit_change_type` group: many change kinds (amendment, modification,
/// synthesis, …) are all "commit a new content version"; the audited change
/// type is carried separately, verbatim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Create,
    Modify,
    Delete,
    /// Attach an `ATTESTATION` to an existing `ORIGINAL_VERSION`
    /// (`666|attestation|`) — no new version (master06 §Contributions).
    Attest,
}

/// Classify one VERSION of a contribution: resolve (and validate) its
/// `commit_audit.change_type` to the canonical numeric `audit_change_type`
/// code, and derive the storage [`Action`], rejecting spec-invalid combinations
/// (RM common master06 §Contributions):
///
/// - *addition of new item* → a **new** `VERSIONED_OBJECT`, `249|creation|`
///   (so `249` with a preceding is invalid, and any non-`249` needs an existing
///   object);
/// - *deletion* → a new version whose data is Void, `523|deleted|` (so data
///   alongside `523` is invalid);
/// - *modification* → `250|amendment|` / `251|modification|` / `252` / `253` /
///   `816` / `817`, content-carrying commits against an existing object;
/// - *attestation* → `666|attestation|` attaches to an existing
///   `ORIGINAL_VERSION` — **not** a new version; requires a preceding and no
///   data.
///
/// # Errors
/// [`ServiceError::BadRequest`] for a change-control mismatch the ITS-REST
/// `400_CONTRIBUTION` scope covers (`249` with a preceding; `666` without one);
/// [`ServiceError::Unprocessable`] for an out-of-group token or a data/preceding
/// combination the spec forbids (data on `523`/`666`, missing data on a
/// content commit, missing preceding on a non-creation).
fn classify(
    token: Option<&str>,
    has_preceding: bool,
    has_data: bool,
) -> Result<(Action, String), ServiceError> {
    let code = match token {
        Some(t) => change_type_code(t).ok_or_else(|| {
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
                // A change-control mismatch, not content validation: ITS-REST
                // scopes `400_CONTRIBUTION` to "the modification type does not
                // match the operation" — a 400, never a 422.
                return Err(ServiceError::BadRequest(
                    "change_type 249|creation| is invalid for an existing object \
                     (preceding_version_uid present); creation commits a new \
                     VERSIONED_OBJECT (RM change_control §Contributions; ITS-REST \
                     contribution 400: modification type does not match)"
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
            // 666 attaches to an existing ORIGINAL_VERSION identified by
            // preceding_version_uid (master06 §Contributions;
            // VERSIONED_OBJECT.commit_attestation pre has_version_id). Absent →
            // the request cannot name its target: a 400, not a 422.
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

/// One parsed `UPDATE_VERSION` of a CONTRIBUTION commit — the single-pass plan
/// entry (see the parse pass in [`commit_version_set`]).
struct PlannedVersion {
    action: Action,
    /// The parsed `preceding_version_uid` target (modify/delete/attest).
    target: Option<(VoId, TreeId)>,
    /// `data` (`null` ≙ absent — the deleted-version shape).
    data: Option<Value>,
    audit: AuditInput,
    /// The raw `commit_audit` (the 666 attestation payload).
    commit_audit: Option<Value>,
    lifecycle_state: Option<String>,
    incomplete: bool,
    signature: Option<String>,
    accompanying: Vec<Value>,
    other_input_version_uids: Vec<String>,
}

/// Commit a CONTRIBUTION's version set atomically under one contribution +
/// audit, returning the new contribution id. Shared by the EHR-scoped
/// contribution path (`ehr_id = Some`, `party_only = false`) and the
/// demographic contribution path (`ehr_id = None`, `party_only = true`). Each
/// version's storage action and preserved audit change-type code come from
/// [`classify`]; the object kind from the payload `_type` (create) or the
/// stored object (modify/delete).
///
/// G-6 (register 03 — SM `i_ehr_contribution.adoc` §`commit_contribution`
/// `Pre_has_ehr`): the target EHR must exist before committing, so a create-only
/// CONTRIBUTION to a missing EHR is a clean `NotFound`, not a storage FK error.
///
/// # Errors
/// [`ServiceError::NotFound`] for a missing target EHR; the [`classify`]
/// rejections per version; [`ServiceError::Unprocessable`] for a malformed
/// CONTRIBUTION `uid`, an empty `versions` list, a scope-mismatched kind, or a
/// failed content/audit validation; [`ServiceError::BadRequest`] when a
/// body-referenced modification target does not exist (the `400_CONTRIBUTION`
/// scope); [`ServiceError::Conflict`] for a duplicate EHR singleton/directory;
/// plus the commit-engine placement/storage/signing errors.
#[allow(clippy::too_many_lines)] // the per-version classify + change-build loop
pub(crate) async fn commit_version_set(
    cx: &impl CommitEnv,
    ehr_id: Option<EhrId>,
    body: &Value,
    party_only: bool,
) -> Result<Uuid, ServiceError> {
    // `Pre_has_ehr` — the CONTRIBUTION's target EHR must exist.
    if let Some(ehr_id) = ehr_id {
        cx.ensure_ehr_exists(ehr_id).await?;
    }

    let versions = body
        .get("versions")
        .and_then(Value::as_array)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            ServiceError::Unprocessable("contribution must contain versions".to_owned())
        })?;

    // A client-supplied CONTRIBUTION uid is honoured when unused and rejected
    // when malformed or already in use (ITS-REST `contribution_create`; master06
    // §CONTRIBUTION `uid`).
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

    // master06 §Committal (m4): system_id/committer/time_committed of the
    // CONTRIBUTION audit "should be copied into the commit_audit of each VERSION
    // included in the CONTRIBUTION". We default each version's committer/system_id
    // from the CONTRIBUTION's own audit; time_committed is always the server
    // commit-act time.
    let contrib_committer = body
        .get("audit")
        .and_then(|a| a.get("committer"))
        .cloned()
        .unwrap_or_else(|| cx.default_committer());
    let contrib_system_id = body
        .get("audit")
        .and_then(|a| a.get("system_id"))
        .and_then(Value::as_str)
        .map_or_else(|| cx.effective_system_id(), str::to_owned);

    // ── ONE parse pass over the version set ────────────────────────────────
    // Each UPDATE_VERSION is read exactly once into a typed plan entry:
    // classification (master06 §Change Type), the parsed preceding target,
    // the merged per-version audit (m4 committer/system_id copy-down), the
    // lifecycle/signature/attestation envelope. The modification targets are
    // then existence/kind-checked in ONE batched statement, and the plan is
    // resolved to [`Change`]s without re-reading any JSON.
    let mut plan: Vec<PlannedVersion> = Vec::with_capacity(versions.len());
    let mut version_codes: Vec<String> = Vec::with_capacity(versions.len());
    for version in versions {
        let token = version
            .get("commit_audit")
            .and_then(|a| a.get("change_type"))
            .and_then(coded_value);
        let data = version.get("data").cloned().filter(|d| !d.is_null());
        // PORT NOTE: a first version legitimately carries no
        // `preceding_version_uid` (SM `update_version.adoc` types it `0..1`;
        // master03 common_package: "must be specified, except … a first
        // version"). The SM glue serializes a `None` preceding to JSON `null`,
        // so treat a `null` as absent — a bare `.is_some()` would misclassify
        // a spec-legal creation as a modify.
        let has_preceding = version
            .get("preceding_version_uid")
            .is_some_and(|v| !v.is_null());
        let (action, code) = classify(token.as_deref(), has_preceding, data.is_some())?;
        version_codes.push(code.clone());

        let target = if action == Action::Create {
            None
        } else {
            Some(parse_preceding(version)?)
        };
        // m4: default committer/system_id from the CONTRIBUTION audit when the
        // version item omits them (a "should be copied", so an explicit
        // per-version value is honoured — PORT NOTE: SHOULD, not MUST).
        let audit = parse_audit(
            version.get("commit_audit"),
            code,
            &contrib_committer,
            &contrib_system_id,
        );
        let lifecycle_state = lifecycle_of(version);
        // A `553|incomplete|` version gets relaxed content validation
        // (master06 §Incomplete Content).
        let incomplete = lifecycle_state
            .as_deref()
            .and_then(lifecycle_state_code)
            .is_some_and(|c| c == state::INCOMPLETE);
        plan.push(PlannedVersion {
            action,
            target,
            data,
            audit,
            commit_audit: version.get("commit_audit").cloned(),
            lifecycle_state,
            incomplete,
            // A client UPDATE_VERSION.signature is stored verbatim; absent,
            // the server signs (master06 §Digital Signature).
            signature: version
                .get("signature")
                .and_then(Value::as_str)
                .map(str::to_owned),
            accompanying: attestation_partials(version),
            // ORIGINAL_VERSION.other_input_version_uids: merge provenance
            // accepted on the wire (master06 §Version Merging).
            other_input_version_uids: version
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
                .unwrap_or_default(),
        });
    }

    // ── ONE batched target read ────────────────────────────────────────────
    let mut target_ids: Vec<VoId> = plan
        .iter()
        .filter_map(|v| v.target.map(|(vo_id, _)| vo_id))
        .collect();
    target_ids.sort_unstable();
    target_ids.dedup();
    let target_kinds: std::collections::HashMap<VoId, Kind> =
        crate::storage::version_repo::meta::object_kinds(cx.pool(), &target_ids)
            .await
            .map_err(ServiceError::from)?
            .into_iter()
            .filter_map(|(id, kind)| Kind::from_type(&kind).map(|k| (id, k)))
            .collect();
    // Every target here is **body-referenced** (`preceding_version_uid` of a
    // modification/deletion/attestation item), so a missing object is the
    // `400_CONTRIBUTION` scope — the ITS-REST `contribution_create` operation
    // declares `404` only for an unknown `ehr_id` (the URI resource), never for
    // content the committed CONTRIBUTION refers to.
    let require_kind = |vo_id: VoId| -> Result<Kind, ServiceError> {
        target_kinds.get(&vo_id).copied().ok_or_else(|| {
            ServiceError::BadRequest(format!(
                "modification target does not exist: versioned object {vo_id} \
                 (ITS-REST contribution 400 — the modification does not match \
                 a stored object)"
            ))
        })
    };

    // ── Resolve the plan to changes ────────────────────────────────────────
    let mut changes: Vec<(AuditInput, Change)> = Vec::with_capacity(plan.len());
    // 666 attestations of existing versions (committing no new version).
    let mut attests: Vec<PendingAttest> = Vec::new();
    for v in plan {
        if v.action == Action::Attest {
            let Some((vo_id, expected)) = v.target else {
                return Err(ServiceError::Internal(
                    "attest plan entry lost its parsed preceding target".to_owned(),
                ));
            };
            let kind = require_kind(vo_id)?;
            check_kind_scope(kind, party_only)?;
            let partial = v.commit_audit.ok_or_else(|| {
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
        // AUDIT_DETAILS.System_id_valid + committer PARTY invariants — a
        // client-supplied version commit_audit must be a valid RM instance.
        validate_commit_audit(&v.audit)?;
        let change = match v.action {
            Action::Create => {
                let data = v.data.ok_or_else(|| {
                    ServiceError::Unprocessable("creation version needs data".to_owned())
                })?;
                let kind = data_kind(&data)?;
                check_kind_scope(kind, party_only)?;
                // A CONTRIBUTION commit is a full commit route: its versions
                // are validated exactly as a direct create/update, relaxed for
                // a `553|incomplete|` lifecycle (master06 §Incomplete Content).
                cx.validate_for_commit(kind, &data, v.incomplete).await?;
                // An EHR holds exactly one EHR_STATUS / EHR_ACCESS (RM ehr,
                // EHR class); FOLDER hierarchies follow the CNF
                // master08-func_tc_ehr_contribution E.2 criterion.
                if let Some(ehr_id) = ehr_id {
                    reject_duplicate_singleton(cx, ehr_id, kind, &data).await?;
                }
                Change::Create {
                    kind,
                    canonical: data,
                    template_id: None,
                    signature: v.signature,
                    lifecycle_state: v.lifecycle_state,
                    attestations: v.accompanying,
                }
            }
            Action::Modify => {
                let data = v.data.ok_or_else(|| {
                    ServiceError::Unprocessable("modification version needs data".to_owned())
                })?;
                let Some((vo_id, expected)) = v.target else {
                    return Err(ServiceError::Internal(
                        "modify plan entry lost its parsed preceding target".to_owned(),
                    ));
                };
                let kind = require_kind(vo_id)?;
                check_kind_scope(kind, party_only)?;
                cx.validate_for_commit(kind, &data, v.incomplete).await?;
                Change::Modify {
                    vo_id,
                    kind,
                    canonical: data,
                    expected: Some(expected),
                    template_id: None,
                    signature: v.signature,
                    lifecycle_state: v.lifecycle_state,
                    attestations: v.accompanying,
                    other_input_version_uids: v.other_input_version_uids,
                }
            }
            Action::Delete => {
                let Some((vo_id, expected)) = v.target else {
                    return Err(ServiceError::Internal(
                        "delete plan entry lost its parsed preceding target".to_owned(),
                    ));
                };
                let kind = require_kind(vo_id)?;
                check_kind_scope(kind, party_only)?;
                Change::Delete {
                    vo_id,
                    kind,
                    expected: Some(expected),
                    signature: v.signature,
                }
            }
            Action::Attest => unreachable!("Action::Attest handled above"),
        };
        changes.push((v.audit, change));
    }

    // EHR_STATUS.is_modifiable = False forbids content writes (RM ehr master04
    // §EHR Active Status): a CONTRIBUTION that creates/modifies/deletes any EHR
    // content (everything other than the EHR_STATUS object) is refused when the
    // EHR is deactivated. An EHR_STATUS-only CONTRIBUTION stays allowed.
    if let Some(ehr_id) = ehr_id
        && changes.iter().any(|(_, c)| c.kind() != Kind::EhrStatus)
    {
        cx.ensure_content_writable(ehr_id).await?;
    }

    // The CONTRIBUTION's own audit: a client change_type is validated and
    // preserved; otherwise the spec's aggregate guidance applies (master06
    // §Contributions).
    let contribution_code = match body
        .get("audit")
        .and_then(|a| a.get("change_type"))
        .and_then(coded_value)
    {
        Some(token) => change_type_code(&token).ok_or_else(|| {
            ServiceError::Unprocessable(format!(
                "contribution audit change_type {token:?} is not a code in the \
                 openEHR audit_change_type group (AUDIT_DETAILS.Change_type_valid)"
            ))
        })?,
        None => aggregate_change_type(&version_codes),
    };
    let contribution_audit = parse_audit(
        body.get("audit"),
        contribution_code,
        &contrib_committer,
        &contrib_system_id,
    );
    validate_commit_audit(&contribution_audit)?;

    // Cross-area commit hooks — the single site of truth for the CONTRIBUTION
    // path (the direct create/update paths run the same fns inline on their own
    // write flow). Collect the EHR_STATUS bodies before `changes` is moved into
    // the commit engine, since the subject-sync hook runs after the commit.
    let status_commits: Vec<Value> = changes
        .iter()
        .filter_map(|(_, c)| match c {
            Change::Create {
                kind: Kind::EhrStatus,
                canonical,
                ..
            }
            | Change::Modify {
                kind: Kind::EhrStatus,
                canonical,
                ..
            } => Some(canonical.clone()),
            _ => None,
        })
        .collect();

    let mut tx = cx.pool().begin().await?;
    // VERSIONED_COMPOSITION cross-version invariants (RM ehr
    // `versioned_composition.adoc`) run before the write of each COMPOSITION
    // modify, in the commit tx.
    for (_, change) in &changes {
        if let Change::Modify {
            vo_id,
            kind: Kind::Composition,
            canonical,
            ..
        } = change
        {
            cx.pre_composition_modify(&mut tx, *vo_id, canonical)
                .await
                .map_err(body_target_not_found_is_bad_request)?;
        }
    }
    let (contribution_id, committed) = change::commit_contribution(
        &mut tx,
        ehr_id,
        supplied_uid,
        &contribution_audit,
        changes,
        attests,
        &cx.signing_ctx(),
    )
    .await
    .map_err(body_target_not_found_is_bad_request)?;
    // Keep the EHR's promoted subject columns in sync after each committed
    // EHR_STATUS version (one EHR per subject — RM ehr master04 §EHR Status).
    if let Some(ehr_id) = ehr_id {
        for status in &status_commits {
            cx.post_status_commit(&mut tx, ehr_id, status).await?;
        }
    }
    tx.commit().await?;

    // An EHR_ACCESS version changes the EHR's access-control policy (the
    // settings are change-controlled — RM ehr master04 §EHR Access), so drop the
    // cached settings the access gate consults per request.
    if let Some(ehr_id) = ehr_id
        && committed.iter().any(|c| c.kind == Kind::EhrAccess)
    {
        cx.invalidate_ehr_access(ehr_id).await;
    }

    Ok(contribution_id)
}

/// Reject the *creation* of a duplicate EHR structure. An EHR holds exactly
/// one `EHR_STATUS` and one `EHR_ACCESS` (RM ehr, EHR class). A FOLDER
/// creation is rejected when a LIVE hierarchy with the SAME root
/// `archetype_node_id` already exists (CNF schedule master08
/// §`commit_contribution` E.2 — creating the existing root FOLDER again is
/// negative); a distinct hierarchy joins `EHR.folders` (RM ehr master04
/// §Folders). COMPOSITIONs are unbounded.
///
/// # Errors
/// [`ServiceError::Conflict`] on a duplicate `EHR_STATUS`/`EHR_ACCESS` or an
/// already-live same-identity folder hierarchy; the `CommitEnv` lookup errors.
async fn reject_duplicate_singleton(
    cx: &impl CommitEnv,
    ehr_id: EhrId,
    kind: Kind,
    data: &Value,
) -> Result<(), ServiceError> {
    match kind {
        Kind::EhrStatus | Kind::EhrAccess => {
            if cx.current_vo(ehr_id, kind).await?.is_some() {
                return Err(ServiceError::Conflict(format!(
                    "EHR {ehr_id} already has a {}; only one is permitted (RM ehr, EHR class)",
                    kind.as_str()
                )));
            }
        }
        Kind::Folder => {
            let root = data
                .get("archetype_node_id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let name = data
                .pointer("/name/value")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if cx.folder_root_exists(ehr_id, root, name).await? {
                return Err(ServiceError::Conflict(format!(
                    "EHR {ehr_id} already has a folder hierarchy rooted at \
                     {root:?} named {name:?}; re-creating an existing directory \
                     is invalid (CNF schedule master08 §commit_contribution \
                     E.2) — commit a modification, or a hierarchy with a \
                     distinct root (RM ehr master04 §Folders)"
                )));
            }
        }
        _ => {}
    }
    Ok(())
}

/// Build an [`AuditInput`] from an ITS-REST audit object (`UpdateAudit`) and the
/// already-resolved numeric `audit_change_type` code, defaulting the
/// `committer`/`system_id` to the supplied fallbacks. Used both for the
/// CONTRIBUTION's own audit and for each VERSION's `commit_audit` — for the
/// latter the fallbacks are the enclosing CONTRIBUTION audit's values (master06
/// §Committal copy rule, m4).
fn parse_audit(
    audit: Option<&Value>,
    change_type: String,
    default_committer: &Value,
    default_system_id: &str,
) -> AuditInput {
    let description = audit
        .and_then(|a| a.get("description"))
        .and_then(|d| d.get("value"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let committer = audit
        .and_then(|a| a.get("committer"))
        .cloned()
        .unwrap_or_else(|| default_committer.clone());
    let system_id = audit
        .and_then(|a| a.get("system_id"))
        .and_then(Value::as_str)
        .map_or_else(|| default_system_id.to_owned(), str::to_owned);
    AuditInput {
        system_id,
        change_type,
        description,
        committer,
    }
}

/// The CONTRIBUTION-level aggregate change type when the client supplied none
/// (master06 §Contributions): the shared code when every member version has the
/// same change type, else `251|modification|` ("accommodates … a mixture").
fn aggregate_change_type(version_codes: &[String]) -> String {
    match version_codes.split_first() {
        Some((first, rest)) if rest.iter().all(|c| c == first) => first.clone(),
        _ => change_type::MODIFICATION.to_owned(),
    }
}

/// Enforce that a version's object kind matches the contribution's scope: a
/// demographic contribution (`party_only`) may carry only party roots +
/// `PARTY_RELATIONSHIP`, and an EHR contribution may carry neither.
///
/// # Errors
/// [`ServiceError::Unprocessable`] (`422`) on a scope mismatch in either
/// direction.
fn check_kind_scope(kind: Kind, party_only: bool) -> Result<(), ServiceError> {
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

/// The change-type code of a `DV_CODED_TEXT`: its `defining_code.code_string` if
/// present, else its `value`.
fn coded_value(dv: &Value) -> Option<String> {
    dv.get("defining_code")
        .and_then(|c| c.get("code_string"))
        .and_then(Value::as_str)
        // PORT NOTE: `UPDATE_AUDIT.change_type` is a `Terminology_code`
        // (`{terminology_id, code_string}`), not a `DV_CODED_TEXT` (SM
        // `update_audit.adoc`). The SM glue serializes it to that shape, so read
        // a top-level `code_string` too; otherwise the client's change type is
        // lost and defaults to creation/modification.
        .or_else(|| dv.get("code_string").and_then(Value::as_str))
        .or_else(|| dv.get("value").and_then(Value::as_str))
        .map(str::to_owned)
}

/// Extract a VERSION item's `lifecycle_state` token (master06 §Version
/// Lifecycle), if present. `UPDATE_VERSION.lifecycle_state` is a
/// `TerminologyCode {terminology_id, code_string}` on the wire; we also accept a
/// `DV_CODED_TEXT`, a bare `{value}`, or a plain string. `None` → the commit
/// path defaults to `532|complete|`.
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
///
/// # Errors
/// [`ServiceError::Unprocessable`] when the `_type` is not a versioned root.
fn data_kind(data: &Value) -> Result<Kind, ServiceError> {
    let rm_type = data
        .get("_type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    Kind::from_type(rm_type).ok_or_else(|| {
        ServiceError::Unprocessable(format!("not a versioned root type: {rm_type:?}"))
    })
}

/// Parse a VERSION's `preceding_version_uid` (`OBJECT_VERSION_ID`, string or
/// `{value}`) into the object id and the version it must currently be at —
/// through the strict BASE three-part parse.
///
/// # Errors
/// [`ServiceError::Unprocessable`] when absent/`null` on a modify/delete/attest
/// item, or when the value is not a valid `OBJECT_VERSION_ID`.
fn parse_preceding(version: &Value) -> Result<(VoId, TreeId), ServiceError> {
    let raw = version
        .get("preceding_version_uid")
        // Treat a JSON `null` as absent (the SM glue serializes `None` to
        // `null`) — consistent with the create-vs-modify classification.
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
    Ok(object_version_id::parse_version_uid(raw)?)
}

/// The raw wire `UPDATE_VERSION.attestations` array of a version item (partial
/// `UPDATE_ATTESTATION`s), empty when absent.
fn attestation_partials(version: &Value) -> Vec<Value> {
    version
        .get("attestations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// Retrieve a CONTRIBUTION by id (scoped to the EHR), with its audit and the
/// `OBJECT_REF`s of the versions it committed. With `resolve_refs` the
/// `versions` list carries the resolved `ORIGINAL_VERSION` objects instead of
/// `OBJECT_REF`s (ITS-REST `Prefer: resolve_refs`).
///
/// # Errors
/// [`ServiceError::NotFound`] when the CONTRIBUTION does not exist in `ehr_id`,
/// or (under `resolve_refs`) a referenced version row is gone; the storage read
/// errors; the [`original_version`] verification error under `resolve_refs`.
pub(crate) async fn get_contribution(
    pool: &sqlx::PgPool,
    signer: &Signer,
    ehr_id: EhrId,
    contribution_id: Uuid,
    resolve_refs: bool,
) -> Result<Value, ServiceError> {
    let audit = crate::storage::version_repo::contribution::contribution_audit(
        pool,
        contribution_id,
        Some(ehr_id),
    )
    .await?
    .ok_or_else(|| ServiceError::NotFound(format!("CONTRIBUTION {contribution_id}")))?;
    let time_committed = audit.time_committed;

    // CONTRIBUTION.versions lists the affected VERSION objects (master06
    // §Contributions); a 666 attestation commits no new version but still
    // affects an existing one, so the storage query unions the versions
    // referenced by this contribution's `vo_attestation` rows (dedup).
    let referenced = crate::storage::version_repo::contribution::contribution_version_refs(
        pool,
        contribution_id,
    )
    .await?;

    let mut versions = Vec::with_capacity(referenced.len());
    for (vo_id, (t, b, v), creating_system_id, kind) in referenced {
        let tree = TreeId::from_columns(t, b, v);
        if resolve_refs {
            let loaded = read::read_version(pool, vo_id, tree)
                .await?
                .ok_or_else(|| ServiceError::NotFound(format!("VERSION {vo_id}::{tree}")))?;
            versions.push(original_version(&loaded, signer)?);
        } else {
            versions.push(json!({
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": kind,
                "id": {
                    "_type": "OBJECT_VERSION_ID",
                    "value": object_version_id::object_version_id(vo_id, &creating_system_id, tree)
                }
            }));
        }
    }

    Ok(json!({
        "_type": "CONTRIBUTION",
        "uid": { "_type": "HIER_OBJECT_ID", "value": contribution_id.to_string() },
        "audit": audit_details(
            &audit.system_id,
            &audit.change_type,
            audit.description.as_deref(),
            &audit.committer,
            &time_committed,
        ),
        "versions": versions
    }))
}

/// SM `I_EHR_CONTRIBUTION.list_contributions` — the ids of the EHR's
/// CONTRIBUTIONs, oldest-first, within the optional commit-time window, paged
/// (SM `i_ehr_contribution.adoc`).
///
/// # Errors
/// [`ServiceError::NotFound`] when the EHR does not exist; the storage read
/// error of the list query.
pub(crate) async fn list_contributions(
    pool: &sqlx::PgPool,
    ehr_id: EhrId,
    time_range: TimeRange,
    page: Page,
) -> Result<Vec<Uuid>, ServiceError> {
    ensure_ehr_exists(pool, ehr_id).await?;
    let (lower, upper) = time_range.unwrap_or((None, None));
    let offset = i64::try_from(page.offset()).unwrap_or(i64::MAX);
    let limit = page.limit().map(|l| i64::try_from(l).unwrap_or(i64::MAX));
    Ok(
        crate::storage::version_repo::contribution::list_contributions(
            pool, ehr_id, lower, upper, offset, limit,
        )
        .await?,
    )
}

/// SM `I_EHR_CONTRIBUTION.contribution_count` — the number of CONTRIBUTIONs in
/// the EHR within the optional commit-time window.
///
/// # Errors
/// [`ServiceError::NotFound`] when the EHR does not exist; the storage read
/// error of the count query.
pub(crate) async fn count_contributions(
    pool: &sqlx::PgPool,
    ehr_id: EhrId,
    time_range: TimeRange,
) -> Result<i64, ServiceError> {
    ensure_ehr_exists(pool, ehr_id).await?;
    let (lower, upper) = time_range.unwrap_or((None, None));
    Ok(
        crate::storage::version_repo::contribution::count_contributions(pool, ehr_id, lower, upper)
            .await?,
    )
}

/// The EHR-existence precheck for the read paths above (SM `ehr_does_not_exist`
/// → `NotFound`).
///
/// Storage exposes the read (`version_repo::meta::ehr_exists`) so versioning stays
/// self-contained.
async fn ensure_ehr_exists(pool: &sqlx::PgPool, ehr_id: EhrId) -> Result<(), ServiceError> {
    if crate::storage::version_repo::meta::ehr_exists(pool, ehr_id).await? {
        Ok(())
    } else {
        Err(ServiceError::NotFound(format!("EHR {ehr_id}")))
    }
}

/// Inside a CONTRIBUTION commit the modification/deletion target is
/// **body-referenced** (`preceding_version_uid`), not the request URI: the
/// ITS-REST `contribution_create` operation declares `404` only for an
/// unknown `ehr_id` (the URI resource), so a missing target versioned object
/// is a change-control mismatch in the committed content — the
/// `400_CONTRIBUTION` scope — never a 404.
fn body_target_not_found_is_bad_request(e: ServiceError) -> ServiceError {
    match e {
        ServiceError::NotFound(m) => ServiceError::BadRequest(format!(
            "modification target does not exist: {m} (ITS-REST contribution 400 — \
             the modification does not match a stored object)"
        )),
        other => other,
    }
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
        for code in ["250", "251", "252", "253", "816", "817"] {
            let (action, kept) = classify(Some(code), true, true).expect(code);
            assert_eq!(action, Action::Modify);
            assert_eq!(kept, code);
        }
        let (action, kept) = classify(Some("amendment"), true, true).unwrap();
        assert_eq!((action, kept.as_str()), (Action::Modify, "250"));
        let (action, kept) = classify(Some("creation"), false, true).unwrap();
        assert_eq!((action, kept.as_str()), (Action::Create, "249"));
        let (action, kept) = classify(Some("523"), true, false).unwrap();
        assert_eq!((action, kept.as_str()), (Action::Delete, "523"));
    }

    #[test]
    fn classify_rejects_spec_invalid_combinations() {
        // creation with a preceding version is a change-control mismatch —
        // the ITS-REST `400_CONTRIBUTION` scope, not content validation.
        assert!(classify_bad_request(Some("249"), true, true).contains("249|creation|"));
        assert!(classify_err(Some("250"), false, true).contains("preceding_version_uid"));
        assert!(classify_err(Some("523"), true, true).contains("must not carry data"));
        assert!(classify_err(Some("523"), false, false).contains("preceding_version_uid"));
        assert!(classify_err(Some("999"), true, true).contains("audit_change_type"));
        assert!(classify_err(Some("251"), true, false).contains("needs data"));
    }

    #[test]
    fn classify_attestation_of_existing_version() {
        let (action, kept) = classify(Some("666"), true, false).unwrap();
        assert_eq!((action, kept.as_str()), (Action::Attest, "666"));
        let (action, kept) = classify(Some("attestation"), true, false).unwrap();
        assert_eq!((action, kept.as_str()), (Action::Attest, "666"));
        assert!(classify_bad_request(Some("666"), false, false).contains("preceding_version_uid"));
        assert!(classify_err(Some("666"), true, true).contains("adds no content"));
    }

    #[test]
    fn classify_defaults_without_a_change_type() {
        let (action, code) = classify(None, false, true).unwrap();
        assert_eq!((action, code.as_str()), (Action::Create, "249"));
        let (action, code) = classify(None, true, true).unwrap();
        assert_eq!((action, code.as_str()), (Action::Modify, "251"));
    }

    #[test]
    fn contribution_aggregate_change_type() {
        let same = vec!["250".to_owned(), "250".to_owned()];
        assert_eq!(aggregate_change_type(&same), "250");
        let mixed = vec!["249".to_owned(), "523".to_owned()];
        assert_eq!(aggregate_change_type(&mixed), "251");
        assert_eq!(aggregate_change_type(&[]), "251");
    }
}
