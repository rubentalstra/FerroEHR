//! [`DemographicService`] + [`PartyRelationshipService`] on [`EhrbaseService`]
//! — the DEMOGRAPHIC API group's trait adapters.
//!
//! Thin adapters that parse the (kind + string) arguments the `ehrbase-rest`
//! seams supply and delegate to the sibling demographic domain modules
//! ([`super::party`], [`super::relationship`], [`super::versioned`],
//! [`super::contribution`], [`super::tags`]). Party / relationship ids parse
//! through the shared BASE decoder in [`crate::versioning`]
//! (`object_version_id.rs`, re-exported).

use serde_json::Value;
use uuid::Uuid;

use crate::service::response::{ResourceMeta, ServiceResponse};
use crate::service::status::{CallStatusType, SmError};
use crate::service::demographic::types::PartyKind;
use crate::service::version_update::UpdateVersion;

use crate::service::{EhrbaseService, ServiceError};
use crate::versioning::{
    components, expected_from_if_match, parse_uid_based_id, parse_version_uid,
};

use super::party::CurrentParty;

/// Wrap a JSON array of item-tag objects as a plain (header-free) response.
fn tags_response(tags: Vec<Value>) -> ServiceResponse {
    ServiceResponse::plain(Value::Array(tags))
}

/// Parse an ISO-8601 `version_at_time` (with offset) for time-travel reads.
fn parse_at_time(raw: &str) -> Result<jiff::Timestamp, SmError> {
    raw.parse::<jiff::Timestamp>()
        .map_err(|_| SmError::precondition(format!("invalid version_at_time: {raw}")))
}

/// The `version_uid` a write produced (the new/deleted `OBJECT_VERSION_ID`),
/// pulled from the response metadata.
fn version_uid(resp: ServiceResponse) -> String {
    resp.meta.map(|m| m.uid).unwrap_or_default()
}

/// The [`PartyKind`] a commit envelope routes to, from its payload `_type`
/// (`i_party.adoc`: parties are addressed by their concrete RM type). An
/// unknown/absent `_type` is a `content_invalid` precondition failure.
fn party_kind_from_body(body: &Value) -> Result<PartyKind, SmError> {
    match body.get("_type").and_then(Value::as_str) {
        Some("AGENT") => Ok(PartyKind::Agent),
        Some("GROUP") => Ok(PartyKind::Group),
        Some("ORGANISATION") => Ok(PartyKind::Organisation),
        Some("PERSON") => Ok(PartyKind::Person),
        Some("ROLE") => Ok(PartyKind::Role),
        other => Err(SmError::new(
            CallStatusType::ContentInvalid,
            format!(
                "not a demographic party _type: {:?}",
                other.unwrap_or("<none>")
            ),
        )),
    }
}

impl EhrbaseService {
    /// Attach the party's stored `ITEM_TAG`s (RM `common.item_tag`) to a response's
    /// metadata seam ([`ResourceMeta::item_tags`]), from which the ITS-REST
    /// adapter derives the `openehr-item-tag`/`openehr-version-item-tag` response
    /// headers. A response without metadata (a deleted read → `Null` body) is
    /// left unchanged. The tags are read from the same store `party_tags_get`
    /// serves, so the header and the tags sub-resource agree.
    async fn attach_party_item_tags(
        &self,
        vo_id: Uuid,
        resp: &mut ServiceResponse,
    ) -> Result<(), SmError> {
        if resp.meta.is_none() {
            return Ok(());
        }
        let tags = self.party_tags(vo_id).await?;
        if let Some(meta) = resp.meta.as_mut() {
            meta.item_tags = Some(Value::Array(tags));
        }
        Ok(())
    }
}

/// Full-`OBJECT_VERSION_ID` `If-Match` verification (ITS-REST overview
/// §Concurrency control): the precondition names the current latest version
/// **in full** — object id + creating system id + version — and a mismatch in
/// ANY segment is a `412`. Reducing the header to the version-tree number
/// alone would accept a precondition naming a version this server never held.
/// Non-OVID tokens (a bare trunk number) keep the lenient tree addressing;
/// an absent header or an object with no current version defers to the
/// versioning path. Mirrors the EHR path's `ensure_if_match`.
fn ensure_full_ovid_if_match(
    if_match: Option<&str>,
    current: Option<&ResourceMeta>,
) -> Result<(), SmError> {
    let Some(raw) = if_match else { return Ok(()) };
    let token = raw.trim().trim_matches('"');
    if <openehr_base::prelude::ObjectVersionId as std::str::FromStr>::from_str(token).is_err() {
        return Ok(());
    }
    match current {
        Some(meta) if meta.uid == token => Ok(()),
        Some(meta) => Err(SmError::version_mismatch(format!(
            "If-Match {token:?} does not match the current latest version {:?}",
            meta.uid
        ))),
        None => Ok(()),
    }
}

