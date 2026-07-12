//! The SM EHR-core service interfaces on [`EhrbaseService`] — the
//! literal openEHR Platform Service Model call set for `I_EHR_SERVICE` /
//! `I_EHR_STATUS` / `I_EHR_COMPOSITION` / `I_EHR_DIRECTORY` /
//! `I_EHR_CONTRIBUTION`, plus the ITS-REST adapter-support extension traits
//! ([`VersionMetaAdapter`], [`ItemTagAdapter`]).
//!
//! Each method is a thin bridge onto the versioning/read machinery in the
//! sibling service modules ([`crate::service::ehr`], [`composition`](crate::service),
//! [`directory`](crate::service), [`contribution`](crate::service),
//! [`item_tag`](crate::service)): it parses the native SM argument types
//! (`Iso8601_date_time` strings → [`jiff::Timestamp`],
//! [`ObjectVersionId`] → the storage `(vo_id, version)` pair), calls the
//! internal method, and adapts the result to the SM return
//! (`UUID`/version-uid `String`, canonical [`Value`], [`EhrSummary`], …).
//! Service failures cross into [`SmError`] via `From<ServiceError>`.
//!
//! Method-name clashes between an SM trait method and an inherent internal
//! method of the same name (`create_composition`, `update_composition`,
//! `delete_composition`, `get_contribution`, …) resolve to the **inherent**
//! method by Rust's method-resolution priority; `self.<name>(…)` therefore
//! calls the internal implementation, never recurses into the trait.

use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;

use ehrbase_sm::SmError;
use ehrbase_sm::services::{
    ContributionAdapter, EhrCompositionService, EhrContributionService, EhrDirectoryService,
    EhrService, EhrStatusService, ItemTagAdapter, MultimediaAdapter, TimeRange as SmTimeRange,
    VersionMetaAdapter,
};
use ehrbase_sm::{EhrSummary, Page, ResourceMeta, SubjectRef, UpdateAudit, UpdateVersion};
use openehr_base::prelude::ObjectVersionId;

use crate::service::ehr::default_ehr_status;
use crate::service::version_id;
use crate::service::vobject::Kind;
use crate::service::{EhrbaseService, ServiceError};

/// Extract the version-uid `String` a write produced from the internal
/// [`ServiceResponse`](ehrbase_sm::ServiceResponse)'s resource metadata —
/// the value the SM `create_*`/`update_*`/`delete_*` calls return.
fn version_uid(resp: ehrbase_sm::ServiceResponse) -> Result<String, SmError> {
    resp.meta
        .map(|m| m.uid)
        .ok_or_else(|| SmError::exception("write produced no version metadata"))
}

/// Enforce the full-`OBJECT_VERSION_ID` `If-Match` precondition (F-01-09 /
/// F-02-08). The client's `preceding_version_uid` (the `If-Match` value) MUST
/// equal the resource's current latest `version_uid` **in full** — `object_id`,
/// creating-system id, and trunk version — not merely the trunk version number
/// (`parameters/If-Match`: "the existing latest `version_uid` … matches this
/// header's value"; the internal versioning path compares only the version
/// number). A mismatch is a `412` ([`SmError::version_mismatch`]).
///
/// A `None` `latest` (the resource has no current version yet) is not a mismatch
/// here — first-version / not-found semantics are handled by the versioning
/// path the caller then invokes.
fn ensure_if_match(
    preceding: Option<&ObjectVersionId>,
    latest: Option<&ResourceMeta>,
) -> Result<(), SmError> {
    let Some(pre) = preceding else {
        return Ok(());
    };
    match latest {
        Some(meta) if meta.uid == pre.value => Ok(()),
        Some(meta) => Err(SmError::version_mismatch(format!(
            "If-Match {:?} does not match the current latest version {:?}",
            pre.value, meta.uid
        ))),
        None => Ok(()),
    }
}

/// Parse an ISO-8601 `Iso8601_date_time` argument (with offset) for a
/// time-travel read; a malformed value is an argument-validity precondition
/// failure (→ `400`).
fn parse_at_time(raw: &str) -> Result<jiff::Timestamp, SmError> {
    raw.parse::<jiff::Timestamp>()
        .map_err(|_| SmError::precondition(format!("invalid version_at_time: {raw}")))
}

