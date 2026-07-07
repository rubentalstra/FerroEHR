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

use ehrbase_rest::{ResourceMeta, ServiceResponse};
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use super::codes::{self, change_type};
use super::vobject::{self, AuditInput, Change, Kind};
use super::{EhrbaseService, ServiceError, version_id};

/// The storage branch an incoming VERSION maps to. This is deliberately
/// narrower than the `audit_change_type` group: many change kinds (amendment,
/// modification, synthesis, …) are all "commit a new content version"; the
/// audited change type is carried separately, verbatim (F-06-06).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Create,
    Modify,
    Delete,
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
/// - *attestation* → an `ATTESTATION` attached to an **existing**
///   `ORIGINAL_VERSION`, not a version commit. PORT NOTE: attestations are
///   out of Stage-1 scope (no `ATTESTATION` storage — F-06-10), so `666` is
///   rejected on this surface rather than silently coerced to a modify.
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
        change_type::ATTESTATION => Err(ServiceError::Unprocessable(
            "change_type 666|attestation| is not a version commit — an ATTESTATION \
             attaches to an existing ORIGINAL_VERSION (attestations are not \
             supported in Stage 1)"
                .to_owned(),
        )),
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

impl EhrbaseService {
    /// Commit a CONTRIBUTION: apply its set of VERSIONs atomically (one
    /// contribution + audit, each version its own commit audit), then return the
    /// created CONTRIBUTION. Each version's storage action and preserved audit
    /// change-type code come from [`classify`]; the object kind from the payload
    /// `_type` (create) or the stored object (modify/delete).
    pub(super) async fn create_contribution(
        &self,
        ehr_id: Uuid,
        body: Value,
    ) -> Result<ServiceResponse, ServiceError> {
        self.ensure_ehr_exists(ehr_id).await?;
        let versions = body
            .get("versions")
            .and_then(Value::as_array)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                ServiceError::Unprocessable("contribution must contain versions".to_owned())
            })?;

        let mut changes: Vec<(AuditInput, Change)> = Vec::with_capacity(versions.len());
        let mut version_codes: Vec<String> = Vec::with_capacity(versions.len());
        for version in versions {
            let token = version
                .get("commit_audit")
                .and_then(|a| a.get("change_type"))
                .and_then(coded_value);
            // A JSON `"data": null` is "no data" (the deleted-version shape).
            let data = version.get("data").cloned().filter(|d| !d.is_null());
            let (action, code) = classify(
                token.as_deref(),
                version.get("preceding_version_uid").is_some(),
                data.is_some(),
            )?;
            let version_audit = self.parse_audit(version.get("commit_audit"), code.clone());
            version_codes.push(code);
            let change = match action {
                Action::Create => {
                    let data = data.ok_or_else(|| {
                        ServiceError::Unprocessable("creation version needs data".to_owned())
                    })?;
                    let kind = data_kind(&data)?;
                    // A CONTRIBUTION commit is a full commit route: its versions
                    // are validated exactly as a direct create/update (F-07-01).
                    self.validate_for_commit(kind, &data).await?;
                    Change::Create {
                        kind,
                        canonical: data,
                        template_id: None,
                    }
                }
                Action::Modify => {
                    let data = data.ok_or_else(|| {
                        ServiceError::Unprocessable("modification version needs data".to_owned())
                    })?;
                    let (vo_id, expected) = parse_preceding(version)?;
                    let kind = self.require_kind(vo_id).await?;
                    self.validate_for_commit(kind, &data).await?;
                    Change::Modify {
                        vo_id,
                        kind,
                        canonical: data,
                        expected: Some(expected),
                        template_id: None,
                    }
                }
                Action::Delete => {
                    let (vo_id, expected) = parse_preceding(version)?;
                    let kind = self.require_kind(vo_id).await?;
                    Change::Delete {
                        vo_id,
                        kind,
                        expected: Some(expected),
                    }
                }
            };
            changes.push((version_audit, change));
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

        let mut tx = self.pool.begin().await?;
        let (contribution_id, _) =
            vobject::commit_contribution(&mut tx, ehr_id, &contribution_audit, changes).await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);

        let body = self.get_contribution(ehr_id, contribution_id).await?;
        // 201_CONTRIBUTION: ETag(contribution_uid) + Location.
        let meta = ResourceMeta::new(ehr_id.to_string(), contribution_id.to_string());
        Ok(ServiceResponse::new(body, meta))
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
            .map_or_else(|| self.system_id.clone(), str::to_owned);
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

        let version_rows = sqlx::query(
            "SELECT vo_id, sys_version, kind FROM vo_version WHERE contribution_id = $1 \
             ORDER BY vo_id",
        )
        .bind(contribution_id)
        .fetch_all(&self.pool)
        .await?;

        let versions: Vec<Value> = version_rows
            .iter()
            .map(|row| -> Result<Value, ServiceError> {
                let vo_id: Uuid = row.try_get("vo_id")?;
                let sys_version: i32 = row.try_get("sys_version")?;
                let kind: String = row.try_get("kind")?;
                Ok(json!({
                    "_type": "OBJECT_REF",
                    "namespace": "local",
                    "type": kind,
                    "id": {
                        "_type": "OBJECT_VERSION_ID",
                        "value": self.object_version_id(vo_id, sys_version)
                    }
                }))
            })
            .collect::<Result<_, _>>()?;

        Ok(json!({
            "_type": "CONTRIBUTION",
            "uid": { "_type": "HIER_OBJECT_ID", "value": contribution_id.to_string() },
            "audit": Self::audit_details(&system_id, &change_type, description.as_deref(), &committer, &time_committed),
            "versions": versions
        }))
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

/// The change-type code of a `DV_CODED_TEXT`: its `defining_code.code_string`
/// if present, else its `value`.
fn coded_value(dv: &Value) -> Option<String> {
    dv.get("defining_code")
        .and_then(|c| c.get("code_string"))
        .and_then(Value::as_str)
        .or_else(|| dv.get("value").and_then(Value::as_str))
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
fn parse_preceding(version: &Value) -> Result<(Uuid, i32), ServiceError> {
    let raw = version
        .get("preceding_version_uid")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn classify_err(token: Option<&str>, has_preceding: bool, has_data: bool) -> String {
        match classify(token, has_preceding, has_data) {
            Err(ServiceError::Unprocessable(msg)) => msg,
            other => panic!("expected Unprocessable, got {other:?}"),
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
        // attestation is not a version commit (Stage-1 scope, F-06-10).
        assert!(classify_err(Some("666"), true, true).contains("attestation"));
        // out-of-group token (AUDIT_DETAILS.Change_type_valid).
        assert!(classify_err(Some("999"), true, true).contains("audit_change_type"));
        // content change types need data.
        assert!(classify_err(Some("251"), true, false).contains("needs data"));
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
        // All members share a code → that code; a mixture → 251|modification|.
        let same = vec!["250".to_owned(), "250".to_owned()];
        assert_eq!(aggregate_change_type(&same), "250");
        let mixed = vec!["249".to_owned(), "523".to_owned()];
        assert_eq!(aggregate_change_type(&mixed), "251");
        assert_eq!(aggregate_change_type(&[]), "251");
    }
}
