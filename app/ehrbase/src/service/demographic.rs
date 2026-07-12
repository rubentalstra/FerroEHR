//! DEMOGRAPHIC (PARTY) domain logic, built on the shared
//! [`vobject`](super::vobject) versioned-object machinery — the same code path
//! as COMPOSITION / `EHR_STATUS`, but with **no EHR scope** (`ehr_id = None`,
//! by design). Parties (PERSON / ORGANISATION / GROUP / AGENT / ROLE) are
//! versioned objects in the demographics repository.
//!
//! ITS-REST 1.0.3 defines no demographic wire contract (the SM demographic
//! service is abstract; the CNF demographic schedule — master10 — is all TBD;
//! CNF profiles list demographic as OPTIONS-profile only). This behaviour is
//! therefore our own extension **by analogy with the EHR group**: identical
//! status/`ETag`/`Location`/`Prefer`/`If-Match`/deleted-read semantics.
//!
//! Spec grounding for the RM-level rules enforced here:
//! `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.demographic.party.adoc`
//! (PARTY invariants `Identities_valid` — `not identities.is_empty` — and
//! `Uid_mandatory` — `uid /= Void`, satisfied by injecting the `uid` on read as
//! the COMPOSITION service does) and `SM/docs/UML/classes/i_party.adoc` /
//! `i_demographic_service.adoc` (the abstract demographic operations).

use ehrbase_rest::{ResourceMeta, ServiceResponse};
use ehrbase_sm::types::PartyKind;
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use super::codes::change_type;
use super::version_id::TreeId;
use super::vobject::{self, Kind, VersionRead};
use super::{EhrbaseService, ServiceError};

/// The versioned-object [`Kind`] for a REST [`PartyKind`].
fn kind_of(kind: PartyKind) -> Kind {
    match kind {
        PartyKind::Agent => Kind::Agent,
        PartyKind::Group => Kind::Group,
        PartyKind::Organisation => Kind::Organisation,
        PartyKind::Person => Kind::Person,
        PartyKind::Role => Kind::Role,
    }
}

/// Structurally validate a candidate party body of concrete RM type `rm_type`:
/// deserialize into the corresponding `openehr_rm` demographic type (a type
/// mismatch → `422`) and enforce the PARTY invariant `Identities_valid`
/// (`not identities.is_empty`). `Uid_mandatory` is met by the server injecting
/// the `uid` on read (mirroring the COMPOSITION service), so an incoming body
/// need not carry one.
fn typed_check(rm_type: &str, data: &Value) -> Result<(), ServiceError> {
    use openehr_rm::prelude::{Agent, Group, Organisation, Person, Role};
    let typed = match rm_type {
        "AGENT" => serde_json::from_value::<Agent>(data.clone()).map(drop),
        "GROUP" => serde_json::from_value::<Group>(data.clone()).map(drop),
        "ORGANISATION" => serde_json::from_value::<Organisation>(data.clone()).map(drop),
        "PERSON" => serde_json::from_value::<Person>(data.clone()).map(drop),
        "ROLE" => serde_json::from_value::<Role>(data.clone()).map(drop),
        other => {
            return Err(ServiceError::Unprocessable(format!(
                "not a demographic party type: {other:?}"
            )));
        }
    };
    typed.map_err(|e| {
        ServiceError::Unprocessable(format!("body does not validate as {rm_type}: {e}"))
    })?;
    // PARTY invariant `Identities_valid`: `not identities.is_empty`.
    let has_identities = data
        .get("identities")
        .and_then(Value::as_array)
        .is_some_and(|a| !a.is_empty());
    if !has_identities {
        return Err(ServiceError::Unprocessable(format!(
            "{rm_type} violates PARTY invariant Identities_valid: identities must be non-empty"
        )));
    }
    // "Present implies non-empty" list invariants — only checkable on the raw
    // JSON (post-deserialize an absent and a present-empty list are the same
    // Vec): PARTY.Contacts_valid + Relationships_validity (party.adoc),
    // ACTOR.Roles_valid (actor.adoc), ROLE.Capabilities_valid (role.adoc).
    for (attr, invariant) in [
        ("contacts", "Contacts_valid"),
        ("relationships", "Relationships_validity"),
        ("roles", "Roles_valid"),
        ("capabilities", "Capabilities_valid"),
    ] {
        if data
            .get(attr)
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        {
            return Err(ServiceError::Unprocessable(format!(
                "{rm_type}.{attr} is present but empty — a present list must be \
                 non-empty ({invariant})"
            )));
        }
    }
    // Relationships_validity, second arm (party.adoc): every inline
    // relationship's `source` must reference THIS party. The party's identity
    // is its `uid` (copied from the version container); when the body carries
    // one, an inline relationship pointing at another source is invalid.
    if let (Some(uid), Some(relationships)) = (
        data.pointer("/uid/value").and_then(Value::as_str),
        data.get("relationships").and_then(Value::as_array),
    ) {
        for (i, rel) in relationships.iter().enumerate() {
            let source = rel.pointer("/source/id/value").and_then(Value::as_str);
            if source.is_some_and(|s| !s.eq_ignore_ascii_case(uid)) {
                return Err(ServiceError::Unprocessable(format!(
                    "{rm_type}.relationships[{i}].source must reference this party \
                     (uid {uid}) — relationships are stored under their source \
                     (PARTY.Relationships_validity)"
                )));
            }
        }
    }
    Ok(())
}