/// Parse the optional `Interval<Iso8601_date_time>` bounds of an SM contribution
/// `time_range` into the internal `jiff::Timestamp` pair; a malformed bound is a
/// `400`-equivalent precondition failure.
#[allow(clippy::type_complexity)]
fn parse_time_range(
    raw: SmTimeRange,
) -> Result<Option<(Option<jiff::Timestamp>, Option<jiff::Timestamp>)>, SmError> {
    let parse = |b: Option<String>| -> Result<Option<jiff::Timestamp>, SmError> {
        b.map(|s| {
            s.parse::<jiff::Timestamp>()
                .map_err(|_| SmError::precondition(format!("invalid time_range bound: {s}")))
        })
        .transpose()
    };
    raw.map(|(lo, hi)| Ok((parse(lo)?, parse(hi)?))).transpose()
}

/// Build the `EHR_STATUS` for a subject-scoped EHR creation: the base status
/// (client-supplied or the default) with its `subject` set to a `PARTY_SELF`
/// whose `external_ref` names the subject — the promoted `ehr.subject_*` columns
/// are kept in sync from `subject.external_ref.id.value`/`.namespace` on commit
/// (see `vobject::sync_subject`).
fn status_for_subject(base: Value, subject: &SubjectRef) -> Value {
    let mut status = base;
    if let Value::Object(map) = &mut status {
        map.insert(
            "subject".to_owned(),
            json!({
                "_type": "PARTY_SELF",
                "external_ref": {
                    "_type": "PARTY_REF",
                    "namespace": subject.namespace,
                    "type": subject.r#type,
                    "id": {
                        "_type": "GENERIC_ID",
                        "value": subject.id,
                        "scheme": subject.namespace
                    }
                }
            }),
        );
    }
    status
}