impl EhrbaseService {
    // ── I_DEMOGRAPHIC_SERVICE + I_PARTY (the SM core) ───────────────────────
    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn create_party(&self, a_version: UpdateVersion) -> Result<Uuid, SmError> {
        let kind = party_kind_from_body(&a_version.data)?;
        // Reuse the wire-seam domain logic (validation + versioned create).
        let resp = self.create_party_response(kind, a_version.data).await?;
        let (vo_id, _) = parse_version_uid(&version_uid(resp))?;
        Ok(vo_id)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn has_party(&self, a_versioned_party_id: Uuid) -> Result<bool, SmError> {
        // True iff a *live* party of some kind exists (a logically deleted party
        // reads `Null`, satisfying the delete post-condition `not has_party`).
        match self.party_kind_at(a_versioned_party_id).await {
            Ok(kind) => match self
                .read_party(kind, a_versioned_party_id, None, None)
                .await
            {
                Ok(resp) => Ok(!resp.is_empty()),
                Err(ServiceError::NotFound(_)) => Ok(false),
                Err(e) => Err(e.into()),
            },
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn has_party_version_id(&self, a_party_version_id: String) -> Result<bool, SmError> {
        let Ok((vo_id, tree)) = parse_version_uid(&a_party_version_id) else {
            return Ok(false);
        };
        match self.party_version(vo_id, tree).await {
            Ok(_) => Ok(true),
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn get_party(&self, a_versioned_party_id: Uuid) -> Result<Value, SmError> {
        let kind = self.party_kind_at(a_versioned_party_id).await?;
        let resp = self
            .read_party(kind, a_versioned_party_id, None, None)
            .await?;
        if resp.is_empty() {
            // Pre `has_party` failed (deleted / no current version).
            return Err(SmError::new(
                CallStatusType::VersionedObjectDoesNotExist,
                format!("party {a_versioned_party_id} has no current version"),
            ));
        }
        Ok(resp.body)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn get_party_at_time(
        &self,
        a_versioned_party_id: Uuid,
        a_time: String,
    ) -> Result<Value, SmError> {
        let kind = self.party_kind_at(a_versioned_party_id).await?;
        let at = parse_at_time(&a_time)?;
        let resp = self
            .read_party(kind, a_versioned_party_id, None, Some(at))
            .await?;
        Ok(resp.body)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn get_party_at_version(&self, a_party_version_id: String) -> Result<Value, SmError> {
        let (vo_id, tree) = parse_version_uid(&a_party_version_id)?;
        match self.party_version(vo_id, tree).await {
            Ok(v) => Ok(v),
            // A specific version miss is `object_version_does_not_exist`.
            Err(ServiceError::NotFound(m)) => {
                Err(SmError::new(CallStatusType::ObjectVersionDoesNotExist, m))
            }
            Err(e) => Err(e.into()),
        }
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn update_party(
        &self,
        a_versioned_party_id: Uuid,
        a_version: UpdateVersion,
    ) -> Result<String, SmError> {
        let kind = party_kind_from_body(&a_version.data)?;
        let expected = match &a_version.preceding_version_uid {
            Some(ovid) => Some(components(ovid)?.1),
            None => None,
        };
        let resp = self.update_party_response(kind, a_versioned_party_id, a_version.data, expected)
            .await?;
        Ok(version_uid(resp))
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn delete_party(&self, a_versioned_party_id: Uuid) -> Result<String, SmError> {
        // The SM `delete_party` has no version argument — delete the current
        // version unconditionally.
        let kind = self.party_kind_at(a_versioned_party_id).await?;
        let resp = self.delete_party_response(kind, a_versioned_party_id, None).await?;
        Ok(version_uid(resp))
    }

    // ── PARTY CRUD (wire seam) ────────────────────────────────────────────────
    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn party_create(&self, kind: PartyKind, body: Value) -> Result<ServiceResponse, SmError> {
        let mut resp = self.create_party_response(kind, body).await?;
        // Surface the party's stored ITEM_TAGs on the response seam for the
        // `openehr-item-tag`/`openehr-version-item-tag` response headers
        // (person_create.yaml). A fresh party has none yet; the wire adapter
        // persists any request-header tags and re-populates the seam afterwards.
        if let Some(uid) = resp.meta.as_ref().map(|m| m.uid.clone()) {
            let (vo_id, _) = parse_version_uid(&uid)?;
            self.attach_party_item_tags(vo_id, &mut resp).await?;
        }
        Ok(resp)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn party_get(
        &self,
        kind: PartyKind,
        uid_based_id: String,
        version_at_time: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, version) = parse_uid_based_id(&uid_based_id)?;
        let at = version_at_time.as_deref().map(parse_at_time).transpose()?;
        let mut resp = self.read_party(kind, vo_id, version, at).await?;
        // Attach the party's stored ITEM_TAGs for the item-tag response headers
        // (person_get.yaml). A deleted read carries no metadata → no-op.
        self.attach_party_item_tags(vo_id, &mut resp).await?;
        Ok(resp)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn party_update(
        &self,
        kind: PartyKind,
        uid_based_id: String,
        if_match: String,
        body: Value,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = parse_uid_based_id(&uid_based_id)?;
        // Resolve the current version ONCE (lean, kind-checked): the same handle
        // serves the `If-Match` `ETag` compare here and the service write gate —
        // the dispatcher no longer resolves and the service again (RSJ).
        let current = self.party_current(kind, vo_id).await?;
        let meta = current.as_ref().map(CurrentParty::resource_meta);
        ensure_full_ovid_if_match(Some(&if_match), meta.as_ref())?;
        let expected = expected_from_if_match(&if_match)?;
        let current =
            current.ok_or_else(|| ServiceError::NotFound(format!("{} {vo_id}", kind.rm_type())))?;
        Ok(self.commit_party_update(current, body, expected).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn party_delete(
        &self,
        kind: PartyKind,
        uid_based_id: String,
        if_match: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        // `delete_party(a_versioned_party_id: UUID)` (our own demographic design,
        // register `docs/design/platform/04-service-demographic-ehr-index.md`):
        // the path carries the versioned-party id (bare `HIER_OBJECT_ID` or full
        // `OBJECT_VERSION_ID`). The preceding trunk version for optimistic
        // concurrency comes from `If-Match` when supplied, else the path OVID,
        // else `None` (delete the current version unconditionally).
        let (vo_id, path_version) = parse_uid_based_id(&uid_based_id)?;
        // One lean resolve shared by the `If-Match` compare and the delete gate.
        let current = self.party_current(kind, vo_id).await?;
        let meta = current.as_ref().map(CurrentParty::resource_meta);
        ensure_full_ovid_if_match(if_match.as_deref(), meta.as_ref())?;
        // A malformed `If-Match` is rejected, not silently ignored (W-14 F-12);
        // an absent header falls back to the path OVID's version.
        let expected = match if_match.as_deref() {
            Some(raw) => expected_from_if_match(raw)?.or(path_version),
            None => path_version,
        };
        let current =
            current.ok_or_else(|| ServiceError::NotFound(format!("{} {vo_id}", kind.rm_type())))?;
        Ok(self.commit_party_delete(current, expected).await?)
    }

    // ── VERSIONED_PARTY ──────────────────────────────────────────────────────
    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn versioned_party_get(
        &self,
        versioned_object_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = parse_uid_based_id(&versioned_object_uid)?;
        Ok(ServiceResponse::plain(self.versioned_party(vo_id).await?))
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn versioned_party_revision_history(
        &self,
        versioned_object_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = parse_uid_based_id(&versioned_object_uid)?;
        Ok(ServiceResponse::plain(
            self.party_revision_history(vo_id).await?,
        ))
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn versioned_party_version_get_at_time(
        &self,
        versioned_object_uid: String,
        version_at_time: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = parse_uid_based_id(&versioned_object_uid)?;
        let at = version_at_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self.party_version_at_time(vo_id, at).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn versioned_party_version_get_by_id(
        &self,
        versioned_object_uid: String,
        version_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = parse_uid_based_id(&versioned_object_uid)?;
        let (_, version) = parse_version_uid(&version_uid)?;
        Ok(ServiceResponse::plain(
            self.party_version(vo_id, version).await?,
        ))
    }

    // ── demographic CONTRIBUTION ─────────────────────────────────────────────
    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn demographic_contribution_create(
        &self,
        body: Value,
    ) -> Result<ServiceResponse, SmError> {
        Ok(self.create_demographic_contribution(body).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn demographic_contribution_get(
        &self,
        contribution_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        let id = Uuid::parse_str(&contribution_uid).map_err(|_| {
            SmError::precondition(format!("invalid contribution id: {contribution_uid}"))
        })?;
        Ok(ServiceResponse::plain(
            self.demographic_contribution(id).await?,
        ))
    }

    // ── demographic item tags ────────────────────────────────────────────────
    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn demographic_tags_get(
        &self,
        tag_key: Option<String>,
        tag_value: Option<String>,
        tag_target_path: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        let tags = self
            .demographic_tags(
                tag_key.as_deref(),
                tag_value.as_deref(),
                tag_target_path.as_deref(),
            )
            .await?;
        Ok(tags_response(tags))
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn party_tags_get(
        &self,
        _kind: PartyKind,
        uid_based_id: String,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = parse_uid_based_id(&uid_based_id)?;
        Ok(tags_response(self.party_tags(vo_id).await?))
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn party_tags_update(
        &self,
        kind: PartyKind,
        uid_based_id: String,
        body: Vec<Value>,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = parse_uid_based_id(&uid_based_id)?;
        Ok(tags_response(
            self.replace_party_tags(kind, vo_id, body).await?,
        ))
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn party_tags_delete(
        &self,
        _kind: PartyKind,
        uid_based_id: String,
        key: String,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = parse_uid_based_id(&uid_based_id)?;
        self.delete_party_tag(vo_id, &key).await?;
        Ok(ServiceResponse::plain(Value::Null))
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn demographic_latest_meta(
        &self,
        kind: PartyKind,
        uid_based_id: String,
    ) -> Result<Option<ResourceMeta>, SmError> {
        let (vo_id, _) = parse_uid_based_id(&uid_based_id)?;
        Ok(self.party_current_meta(kind, vo_id).await?)
    }
}

impl EhrbaseService {
    // ── I_PARTY_RELATIONSHIP + create factory (the SM core) ─────────────────
    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn create_party_relationship(&self, a_version: UpdateVersion) -> Result<Uuid, SmError> {
        let resp = self.create_relationship(a_version.data).await?;
        let (vo_id, _) = parse_version_uid(&version_uid(resp))?;
        Ok(vo_id)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn has_party_relationship(
        &self,
        a_versioned_party_rel_id: Uuid,
    ) -> Result<bool, SmError> {
        // True iff a *live* relationship exists (a logically deleted one reads
        // `Null`, satisfying the delete post-condition).
        match self
            .read_relationship(a_versioned_party_rel_id, None, None)
            .await
        {
            Ok(resp) => Ok(!resp.is_empty()),
            Err(ServiceError::NotFound(_)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn get_party_relationship(
        &self,
        a_versioned_party_rel_id: Uuid,
    ) -> Result<Value, SmError> {
        let resp = self
            .read_relationship(a_versioned_party_rel_id, None, None)
            .await?;
        if resp.is_empty() {
            return Err(SmError::new(
                CallStatusType::VersionedObjectDoesNotExist,
                format!("party relationship {a_versioned_party_rel_id} has no current version"),
            ));
        }
        Ok(resp.body)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn get_party_relationship_at_time(
        &self,
        a_versioned_party_rel_id: Uuid,
        a_time: String,
    ) -> Result<Value, SmError> {
        let at = parse_at_time(&a_time)?;
        let resp = self
            .read_relationship(a_versioned_party_rel_id, None, Some(at))
            .await?;
        Ok(resp.body)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn get_party_relationship_at_version(
        &self,
        a_party_rel_version_id: String,
    ) -> Result<Value, SmError> {
        let (vo_id, tree) = parse_version_uid(&a_party_rel_version_id)?;
        match self.relationship_version(vo_id, tree).await {
            Ok(v) => Ok(v),
            Err(ServiceError::NotFound(m)) => {
                Err(SmError::new(CallStatusType::ObjectVersionDoesNotExist, m))
            }
            Err(e) => Err(e.into()),
        }
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn update_party_relationship(
        &self,
        a_versioned_party_rel_id: Uuid,
        a_version: UpdateVersion,
    ) -> Result<String, SmError> {
        let expected = match &a_version.preceding_version_uid {
            Some(ovid) => Some(components(ovid)?.1),
            None => None,
        };
        let resp = self
            .update_relationship(a_versioned_party_rel_id, a_version.data, expected)
            .await?;
        Ok(version_uid(resp))
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn delete_party_relationship(
        &self,
        a_versioned_party_rel_id: Uuid,
    ) -> Result<String, SmError> {
        // The SM `delete_party_relationship` has no version argument; the domain
        // delete needs the preceding trunk version, taken from the current one.
        let meta = self
            .relationship_current_meta(a_versioned_party_rel_id)
            .await?
            .ok_or_else(|| {
                SmError::new(
                    CallStatusType::VersionedObjectDoesNotExist,
                    format!("party relationship {a_versioned_party_rel_id}"),
                )
            })?;
        let (_, tree) = parse_version_uid(&meta.uid)?;
        let resp = self
            .delete_relationship(a_versioned_party_rel_id, Some(tree))
            .await?;
        Ok(version_uid(resp))
    }

    // ── the relationship wire seam ────────────────────────────────────────────
    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn party_relationship_create(&self, body: Value) -> Result<ServiceResponse, SmError> {
        Ok(self.create_relationship(body).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn party_relationship_get(
        &self,
        uid_based_id: String,
        version_at_time: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, version) = parse_uid_based_id(&uid_based_id)?;
        let at = version_at_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self.read_relationship(vo_id, version, at).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn party_relationship_update(
        &self,
        uid_based_id: String,
        if_match: String,
        body: Value,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = parse_uid_based_id(&uid_based_id)?;
        let current = self.relationship_current_meta(vo_id).await?;
        ensure_full_ovid_if_match(Some(&if_match), current.as_ref())?;
        let expected = expected_from_if_match(&if_match)?;
        Ok(self.update_relationship(vo_id, body, expected).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn party_relationship_delete(
        &self,
        uid_based_id: String,
        if_match: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        // Mirrors `party_delete`: the path carries the versioned-relationship
        // id (bare `HIER_OBJECT_ID` or full `OBJECT_VERSION_ID`); the preceding
        // version for optimistic concurrency comes from `If-Match` when
        // supplied, else the path OVID, else `None` (delete the current
        // version — ITS-REST overview §Concurrency control).
        let (vo_id, path_version) = parse_uid_based_id(&uid_based_id)?;
        let current = self.relationship_current_meta(vo_id).await?;
        ensure_full_ovid_if_match(if_match.as_deref(), current.as_ref())?;
        // A malformed `If-Match` is rejected, not silently ignored (W-14 F-12);
        // an absent header falls back to the path OVID's version.
        let expected = match if_match.as_deref() {
            Some(raw) => expected_from_if_match(raw)?.or(path_version),
            None => path_version,
        };
        Ok(self.delete_relationship(vo_id, expected).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn versioned_party_relationship_get(
        &self,
        versioned_object_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = parse_uid_based_id(&versioned_object_uid)?;
        Ok(ServiceResponse::plain(
            self.versioned_relationship(vo_id).await?,
        ))
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn party_relationship_revision_history(
        &self,
        versioned_object_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = parse_uid_based_id(&versioned_object_uid)?;
        Ok(ServiceResponse::plain(
            self.relationship_revision_history(vo_id).await?,
        ))
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn party_relationship_version_get_at_time(
        &self,
        versioned_object_uid: String,
        version_at_time: Option<String>,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = parse_uid_based_id(&versioned_object_uid)?;
        let at = version_at_time.as_deref().map(parse_at_time).transpose()?;
        Ok(self.relationship_version_at_time(vo_id, at).await?)
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn party_relationship_version_get_by_id(
        &self,
        versioned_object_uid: String,
        version_uid: String,
    ) -> Result<ServiceResponse, SmError> {
        let (vo_id, _) = parse_uid_based_id(&versioned_object_uid)?;
        let (_, version) = parse_version_uid(&version_uid)?;
        Ok(ServiceResponse::plain(
            self.relationship_version(vo_id, version).await?,
        ))
    }

    /// See the SM interface doc for this call (module doc cites the chapter).
    ///
    /// # Errors
    /// Returns the SM call-status error ([`SmError`]-mapped at the
    /// protocol adapter) for the failure conditions of this call.
    pub async fn party_relationship_latest_meta(
        &self,
        uid_based_id: String,
    ) -> Result<Option<ResourceMeta>, SmError> {
        let (vo_id, _) = parse_uid_based_id(&uid_based_id)?;
        Ok(self.relationship_current_meta(vo_id).await?)
    }
}