impl EhrbaseService {
    /// Validate a party body for a create/update: its root `_type` must equal
    /// the routed [`PartyKind`]'s RM type (mismatch → `422` naming both), then
    /// the structural + invariant checks of [`typed_check`].
    fn validate_party_body(kind: PartyKind, body: &Value) -> Result<(), ServiceError> {
        let declared = body.get("_type").and_then(Value::as_str);
        if declared != Some(kind.rm_type()) {
            return Err(ServiceError::Unprocessable(format!(
                "party _type mismatch: the {} endpoint requires _type {:?}, got {:?}",
                kind.segment(),
                kind.rm_type(),
                declared.unwrap_or("<none>"),
            )));
        }
        typed_check(kind.rm_type(), body)
    }

    /// Validate a party version reached through the contribution path, where the
    /// [`Kind`] was already derived from the payload `_type` (so only the
    /// structural + invariant checks remain). Called from
    /// [`validate_for_commit`](Self::validate_for_commit).
    pub(super) fn validate_party_kind_for_commit(
        kind: Kind,
        data: &Value,
    ) -> Result<(), ServiceError> {
        typed_check(kind.as_str(), data)
    }

    // ── PARTY CRUD ───────────────────────────────────────────────────────────

    /// Create a party, returning it with its `uid` set and the version metadata
    /// (the `ETag`/`Location` for the create response).
    pub(super) async fn create_party(
        &self,
        kind: PartyKind,
        body: Value,
    ) -> Result<ServiceResponse, ServiceError> {
        Self::validate_party_body(kind, &body)?;

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::CREATION, "PARTY creation");
        let committed = vobject::create(
            &mut tx,
            None,
            kind_of(kind),
            body,
            None,
            &audit,
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);