#[async_trait]
impl EhrService for EhrbaseService {
    async fn has_ehr(&self, ehr_id: Uuid) -> Result<bool, SmError> {
        match self.ensure_ehr_exists(ehr_id).await {
            Ok(()) => Ok(true),
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    async fn has_ehr_for_subject(&self, a_subject_id: SubjectRef) -> Result<bool, SmError> {
        match self
            .ehr_by_subject(&a_subject_id.id, &a_subject_id.namespace)
            .await
        {
            Ok(_) => Ok(true),
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    async fn create_ehr(&self, an_ehr_status: Option<Value>) -> Result<Uuid, SmError> {
        let ehr_id = Uuid::now_v7();
        let status = an_ehr_status.unwrap_or_else(default_ehr_status);
        self.create_ehr(ehr_id, status).await?;
        Ok(ehr_id)
    }

    async fn create_ehr_with_id(
        &self,
        an_ehr_id: Uuid,
        an_ehr_status: Option<Value>,
    ) -> Result<Uuid, SmError> {
        let status = an_ehr_status.unwrap_or_else(default_ehr_status);
        self.create_ehr(an_ehr_id, status).await?;
        Ok(an_ehr_id)
    }

    async fn create_ehr_for_subject(
        &self,
        a_subject_id: SubjectRef,
        an_ehr_status: Option<Value>,
    ) -> Result<Uuid, SmError> {
        let ehr_id = Uuid::now_v7();
        let status = status_for_subject(
            an_ehr_status.unwrap_or_else(default_ehr_status),
            &a_subject_id,
        );
        self.create_ehr(ehr_id, status).await?;
        Ok(ehr_id)
    }

    async fn create_ehr_for_subject_with_id(
        &self,
        an_ehr_id: Uuid,
        a_subject_id: SubjectRef,
        an_ehr_status: Option<Value>,
    ) -> Result<Uuid, SmError> {
        let status = status_for_subject(
            an_ehr_status.unwrap_or_else(default_ehr_status),
            &a_subject_id,
        );
        self.create_ehr(an_ehr_id, status).await?;
        Ok(an_ehr_id)
    }

    async fn get_ehr(&self, an_ehr_id: Uuid) -> Result<EhrSummary, SmError> {
        Ok(self.summarize_ehr(an_ehr_id).await?)
    }

    async fn get_ehrs_for_subject(
        &self,
        a_subject_id: SubjectRef,
    ) -> Result<Vec<EhrSummary>, SmError> {
        match self
            .ehr_by_subject(&a_subject_id.id, &a_subject_id.namespace)
            .await
        {
            Ok(resp) => {
                let ehr_id = resp
                    .body
                    .pointer("/ehr_id/value")
                    .and_then(Value::as_str)
                    .and_then(|v| Uuid::parse_str(v).ok())
                    .ok_or_else(|| SmError::exception("EHR body carries no ehr_id"))?;
                Ok(vec![self.summarize_ehr(ehr_id).await?])
            }
            Err(ServiceError::NotFound(_)) => Ok(Vec::new()),
            Err(e) => Err(e.into()),
        }
    }

    async fn ehr_object(&self, an_ehr_id: Uuid) -> Result<Value, SmError> {
        Ok(self.ehr_summary(an_ehr_id).await?.body)
    }

    async fn ehr_object_for_subject(
        &self,
        subject_id: &str,
        subject_namespace: &str,
    ) -> Result<Value, SmError> {
        Ok(self
            .ehr_by_subject(subject_id, subject_namespace)
            .await?
            .body)
    }
}

#[async_trait]
impl EhrStatusService for EhrbaseService {
    async fn has_ehr_status_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: Uuid,
    ) -> Result<bool, SmError> {
        // An EHR holds exactly one EHR_STATUS versioned object; the version
        // exists iff that object's `vo_id` matches.
        Ok(self
            .current_vo(an_ehr_id, Kind::EhrStatus)
            .await?
            .is_some_and(|(vo, _)| vo == a_version_uid))
    }

    async fn get_ehr_status(&self, an_ehr_id: Uuid) -> Result<Value, SmError> {
        Ok(self.status_at(an_ehr_id, None).await?.body)
    }

    async fn get_ehr_status_at_time(
        &self,
        an_ehr_id: Uuid,
        a_time: Option<String>,
    ) -> Result<Value, SmError> {
        let at = a_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self.status_at(an_ehr_id, at).await?.body)
    }

    async fn get_ehr_status_at_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: Uuid,
        a_version: &str,
    ) -> Result<Value, SmError> {
        let tree = version_id::parse_tree_id(a_version)?;
        // F-01-03: the bare EHR_STATUS at that version, not an ORIGINAL_VERSION.
        Ok(self
            .status_by_version(an_ehr_id, a_version_uid, tree)
            .await?
            .body)
    }

    async fn get_versioned_ehr_status(&self, an_ehr_id: Uuid) -> Result<Value, SmError> {
        Ok(self.versioned_status(an_ehr_id).await?)
    }

    async fn replace_ehr_status(
        &self,
        an_ehr_id: Uuid,
        a_status: UpdateVersion,
    ) -> Result<String, SmError> {
        let latest = self.ehr_status_meta(an_ehr_id).await?;
        ensure_if_match(a_status.preceding_version_uid.as_ref(), latest.as_ref())?;
        let if_match = a_status
            .preceding_version_uid
            .map(|o| o.value)
            .unwrap_or_default();
        version_uid(
            self.status_update(an_ehr_id, a_status.data, &if_match)
                .await?,
        )
    }

    async fn ehr_status_revision_history(&self, an_ehr_id: Uuid) -> Result<Value, SmError> {
        Ok(self.status_revision_history(an_ehr_id).await?)
    }

    async fn ehr_status_version_at_time(
        &self,
        an_ehr_id: Uuid,
        a_time: Option<String>,
    ) -> Result<Value, SmError> {
        // F-01-05: the ORIGINAL_VERSION extant at `a_time` (or the latest).
        let at = a_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self.status_version_at_time(an_ehr_id, at).await?.body)
    }

    async fn ehr_status_original_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: Uuid,
        a_version: &str,
    ) -> Result<Value, SmError> {
        let tree = version_id::parse_tree_id(a_version)?;
        Ok(self.status_version(an_ehr_id, a_version_uid, tree).await?)
    }
}

#[async_trait]
impl EhrCompositionService for EhrbaseService {
    async fn has_composition(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: ObjectVersionId,
    ) -> Result<bool, SmError> {
        let (vo_id, version) = version_id::components(&a_version_uid)?;
        match self.read_composition(an_ehr_id, vo_id, Some(version)).await {
            Ok(_) => Ok(true),
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    async fn get_composition_latest(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
    ) -> Result<Value, SmError> {
        Ok(self
            .read_composition(an_ehr_id, a_versioned_object_uid, None)
            .await?
            .body)
    }

    async fn get_composition_at_time(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
        a_time: Option<String>,
    ) -> Result<Value, SmError> {
        match a_time.as_deref() {
            None => Ok(self
                .read_composition(an_ehr_id, a_versioned_object_uid, None)
                .await?
                .body),
            Some(raw) => Ok(self
                .composition_at_time(an_ehr_id, a_versioned_object_uid, parse_at_time(raw)?)
                .await?
                .body),
        }
    }

    async fn get_composition_at_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: ObjectVersionId,
    ) -> Result<Value, SmError> {
        let (vo_id, version) = version_id::components(&a_version_uid)?;
        Ok(self
            .read_composition(an_ehr_id, vo_id, Some(version))
            .await?
            .body)
    }

    async fn get_versioned_composition(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
    ) -> Result<Value, SmError> {
        Ok(self
            .versioned_composition(an_ehr_id, a_versioned_object_uid)
            .await?)
    }

    async fn create_composition(
        &self,
        an_ehr_id: Uuid,
        a_comp: UpdateVersion,
    ) -> Result<String, SmError> {
        // Inherent `create_composition` (Value) — see the module note.
        version_uid(self.create_composition(an_ehr_id, a_comp.data).await?)
    }

    async fn update_composition(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
        a_comp: UpdateVersion,
    ) -> Result<String, SmError> {
        let latest = self
            .composition_current_meta(an_ehr_id, a_versioned_object_uid)
            .await?;
        ensure_if_match(a_comp.preceding_version_uid.as_ref(), latest.as_ref())?;
        let expected = a_comp
            .preceding_version_uid
            .as_ref()
            .map(|o| version_id::components(o).map(|(_, v)| v))
            .transpose()?;
        version_uid(
            self.update_composition(an_ehr_id, a_versioned_object_uid, a_comp.data, expected)
                .await?,
        )
    }

    async fn delete_composition(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: ObjectVersionId,
    ) -> Result<String, SmError> {
        let (vo_id, version) = version_id::components(&a_version_uid)?;
        version_uid(self.delete_composition(an_ehr_id, vo_id, version).await?)
    }

    async fn composition_revision_history(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
    ) -> Result<Value, SmError> {
        Ok(self
            .revision_history(an_ehr_id, a_versioned_object_uid)
            .await?)
    }

    async fn composition_version_at_time(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
        a_time: Option<String>,
    ) -> Result<Value, SmError> {
        let at = a_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self
            .composition_version_at_time(an_ehr_id, a_versioned_object_uid, at)
            .await?
            .body)
    }

    async fn composition_original_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: ObjectVersionId,
    ) -> Result<Value, SmError> {
        let (vo_id, version) = version_id::components(&a_version_uid)?;
        Ok(self.composition_version(an_ehr_id, vo_id, version).await?)
    }
}

#[async_trait]
impl EhrDirectoryService for EhrbaseService {
    async fn has_directory(&self, an_ehr_id: Uuid) -> Result<bool, SmError> {
        // `EHR.directory` (= `folders[1]`) — the lowest-rank folder hierarchy
        // (RM ehr §EHR Class `Directory_in_folders`); an EHR may index several
        // hierarchies (RM ehr master04 §Folders), so resolve the directory slot
        // rather than assuming a single FOLDER versioned object.
        Ok(self.directory_vo_opt(an_ehr_id).await?.is_some())
    }