        self.read_party(kind, committed.vo_id, Some(committed.tree), None)
            .await
    }

    /// Retrieve a party by its versioned-object id, optionally at a specific
    /// version (`version`) or instant (`at`; else the latest). A deleted current
    /// version resolves to `Value::Null` (→ `204`, mirroring COMPOSITION). A
    /// wrong-kind object (a PERSON under the `agent` route, or a COMPOSITION) is
    /// `404`.
    pub(super) async fn read_party(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
        version: Option<TreeId>,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = self.load_party_version(kind, vo_id, version, at).await?;
        if read.deleted() {
            return Ok(ServiceResponse::plain(Value::Null));
        }
        Ok(self.party_version_response(vo_id, read))
    }

    /// Commit a new party version. `expected` (from `If-Match`) enforces
    /// optimistic concurrency (a stale precondition → version conflict → `412`).
    pub(super) async fn update_party(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
        body: Value,
        expected: Option<TreeId>,
    ) -> Result<ServiceResponse, ServiceError> {
        self.ensure_party(kind, vo_id).await?;
        Self::validate_party_body(kind, &body)?;

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::MODIFICATION, "PARTY update");
        let committed = vobject::update(
            &mut tx,
            None,
            vo_id,
            kind_of(kind),
            body,
            expected,
            None,
            &audit,
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);

        self.read_party(kind, vo_id, Some(committed.tree), None)
            .await
    }

    /// Logically delete a party (a new `523|deleted|` version). `expected` is
    /// the caller-supplied trunk version (from `If-Match` or the path
    /// `OBJECT_VERSION_ID`); when `Some`, a mismatch with the current version →
    /// `409`, when `None` the current version is deleted unconditionally
    /// (SM `delete_party` has no version argument). An already-deleted target →
    /// `400` (mirroring COMPOSITION delete).
    pub(super) async fn delete_party(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
        expected: Option<TreeId>,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = self.load_party_version(kind, vo_id, None, None).await?;
        if read.deleted() {
            return Err(ServiceError::BadRequest(format!(
                "{} {vo_id} is already deleted",
                kind.rm_type()
            )));
        }
        if let Some(expected) = expected
            && read.tree != expected
        {
            return Err(ServiceError::Conflict(format!(
                "preceding_version_uid names version {expected}, latest is {}",
                read.tree
            )));
        }

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::DELETED, "PARTY delete");
        let committed = vobject::delete(
            &mut tx,
            None,
            vo_id,
            kind_of(kind),
            Some(read.tree),
            &audit,
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;
        metrics::counter!(crate::telemetry::prometheus::DB_TRANSACTIONS, "outcome" => "commit")
            .increment(1);

        Ok(ServiceResponse::deleted(ResourceMeta::new(
            String::new(),
            // Just-created locally → creating_system_id is the service system id.
            self.object_version_id(vo_id, &committed.creating_system_id, committed.tree),
        )))
    }

    /// The current party version metadata (the latest `version_uid` a `412`
    /// echoes in `ETag`/`Location`), or `None` if unknown/wrong-kind.
    pub(super) async fn party_current_meta(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        match self.load_party_version(kind, vo_id, None, None).await {
            Ok(read) => Ok(Some(ResourceMeta::new(
                String::new(),
                self.object_version_id(vo_id, &read.creating_system_id, read.tree),
            ))),
            Err(ServiceError::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // ── VERSIONED_PARTY ──────────────────────────────────────────────────────

    /// The `VERSIONED_PARTY` for a party (any of the five kinds). A non-party id
    /// is `404`.
    pub(super) async fn versioned_party(&self, vo_id: Uuid) -> Result<Value, ServiceError> {
        self.ensure_any_party(vo_id).await?;
        let time_created: jiff_sqlx::Timestamp = sqlx::query_scalar(
            "SELECT a.time_committed FROM vo_version v JOIN audit a ON a.id = v.audit_id \
             WHERE v.vo_id = $1 AND v.sys_version = 1",
        )
        .bind(vo_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("versioned party {vo_id}")))?;
        let time_created = time_created.to_jiff();
        // PORT NOTE: `VERSIONED_OBJECT.owner_id` (1..1) has no EHR owner for a
        // demographic party (ITS-REST defines no demographic wire shape); we
        // reference the party's own versioned-object id as its owner (an
        // idiomatic choice — the demographics repository owns it).
        Ok(json!({
            "_type": "VERSIONED_PARTY",
            "uid": { "_type": "HIER_OBJECT_ID", "value": vo_id.to_string() },
            "owner_id": {
                "_type": "OBJECT_REF",
                "namespace": "demographic",
                "type": "PARTY",
                "id": { "_type": "HIER_OBJECT_ID", "value": vo_id.to_string() }
            },
            "time_created": { "_type": "DV_DATE_TIME", "value": time_created.to_string() }
        }))
    }

    /// The `REVISION_HISTORY` of a party: one item per version with its
    /// `OBJECT_VERSION_ID` and the change's `AUDIT_DETAILS`. A non-party id is
    /// `404`.
    pub(super) async fn party_revision_history(&self, vo_id: Uuid) -> Result<Value, ServiceError> {
        self.ensure_any_party(vo_id).await?;
        let rows = sqlx::query(
            "SELECT v.trunk_version, v.branch_number, v.branch_version, \
             v.creating_system_id, a.system_id, a.change_type, \
             a.description, a.committer, a.time_committed \
             FROM vo_version v JOIN audit a ON a.id = v.audit_id \
             WHERE v.vo_id = $1 ORDER BY v.sys_version",
        )
        .bind(vo_id)
        .fetch_all(&self.pool)
        .await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            let tree = TreeId::from_columns(
                row.try_get("trunk_version")?,
                row.try_get("branch_number")?,
                row.try_get("branch_version")?,
            );
            let creating_system_id: String = row.try_get("creating_system_id")?;
            let system_id: String = row.try_get("system_id")?;
            let change_type: String = row.try_get("change_type")?;
            let description: Option<String> = row.try_get("description")?;
            let committer: Value = row.try_get("committer")?;
            let time_committed: jiff::Timestamp = row
                .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
                .to_jiff();
            items.push(json!({
                "_type": "REVISION_HISTORY_ITEM",
                "version_id": {
                    "_type": "OBJECT_VERSION_ID",
                    "value": self.object_version_id(vo_id, &creating_system_id, tree)
                },
                "audits": [Self::audit_details(
                    &system_id, &change_type, description.as_deref(), &committer, &time_committed,
                )]
            }));
        }
        Ok(json!({ "_type": "REVISION_HISTORY", "items": items }))
    }

    /// An `ORIGINAL_VERSION` of a party at a specific version. A non-party id is
    /// `404`.
    pub(super) async fn party_version(
        &self,
        vo_id: Uuid,
        version: TreeId,
    ) -> Result<Value, ServiceError> {
        self.ensure_any_party(vo_id).await?;
        let read = vobject::read_version(&self.pool, vo_id, version)
            .await?
            .filter(|r| r.ehr_id.is_none())
            .ok_or_else(|| ServiceError::NotFound(format!("party {vo_id} v{version}")))?;
        self.original_version(&read)
    }

    /// The `ORIGINAL_VERSION` of a party extant at `at`, or the latest when `at`
    /// is `None`, with `ETag`/`Location` metadata for the VERSION resource.
    pub(super) async fn party_version_at_time(
        &self,
        vo_id: Uuid,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        self.ensure_any_party(vo_id).await?;
        let read = match at {
            Some(at) => vobject::version_at(&self.pool, vo_id, at).await?,
            None => vobject::read_current(&self.pool, vo_id).await?,
        }
        .filter(|r| r.ehr_id.is_none())
        .ok_or_else(|| ServiceError::NotFound(format!("party {vo_id} version at time")))?;
        let meta = ResourceMeta::new(
            String::new(),
            self.object_version_id(vo_id, &read.creating_system_id, read.tree),
        )
        .with_last_modified(read.time_committed);
        let ov = self.original_version(&read)?;
        Ok(ServiceResponse::new(ov, meta))
    }

    // ── demographic CONTRIBUTION ─────────────────────────────────────────────

    /// Commit a demographic CONTRIBUTION (ehr-less): its versions must reference
    /// party objects (an EHR-kind type inside is rejected `422`). Reuses the
    /// shared [`commit_version_set`](Self::commit_version_set) with `ehr_id =
    /// None` and `party_only = true`.
    pub(super) async fn create_demographic_contribution(
        &self,
        body: Value,
    ) -> Result<ServiceResponse, ServiceError> {
        let contribution_id = self.commit_version_set(None, &body, true).await?;
        let body = self.demographic_contribution(contribution_id).await?;
        let meta = ResourceMeta::new(String::new(), contribution_id.to_string());
        Ok(ServiceResponse::new(body, meta))
    }

    /// Retrieve a demographic (ehr-less) CONTRIBUTION by id. An EHR-scoped
    /// contribution uid here is `404` (the demographic surface only sees
    /// `ehr_id IS NULL` contributions).
    pub(super) async fn demographic_contribution(
        &self,
        contribution_id: Uuid,
    ) -> Result<Value, ServiceError> {
        let meta = sqlx::query(
            "SELECT a.system_id, a.change_type, a.description, a.committer, a.time_committed \
             FROM contribution c JOIN audit a ON a.id = c.audit_id \
             WHERE c.id = $1 AND c.ehr_id IS NULL",
        )
        .bind(contribution_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            ServiceError::NotFound(format!("demographic CONTRIBUTION {contribution_id}"))
        })?;

        let system_id: String = meta.try_get("system_id")?;
        let change_type: String = meta.try_get("change_type")?;
        let description: Option<String> = meta.try_get("description")?;
        let committer: Value = meta.try_get("committer")?;
        let time_committed: jiff::Timestamp = meta
            .try_get::<jiff_sqlx::Timestamp, _>("time_committed")?
            .to_jiff();

        let version_rows = sqlx::query(
            "SELECT vo_id, trunk_version, branch_number, branch_version, creating_system_id, kind FROM vo_version \
             WHERE contribution_id = $1 ORDER BY vo_id",
        )
        .bind(contribution_id)
        .fetch_all(&self.pool)
        .await?;
        let versions: Vec<Value> = version_rows
            .iter()
            .map(|row| -> Result<Value, ServiceError> {
                let vo_id: Uuid = row.try_get("vo_id")?;
                let tree = TreeId::from_columns(
                    row.try_get("trunk_version")?,
                    row.try_get("branch_number")?,
                    row.try_get("branch_version")?,
                );
                let creating_system_id: String = row.try_get("creating_system_id")?;
                let kind: String = row.try_get("kind")?;
                Ok(json!({
                    "_type": "OBJECT_REF",
                    "namespace": "demographic",
                    "type": kind,
                    "id": {
                        "_type": "OBJECT_VERSION_ID",
                        "value": self.object_version_id(vo_id, &creating_system_id, tree)
                    }
                }))
            })
            .collect::<Result<_, _>>()?;

        Ok(json!({
            "_type": "CONTRIBUTION",
            "uid": { "_type": "HIER_OBJECT_ID", "value": contribution_id.to_string() },
            "audit": Self::audit_details(
                &system_id, &change_type, description.as_deref(), &committer, &time_committed,
            ),
            "versions": versions
        }))
    }

    // ── demographic item tags (ehr_id IS NULL) ───────────────────────────────

    /// All demographic tags (ehr-less), optionally filtered by key/value/path.
    pub(super) async fn demographic_tags(
        &self,
        key: Option<&str>,
        value: Option<&str>,
        target_path: Option<&str>,
    ) -> Result<Vec<Value>, ServiceError> {
        let rows = sqlx::query(
            "SELECT target_vo_id, target_type, key, value, target_path FROM item_tag \
             WHERE ehr_id IS NULL \
             AND ($1::text IS NULL OR key = $1) \
             AND ($2::text IS NULL OR value = $2) \
             AND ($3::text IS NULL OR target_path = $3) \
             ORDER BY key",
        )
        .bind(key)
        .bind(value)
        .bind(target_path)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(Self::party_tag_json).collect())
    }

    /// The tags on one party.
    pub(super) async fn party_tags(&self, vo_id: Uuid) -> Result<Vec<Value>, ServiceError> {
        let rows = sqlx::query(
            "SELECT target_vo_id, target_type, key, value, target_path FROM item_tag \
             WHERE ehr_id IS NULL AND target_vo_id = $1 ORDER BY key",
        )
        .bind(vo_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(Self::party_tag_json).collect())
    }

    /// Replace the whole tag collection of a party with the posted set (PUT
    /// full-collection semantics; an empty list clears all). Duplicate keys in
    /// the body are last-wins.
    pub(super) async fn replace_party_tags(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
        tags: Vec<Value>,
    ) -> Result<Vec<Value>, ServiceError> {
        self.ensure_party(kind, vo_id).await?;
        // Validate + dedup (last wins) before touching the DB. A BTreeMap keys
        // by tag key, matching the `ORDER BY key` read-back order.
        let mut deduped: std::collections::BTreeMap<String, (Option<String>, Option<String>)> =
            std::collections::BTreeMap::new();
        for tag in &tags {
            let key = tag
                .get("key")
                .and_then(Value::as_str)
                .ok_or_else(|| ServiceError::Unprocessable("item tag requires a key".to_owned()))?;
            // RM ITEM_TAG Inv_key_valid: non-empty, no leading/trailing whitespace.
            if key.is_empty() || key.trim() != key {
                return Err(ServiceError::Unprocessable(format!(
                    "item tag key {key:?} must be non-empty without leading/trailing whitespace"
                )));
            }
            let value = tag.get("value").and_then(Value::as_str);
            // RM ITEM_TAG Inv_value_valid: `value /= Void implies not value.is_empty`.
            if value == Some("") {
                return Err(ServiceError::Unprocessable(format!(
                    "item tag {key:?}: a value, if set, may not be empty"
                )));
            }
            let target_path = tag.get("target_path").and_then(Value::as_str);
            deduped.insert(
                key.to_owned(),
                (value.map(str::to_owned), target_path.map(str::to_owned)),
            );
        }

        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM item_tag WHERE ehr_id IS NULL AND target_vo_id = $1")
            .bind(vo_id)
            .execute(&mut *tx)
            .await?;
        for (key, (value, target_path)) in &deduped {
            sqlx::query(
                "INSERT INTO item_tag (ehr_id, target_vo_id, target_type, key, value, target_path) \
                 VALUES (NULL, $1, $2, $3, $4, $5)",
            )
            .bind(vo_id)
            .bind(kind.rm_type())
            .bind(key)
            .bind(value)
            .bind(target_path)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        self.party_tags(vo_id).await
    }

    /// Delete a tag by key from a party.
    pub(super) async fn delete_party_tag(
        &self,
        vo_id: Uuid,
        key: &str,
    ) -> Result<(), ServiceError> {
        let deleted = sqlx::query(
            "DELETE FROM item_tag WHERE ehr_id IS NULL AND target_vo_id = $1 AND key = $2",
        )
        .bind(vo_id)
        .bind(key)
        .execute(&self.pool)
        .await?;
        if deleted.rows_affected() == 0 {
            return Err(ServiceError::NotFound(format!("item tag {key:?}")));
        }
        Ok(())
    }

    // ── shared helpers ───────────────────────────────────────────────────────

    /// Load a version of a party, verifying it is of the expected [`PartyKind`]
    /// and ehr-less. A wrong-kind or unknown id is `404`.
    async fn load_party_version(
        &self,
        kind: PartyKind,
        vo_id: Uuid,
        version: Option<TreeId>,
        at: Option<jiff::Timestamp>,
    ) -> Result<VersionRead, ServiceError> {
        // The stored kind (constant per versioned object) must match the route.
        let stored = vobject::object_kind(&self.pool, vo_id).await?;
        if stored != Some(kind_of(kind)) {
            return Err(ServiceError::NotFound(format!(
                "{} {vo_id}",
                kind.rm_type()
            )));
        }
        let read = match (version, at) {
            (Some(v), _) => vobject::read_version(&self.pool, vo_id, v).await?,
            (None, Some(at)) => vobject::version_at(&self.pool, vo_id, at).await?,
            (None, None) => vobject::read_current(&self.pool, vo_id).await?,
        }
        .filter(|r| r.ehr_id.is_none())
        .ok_or_else(|| ServiceError::NotFound(format!("{} {vo_id}", kind.rm_type())))?;
        Ok(read)
    }

    /// Confirm a live party of the expected kind exists (not deleted).
    async fn ensure_party(&self, kind: PartyKind, vo_id: Uuid) -> Result<(), ServiceError> {
        let read = self.load_party_version(kind, vo_id, None, None).await?;
        if read.deleted() {
            return Err(ServiceError::NotFound(format!(
                "{} {vo_id} is deleted",
                kind.rm_type()
            )));
        }
        Ok(())
    }

    /// Confirm `vo_id` is some party (any of the five kinds) — the check for the
    /// kind-agnostic `versioned_party` reads. A non-party id (COMPOSITION, …) or
    /// unknown id is `404`.
    async fn ensure_any_party(&self, vo_id: Uuid) -> Result<(), ServiceError> {
        match vobject::object_kind(&self.pool, vo_id).await? {
            Some(k) if k.is_party() => Ok(()),
            _ => Err(ServiceError::NotFound(format!("versioned party {vo_id}"))),
        }
    }

    /// A [`ServiceResponse`] for a loaded party: its canonical body with the
    /// `uid` injected (PARTY `Uid_mandatory`) plus the resource metadata (an
    /// empty `ehr_id` — parties are not EHR-scoped).
    fn party_version_response(&self, vo_id: Uuid, read: VersionRead) -> ServiceResponse {
        let meta = ResourceMeta::new(
            String::new(),
            self.object_version_id(vo_id, &read.creating_system_id, read.tree),
        )
        .with_last_modified(read.time_committed);
        ServiceResponse::new(
            self.with_uid(read.canonical, vo_id, &read.creating_system_id, read.tree),
            meta,
        )
    }

    /// One demographic `ITEM_TAG` in its wire shape. PORT NOTE: `owner_id`
    /// references the tagged party itself (there is no owning EHR for a
    /// demographic tag).
    fn party_tag_json(row: &sqlx::postgres::PgRow) -> Value {
        let target_vo_id: Uuid = row.try_get("target_vo_id").unwrap_or_default();
        let target_type: String = row.try_get("target_type").unwrap_or_default();
        let mut tag = json!({
            "_type": "ITEM_TAG",
            "key": row.try_get::<String, _>("key").unwrap_or_default(),
            "target": {
                "_type": "OBJECT_REF",
                "namespace": "demographic",
                "type": target_type,
                "id": { "_type": "HIER_OBJECT_ID", "value": target_vo_id.to_string() }
            },
            "owner_id": {
                "_type": "OBJECT_REF",
                "namespace": "demographic",
                "type": target_type,
                "id": { "_type": "HIER_OBJECT_ID", "value": target_vo_id.to_string() }
            },
        });
        if let Ok(Some(value)) = row.try_get::<Option<String>, _>("value") {
            tag["value"] = json!(value);
        }
        if let Ok(Some(path)) = row.try_get::<Option<String>, _>("target_path") {
            tag["target_path"] = json!(path);
        }
        tag
    }
}