    async fn has_path(&self, an_ehr_id: Uuid, a_path: String) -> Result<bool, SmError> {
        match self.directory_at_time(an_ehr_id, None, Some(&a_path)).await {
            Ok(resp) => Ok(!resp.body.is_null()),
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    async fn create_directory(
        &self,
        an_ehr_id: Uuid,
        a_dir_struct: UpdateVersion,
    ) -> Result<String, SmError> {
        version_uid(self.create_directory(an_ehr_id, a_dir_struct.data).await?)
    }

    async fn get_directory_at_time(
        &self,
        an_ehr_id: Uuid,
        a_time: Option<String>,
        a_path: Option<String>,
    ) -> Result<Value, SmError> {
        let at = a_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self
            .directory_at_time(an_ehr_id, at, a_path.as_deref())
            .await?
            .body)
    }

    async fn update_directory(
        &self,
        an_ehr_id: Uuid,
        a_dir_struct: UpdateVersion,
    ) -> Result<String, SmError> {
        let latest = self.directory_meta(an_ehr_id).await?;
        ensure_if_match(a_dir_struct.preceding_version_uid.as_ref(), latest.as_ref())?;
        let expected = a_dir_struct
            .preceding_version_uid
            .as_ref()
            .map(|o| version_id::components(o).map(|(_, v)| v))
            .transpose()?;
        version_uid(
            self.update_directory(an_ehr_id, a_dir_struct.data, expected)
                .await?,
        )
    }

    async fn delete_directory(
        &self,
        an_ehr_id: Uuid,
        preceding_version_uid: Option<ObjectVersionId>,
    ) -> Result<(), SmError> {
        let latest = self.directory_meta(an_ehr_id).await?;
        ensure_if_match(preceding_version_uid.as_ref(), latest.as_ref())?;
        let expected = preceding_version_uid
            .as_ref()
            .map(|o| version_id::components(o).map(|(_, v)| v))
            .transpose()?;
        self.delete_directory(an_ehr_id, expected).await?;
        Ok(())
    }

    async fn get_directory_at_version(
        &self,
        an_ehr_id: Uuid,
        a_version_uid: ObjectVersionId,
    ) -> Result<Value, SmError> {
        let (vo_id, version) = version_id::components(&a_version_uid)?;
        Ok(self
            .directory_version(an_ehr_id, vo_id, version)
            .await?
            .body)
    }
}

#[async_trait]
impl EhrContributionService for EhrbaseService {
    async fn has_contribution(&self, an_ehr_id: Uuid, a_contrib_id: Uuid) -> Result<bool, SmError> {
        match self.get_contribution(an_ehr_id, a_contrib_id).await {
            Ok(_) => Ok(true),
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    async fn get_contribution(
        &self,
        an_ehr_id: Uuid,
        a_contrib_id: Uuid,
    ) -> Result<Value, SmError> {
        // Inherent `get_contribution` (returns the CONTRIBUTION Value).
        Ok(self.get_contribution(an_ehr_id, a_contrib_id).await?)
    }

    async fn get_contribution_resolved(
        &self,
        an_ehr_id: Uuid,
        a_contrib_id: Uuid,
    ) -> Result<Value, SmError> {
        Ok(self
            .get_contribution_resolved(an_ehr_id, a_contrib_id)
            .await?)
    }

    async fn commit_contribution(
        &self,
        an_ehr_id: Uuid,
        versions: Vec<UpdateVersion>,
        an_audit: UpdateAudit,
    ) -> Result<String, SmError> {
        // Reassemble the wire CONTRIBUTION body `commit_version_set` parses:
        // `{ versions: [ { commit_audit, data, preceding_version_uid,
        // lifecycle_state, attestations, signature } … ], audit }`. The typed
        // `UpdateVersion`/`UpdateAudit` serialize to exactly those field names
        // (`commit_audit` is the serde-renamed audit field).
        //
        // PORT NOTE: this typed → wire-JSON → re-parse round-trip through
        // `commit_version_set` is a known glue seam. The typed shapes differ from
        // the raw wire in two ways `commit_version_set` now tolerates explicitly:
        // `preceding_version_uid: None` serializes to JSON `null` (not absent),
        // and `change_type` is a `Terminology_code` (`{terminology_id,
        // code_string}`), not a `DV_CODED_TEXT` (see the PORT NOTEs in
        // `service/contribution.rs` `coded_value`/`classify`). A future cleanup
        // would give the contribution commit a native typed path that skips the
        // JSON round-trip entirely; do not refactor it here.
        let versions_json =
            serde_json::to_value(&versions).map_err(|e| SmError::exception(e.to_string()))?;
        let audit_json =
            serde_json::to_value(&an_audit).map_err(|e| SmError::exception(e.to_string()))?;
        let body = json!({ "versions": versions_json, "audit": audit_json });
        let id = self
            .commit_version_set(Some(an_ehr_id), &body, false)
            .await?;
        Ok(id.to_string())
    }

    async fn list_contributions(
        &self,
        an_ehr_id: Uuid,
        time_range: SmTimeRange,
        page: Page,
    ) -> Result<Vec<String>, SmError> {
        let time_range = parse_time_range(time_range)?;
        let ids = self.list_contributions(an_ehr_id, time_range, page).await?;
        Ok(ids.iter().map(Uuid::to_string).collect())
    }

    async fn contribution_count(
        &self,
        an_ehr_id: Uuid,
        time_range: SmTimeRange,
    ) -> Result<i64, SmError> {
        let time_range = parse_time_range(time_range)?;
        Ok(self.count_contributions(an_ehr_id, time_range).await?)
    }
}

#[async_trait]
impl ContributionAdapter for EhrbaseService {
    async fn ehr_contribution_commit(
        &self,
        an_ehr_id: Uuid,
        a_contribution: Value,
    ) -> Result<ehrbase_sm::ServiceResponse, SmError> {
        self.create_ehr_contribution(an_ehr_id, a_contribution)
            .await
    }
}

#[async_trait]
impl MultimediaAdapter for EhrbaseService {
    async fn expand_multimedia(&self, body: Value) -> Result<Value, SmError> {
        // Off by default: no engine ⇒ serve the stored form unchanged.
        let Some(engine) = &self.multimedia else {
            return Ok(body);
        };
        let mut body = body;
        engine
            .expand(&mut body)
            .await
            .map_err(|e| SmError::exception(e.to_string()))?;
        Ok(body)
    }
}

#[async_trait]
impl VersionMetaAdapter for EhrbaseService {
    async fn composition_latest_meta(
        &self,
        an_ehr_id: Uuid,
        a_versioned_object_uid: Uuid,
    ) -> Result<Option<ResourceMeta>, SmError> {
        Ok(self
            .composition_current_meta(an_ehr_id, a_versioned_object_uid)
            .await?)
    }

    async fn ehr_status_latest_meta(
        &self,
        an_ehr_id: Uuid,
    ) -> Result<Option<ResourceMeta>, SmError> {
        Ok(self.ehr_status_meta(an_ehr_id).await?)
    }

    async fn directory_latest_meta(
        &self,
        an_ehr_id: Uuid,
    ) -> Result<Option<ResourceMeta>, SmError> {
        Ok(self.directory_meta(an_ehr_id).await?)
    }
}

#[async_trait]
impl ItemTagAdapter for EhrbaseService {
    async fn ehr_tags_get(
        &self,
        an_ehr_id: Uuid,
        key: Option<String>,
        value: Option<String>,
        target_path: Option<String>,
    ) -> Result<Vec<Value>, SmError> {
        Ok(self
            .ehr_tags(
                an_ehr_id,
                key.as_deref(),
                value.as_deref(),
                target_path.as_deref(),
            )
            .await?)
    }

    async fn target_tags_get(
        &self,
        an_ehr_id: Uuid,
        uid_based_id: String,
    ) -> Result<Vec<Value>, SmError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&uid_based_id)?;
        Ok(self.target_tags(an_ehr_id, vo_id).await?)
    }

    async fn target_tags_replace(
        &self,
        an_ehr_id: Uuid,
        uid_based_id: String,
        target_type: &str,
        tags: Vec<Value>,
    ) -> Result<Vec<Value>, SmError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&uid_based_id)?;
        Ok(self
            .replace_tags(an_ehr_id, vo_id, target_type, tags)
            .await?)
    }

    async fn target_tag_delete(
        &self,
        an_ehr_id: Uuid,
        uid_based_id: String,
        key: String,
    ) -> Result<(), SmError> {
        let (vo_id, _) = version_id::parse_uid_based_id(&uid_based_id)?;
        self.delete_tag(an_ehr_id, vo_id, &key).await?;
        Ok(())
    }
}
