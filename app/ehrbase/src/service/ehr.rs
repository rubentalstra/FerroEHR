//! EHR + `EHR_STATUS` domain logic, built on the [`vobject`](super::vobject)
//! versioned-object machinery. This is the first fully-implemented vertical of
//! the P12 service; COMPOSITION / DIRECTORY reuse the same machinery.

use ehrbase_rest::{EhrSummary, ResourceMeta, ServiceResponse};
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use super::codes::change_type;
use super::version_id::TreeId;
use super::vobject::{self, AuditInput, Change, Kind};
use super::{EhrbaseService, ServiceError};

impl EhrbaseService {
    /// Create an EHR (with the given id), its initial `EHR_STATUS`, and its
    /// `EHR_ACCESS`, all committed under **one** CONTRIBUTION — RM ehr §"EHR
    /// Creation": "the result should be a root EHR object, an EHR Status
    /// object, and an EHR Access object … the EHR Status and EHR Access objects
    /// would be created and committed in a Contribution". Shared by `POST /ehr`
    /// and `PUT /ehr/{ehr_id}`. The response carries the EHR body and its
    /// `ehr_id` (the `ETag`/`Location` for `201_EHR`).
    ///
    /// A duplicate subject (`EHR_STATUS.subject.external_ref`) conflicts at the
    /// database (`ehr_subject_uq`) → 409 (`409_EHR.yaml`; CNF
    /// `create_ehr-two_ehrs_same_patient`).
    pub(super) async fn create_ehr(
        &self,
        ehr_id: Uuid,
        status: Value,
    ) -> Result<ServiceResponse, ServiceError> {
        // The supplied EHR_STATUS must be a structurally valid RM instance before
        // the EHR is created (CNF master06 §Test Data Sets INVALID class 2 —
        // every malformed EHR_STATUS is rejected 4xx).
        validate_ehr_status(&status)?;

        let mut tx = self.pool.begin().await?;

        // system_id is recorded on the EHR at creation, immutable thereafter
        // (RM ehr §"EHR object"; review doc 03 req 2.1 — a stored value, not
        // merely the live service config).
        let inserted =
            sqlx::query("INSERT INTO ehr (id, system_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                .bind(ehr_id)
                .bind(self.effective_system_id())
                .execute(&mut *tx)
                .await?;
        if inserted.rows_affected() == 0 {
            return Err(ServiceError::Conflict(format!(
                "EHR {ehr_id} already exists"
            )));
        }

        let audit = self.audit(change_type::CREATION, "EHR creation");
        vobject::commit_contribution(
            &mut tx,
            Some(ehr_id),
            &audit,
            vec![
                (
                    audit.clone(),
                    Change::Create {
                        kind: Kind::EhrStatus,
                        canonical: status,
                        template_id: None,
                        signature: None,
                        lifecycle_state: None,
                        attestations: Vec::new(),
                    },
                ),
                (
                    audit.clone(),
                    Change::Create {
                        kind: Kind::EhrAccess,
                        canonical: default_ehr_access(),
                        template_id: None,
                        signature: None,
                        lifecycle_state: None,
                        attestations: Vec::new(),
                    },
                ),
            ],
            Vec::new(),
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;

        self.ehr_summary(ehr_id).await
    }

    /// Find an EHR by the subject its current `EHR_STATUS` names (external ref
    /// `id.value` + `namespace`), returning the EHR summary. Served from the
    /// promoted `ehr.subject_*` columns the `EHR_STATUS` writes keep in sync
    /// (unique per subject — `ehr_subject_uq`).
    pub(super) async fn ehr_by_subject(
        &self,
        subject_id: &str,
        namespace: &str,
    ) -> Result<ServiceResponse, ServiceError> {
        let ehr_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM ehr WHERE subject_id = $1 AND subject_namespace = $2",
        )
        .bind(subject_id)
        .bind(namespace)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            ServiceError::NotFound(format!("EHR for subject {subject_id}@{namespace}"))
        })?;
        self.ehr_summary(ehr_id).await
    }

    /// Build the canonical EHR object for an existing EHR, with its `ehr_id`
    /// metadata (the `ETag`/`Location` for `POST /ehr`).
    pub(super) async fn ehr_summary(&self, ehr_id: Uuid) -> Result<ServiceResponse, ServiceError> {
        let row = sqlx::query("SELECT system_id, time_created FROM ehr WHERE id = $1")
            .bind(ehr_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR {ehr_id}")))?;
        // EHR.system_id is IMMUTABLE after creation (RM ehr master04 §Root EHR
        // Object) — served from the STORED per-EHR value, never the live
        // service configuration (a config change must not mutate it).
        let stored_system_id: String = row.try_get("system_id")?;
        // timestamptz via the official jiff-sqlx wrapper (sqlx-conventions.md).
        let time_created: jiff::Timestamp = row
            .try_get::<jiff_sqlx::Timestamp, _>("time_created")?
            .to_jiff();

        let (status_vo, status_version) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        // The uid uses the stored per-version creating_system_id (M2), not the
        // live config, so it is stable across a `with_system_id` change.
        let (t, b, v) = status_version.columns();
        let status_csid: String = sqlx::query_scalar(
            "SELECT creating_system_id FROM vo_version WHERE vo_id = $1 \
             AND trunk_version = $2 AND branch_number = $3 AND branch_version = $4",
        )
        .bind(status_vo)
        .bind(t)
        .bind(b)
        .bind(v)
        .fetch_one(&self.pool)
        .await?;
        let status_ovid = self.object_version_id(status_vo, &status_csid, status_version);

        let mut body = json!({
            "_type": "EHR",
            "system_id": { "_type": "HIER_OBJECT_ID", "value": stored_system_id },
            "ehr_id": { "_type": "HIER_OBJECT_ID", "value": ehr_id.to_string() },
            // Ehr_status_valid: `ehr_status.type.is_equal("VERSIONED_EHR_STATUS")`
            // (RM ehr `ehr.adoc` invariants — normative). PORT NOTE (spec
            // defect): the non-normative ITS-REST example
            // (`schemas/ehr/Ehr.yaml`) shows `type: EHR_STATUS` with an
            // OBJECT_VERSION_ID id, contradicting the RM invariant; the RM
            // invariant wins for `type`, while the id keeps the example's
            // OBJECT_VERSION_ID form (the invariant does not constrain it and
            // clients use it to address the current status version).
            "ehr_status": {
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": "VERSIONED_EHR_STATUS",
                "id": { "_type": "OBJECT_VERSION_ID", "value": status_ovid }
            },
            "time_created": {
                "_type": "DV_DATE_TIME",
                "value": time_created.to_string()
            }
        });
        // EHR.ehr_access (1..1): a reference to the VERSIONED_EHR_ACCESS version
        // container — invariant `Ehr_access_valid:
        // ehr_access.type.is_equal("VERSIONED_EHR_ACCESS")` (RM ehr, EHR class;
        // finding F-06-07). Every EHR this service creates has one; tolerate its
        // absence only for rows inserted outside `create_ehr` (raw fixtures).
        if let Some((access_vo, _)) = self.current_vo(ehr_id, Kind::EhrAccess).await? {
            body["ehr_access"] = json!({
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": "VERSIONED_EHR_ACCESS",
                "id": { "_type": "HIER_OBJECT_ID", "value": access_vo.to_string() }
            });
        }
        // For an EHR the `ETag`/`Location` are keyed by the `ehr_id`
        // (`ETag_EHR.yaml` / `Location_EHR.yaml`).
        let meta = ResourceMeta::new(ehr_id.to_string(), ehr_id.to_string())
            .with_last_modified(time_created);
        Ok(ServiceResponse::new(body, meta))
    }

    /// SM `EHR_SUMMARY` for an existing EHR — the summary form of `EHR` +
    /// `EHR_STATUS` (`docs/specs/openehr/SM/docs/UML/classes/ehr_summary.adoc`):
    /// all six mandatory attributes. `system_id` is the service system id (the
    /// same `EHR.system_id` the wire EHR body carries); `ehr_status` is the
    /// current `EHR_STATUS` canonical JSON (reusing [`Self::status_at`]);
    /// `contribution_count` counts the EHR's CONTRIBUTIONs; `composition_count`
    /// is the "Number of (versioned) Compositions" — distinct versioned objects
    /// (`vo_id`), **not** versions (`ehr_summary.adoc` wording). A missing EHR is
    /// `NotFound` (SM `ehr_does_not_exist`).
    pub(super) async fn summarize_ehr(&self, ehr_id: Uuid) -> Result<EhrSummary, ServiceError> {
        let row = sqlx::query("SELECT system_id, time_created FROM ehr WHERE id = $1")
            .bind(ehr_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR {ehr_id}")))?;
        // EHR.system_id is immutable per EHR (master04 §Root EHR Object) —
        // the stored value, not the live config.
        let stored_system_id: String = row.try_get("system_id")?;
        let time_created: jiff::Timestamp = row
            .try_get::<jiff_sqlx::Timestamp, _>("time_created")?
            .to_jiff();

        // Copy of EHR.ehr_status: the current EHR_STATUS (bare, with its uid).
        let ehr_status = self.status_at(ehr_id, None).await?.body;

        let contribution_count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM contribution WHERE ehr_id = $1")
                .bind(ehr_id)
                .fetch_one(&self.pool)
                .await?;
        // "Number of (versioned) Compositions" = versioned objects, not versions
        // (ehr_summary.adoc): count distinct COMPOSITION vo_id in vo_version.
        let composition_count: i64 = sqlx::query_scalar(
            "SELECT count(DISTINCT vo_id) FROM vo_version WHERE ehr_id = $1 AND kind = 'COMPOSITION'",
        )
        .bind(ehr_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(EhrSummary {
            ehr_id: ehr_id.to_string(),
            system_id: stored_system_id,
            ehr_status,
            time_created: time_created.to_string(),
            contribution_count,
            composition_count,
        })
    }

    /// The `EHR_STATUS` of an EHR as canonical JSON with its `uid` set — the
    /// current version, or the one current at `at` (time-travel) when given.
    pub(super) async fn status_at(
        &self,
        ehr_id: Uuid,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        let read = match at {
            Some(at) => vobject::version_at(&self.pool, vo_id, at).await?,
            None => vobject::read_current(&self.pool, vo_id).await?,
        }
        .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        Ok(self.version_response(ehr_id, vo_id, read))
    }

    /// The `EHR_STATUS` at a specific version as canonical JSON with its `uid`
    /// set — the **bare** resource (not the `ORIGINAL_VERSION` wrapper) that
    /// `GET /ehr/{ehr_id}/ehr_status/{version_uid}` returns (F-01-03).
    pub(super) async fn status_by_version(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        version: TreeId,
    ) -> Result<ServiceResponse, ServiceError> {
        let read = vobject::read_version(&self.pool, vo_id, version)
            .await?
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS {vo_id} v{version}")))?;
        Ok(self.version_response(ehr_id, vo_id, read))
    }

    /// Update an EHR's `EHR_STATUS`, returning the new version. `if_match` is the
    /// `OBJECT_VERSION_ID` (or bare version) the client believes is current.
    pub(super) async fn status_update(
        &self,
        ehr_id: Uuid,
        body: Value,
        if_match: &str,
    ) -> Result<ServiceResponse, ServiceError> {
        // A modified EHR_STATUS must remain a structurally valid RM instance
        // (RM ehr §EHR_STATUS: mandatory subject / is_queryable / is_modifiable /
        // name / archetype_node_id).
        validate_ehr_status(&body)?;

        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        let expected = super::version_id::expected_from_if_match(if_match);

        let mut tx = self.pool.begin().await?;
        let audit = self.audit(change_type::MODIFICATION, "EHR_STATUS update");
        vobject::update(
            &mut tx,
            Some(ehr_id),
            vo_id,
            Kind::EhrStatus,
            body,
            expected,
            None,
            &audit,
            &self.signing_ctx(),
        )
        .await?;
        tx.commit().await?;

        let read = vobject::read_current(&self.pool, vo_id)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        Ok(self.version_response(ehr_id, vo_id, read))
    }

    /// The `VERSIONED_OBJECT` for an EHR's `EHR_STATUS`.
    pub(super) async fn versioned_status(&self, ehr_id: Uuid) -> Result<Value, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        self.versioned_object(vo_id, ehr_id).await
    }

    /// The `REVISION_HISTORY` of an EHR's `EHR_STATUS`.
    pub(super) async fn status_revision_history(
        &self,
        ehr_id: Uuid,
    ) -> Result<Value, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        self.revision_history(ehr_id, vo_id).await
    }

    /// An `ORIGINAL_VERSION` of an `EHR_STATUS` at a specific version.
    pub(super) async fn status_version(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        version: TreeId,
    ) -> Result<Value, ServiceError> {
        let read = vobject::read_version(&self.pool, vo_id, version)
            .await?
            .filter(|r| r.ehr_id == Some(ehr_id))
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS {vo_id} v{version}")))?;
        self.original_version(&read)
    }

    /// The `ORIGINAL_VERSION` of an EHR's `EHR_STATUS` extant at `at`, or the
    /// latest when `at` is `None` — `GET /ehr/{ehr_id}/versioned_ehr_status/version`
    /// (`versioned_ehr_status_version_get_at_time.yaml`; finding F-01-05). The
    /// metadata carries the `version_uid` for `200_VERSION_at_time`'s
    /// `ETag`/`Location`.
    pub(super) async fn status_version_at_time(
        &self,
        ehr_id: Uuid,
        at: Option<jiff::Timestamp>,
    ) -> Result<ServiceResponse, ServiceError> {
        let (vo_id, _) = self
            .current_vo(ehr_id, Kind::EhrStatus)
            .await?
            .ok_or_else(|| ServiceError::NotFound(format!("EHR_STATUS for EHR {ehr_id}")))?;
        let read = match at {
            Some(at) => vobject::version_at(&self.pool, vo_id, at).await?,
            None => vobject::read_current(&self.pool, vo_id).await?,
        }
        .ok_or_else(|| {
            ServiceError::NotFound(format!("EHR_STATUS version at time for EHR {ehr_id}"))
        })?;
        let meta = self.version_meta(
            ehr_id,
            vo_id,
            &read.creating_system_id,
            read.tree,
            read.time_committed,
        );
        let ov = self.original_version(&read)?;
        Ok(ServiceResponse::new(ov, meta))
    }

    /// The current `EHR_STATUS` version metadata (for a `412` `ETag`/`Location`).
    pub(super) async fn ehr_status_meta(
        &self,
        ehr_id: Uuid,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        self.latest_version_meta(ehr_id, Kind::EhrStatus).await
    }

    /// The current directory FOLDER version metadata (for a `412`
    /// `ETag`/`Location`).
    pub(super) async fn directory_meta(
        &self,
        ehr_id: Uuid,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        self.latest_version_meta(ehr_id, Kind::Folder).await
    }

    /// The current version row (`vo_id`, `sys_version`) of an EHR's object of a
    /// given kind, if any.
    pub(super) async fn current_vo(
        &self,
        ehr_id: Uuid,
        kind: Kind,
    ) -> Result<Option<(Uuid, TreeId)>, ServiceError> {
        let row = sqlx::query(
            "SELECT vo_id, trunk_version, branch_number, branch_version FROM vo_version \
             WHERE ehr_id = $1 AND kind = $2 AND upper_inf(sys_period) AND branch_number = 0",
        )
        .bind(ehr_id)
        .bind(kind.as_str())
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(r) => Ok(Some((
                r.try_get("vo_id")?,
                TreeId::from_columns(
                    r.try_get("trunk_version")?,
                    r.try_get("branch_number")?,
                    r.try_get("branch_version")?,
                ),
            ))),
            None => Ok(None),
        }
    }

    /// Whether the EHR's current `EHR_STATUS` has `is_modifiable = true`
    /// (RM ehr `EHR_STATUS.is_modifiable`, 1..1 Boolean). `is_modifiable` is a
    /// scalar attribute of `EHR_STATUS`, so it lives inline in the `EHR_STATUS`
    /// **root** node's verbatim canonical `data` fragment (`num = 0`; children
    /// are pruned but scalars stay — ADR-008 §2), the same access the AQL
    /// `is_queryable` population filter uses (`aql/sql.rs`). An EHR with no
    /// current `EHR_STATUS` (should not occur — every EHR is created with one)
    /// is treated as modifiable, so the guard never spuriously blocks.
    async fn ehr_is_modifiable(&self, ehr_id: Uuid) -> Result<bool, ServiceError> {
        let flag: Option<bool> = sqlx::query_scalar(
            "SELECT (n.data->>'is_modifiable') = 'true' \
             FROM vo_version v \
             JOIN node n ON n.vo_id = v.vo_id AND n.sys_version = v.sys_version AND n.num = 0 \
             WHERE v.ehr_id = $1 AND v.kind = 'EHR_STATUS' AND upper_inf(v.sys_period) \
             AND v.branch_number = 0",
        )
        .bind(ehr_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(flag.unwrap_or(true))
    }

    /// Refuse a write to *EHR contents* when the EHR is deactivated
    /// (`EHR_STATUS.is_modifiable = False`). Per `ehr/master04-ehr_package.adoc`
    /// §"EHR Active Status", `is_modifiable` "is used to indicate whether the
    /// contents of an EHR are modifiable"; "an EHR's 'contents' consist of
    /// everything other than the `EHR_STATUS` object, i.e. its Compositions …
    /// its hierarchical Folders … and any other content". The `EHR_STATUS`
    /// object itself "is always modifiable" (`master04` §"EHR Creation" +
    /// §"EHR Active Status"), which is how a deactivated EHR is flipped back —
    /// so this guard is applied to COMPOSITION / DIRECTORY / content-CONTRIBUTION
    /// writes only, never to `status_update`.
    ///
    /// PORT NOTE (wire): openEHR ITS-REST 1.0.3 does not enumerate a status code
    /// for a write to a non-modifiable EHR (`composition_create.yaml` etc. list
    /// only 201/400/404/422; the CNF schedule
    /// `master06-func_tc_ehr.adoc` §set/clear-modifiable tests the flag flip, not
    /// the write-block outcome), so the wire code is underdetermined. We return
    /// `409 Conflict` — the write conflicts with the current state of the target
    /// resource (RFC 9110 §15.5.10), the closest HTTP semantics and `EHRbase`'s
    /// prior-art behaviour — via [`ServiceError::Conflict`].
    pub(super) async fn ensure_content_writable(&self, ehr_id: Uuid) -> Result<(), ServiceError> {
        if self.ehr_is_modifiable(ehr_id).await? {
            Ok(())
        } else {
            Err(ServiceError::Conflict(format!(
                "EHR {ehr_id} is not modifiable (EHR_STATUS.is_modifiable = false); its \
                 contents cannot be created, updated or deleted (ehr/master04 §\"EHR Active \
                 Status\"). Set EHR_STATUS.is_modifiable = true to reactivate it."
            )))
        }
    }

    /// The `OBJECT_VERSION_ID` string `{object_id}::{creating_system_id}::
    /// {version_tree_id}` (RM common master06 §"Version Identification").
    /// `creating_system_id` is the stored per-version value, reconstructed from
    /// storage — never re-derived from the live config — so a version's uid and
    /// digital signature stay stable across a later `with_system_id` change (RM
    /// common master06 §"Distributed Versioning"). The service writes the real
    /// creating system id on every version (ADR-013 §8 — no `''` sentinel).
    // Kept as a method (not a free fn) for call-site ergonomics — every caller
    // already holds the service; the stored `creating_system_id` is now
    // authoritative, so no `self` state is consulted (ADR-013 §8).
    #[allow(clippy::unused_self)]
    pub(super) fn object_version_id(
        &self,
        vo_id: Uuid,
        creating_system_id: &str,
        sys_version: TreeId,
    ) -> String {
        format!("{vo_id}::{creating_system_id}::{sys_version}")
    }

    /// The [`ResourceMeta`] for a versioned resource: the owning EHR plus the
    /// resource `OBJECT_VERSION_ID` (the `ETag` value + `Location` tail) and its
    /// commit time.
    pub(super) fn version_meta(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        creating_system_id: &str,
        sys_version: TreeId,
        at: jiff::Timestamp,
    ) -> ResourceMeta {
        ResourceMeta::new(
            ehr_id.to_string(),
            self.object_version_id(vo_id, creating_system_id, sys_version),
        )
        .with_last_modified(at)
    }

    /// A [`ServiceResponse`] for a loaded versioned object: its canonical body
    /// with the `uid` injected, plus the resource metadata for the wire headers.
    pub(super) fn version_response(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        read: vobject::VersionRead,
    ) -> ServiceResponse {
        let meta = self.version_meta(
            ehr_id,
            vo_id,
            &read.creating_system_id,
            read.tree,
            read.time_committed,
        );
        ServiceResponse::new(
            self.with_uid(read.canonical, vo_id, &read.creating_system_id, read.tree),
            meta,
        )
    }

    /// The current version metadata of an EHR-owned versioned object of `kind`
    /// (for the latest `version_uid` a `409`/`412` must echo). `None` if none.
    pub(super) async fn latest_version_meta(
        &self,
        ehr_id: Uuid,
        kind: Kind,
    ) -> Result<Option<ResourceMeta>, ServiceError> {
        let Some((vo_id, _)) = self.current_vo(ehr_id, kind).await? else {
            return Ok(None);
        };
        let Some(read) = vobject::read_current(&self.pool, vo_id).await? else {
            return Ok(None);
        };
        Ok(Some(self.version_meta(
            ehr_id,
            vo_id,
            &read.creating_system_id,
            read.tree,
            read.time_committed,
        )))
    }

    /// Inject the `uid` (`OBJECT_VERSION_ID`) into a versioned object's JSON.
    pub(super) fn with_uid(
        &self,
        mut canonical: Value,
        vo_id: Uuid,
        creating_system_id: &str,
        sys_version: TreeId,
    ) -> Value {
        if let Value::Object(map) = &mut canonical {
            map.insert(
                "uid".to_owned(),
                json!({
                    "_type": "OBJECT_VERSION_ID",
                    "value": self.object_version_id(vo_id, creating_system_id, sys_version)
                }),
            );
        }
        canonical
    }

    pub(super) fn audit(&self, change_type: &str, description: &str) -> AuditInput {
        AuditInput {
            system_id: self.effective_system_id(),
            change_type: change_type.to_owned(),
            description: Some(description.to_owned()),
            committer: committer(),
        }
    }
}

/// The default `EHR_STATUS` for a new EHR (queryable, modifiable, `PARTY_SELF`).
pub(super) fn default_ehr_status() -> Value {
    json!({
        "_type": "EHR_STATUS",
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "EHR Status" },
        "subject": { "_type": "PARTY_SELF" },
        "is_queryable": true,
        "is_modifiable": true
    })
}

/// The default `EHR_ACCESS` created with every EHR (RM ehr §"EHR Creation";
/// finding F-06-07). `EHR_ACCESS` is a LOCATABLE with only the optional
/// `settings` attribute; with no access-control scheme configured (Stage 1 has
/// no RBAC — Stage 2), it is committed with none.
pub(super) fn default_ehr_access() -> Value {
    json!({
        "_type": "EHR_ACCESS",
        "archetype_node_id": "openEHR-EHR-EHR_ACCESS.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "EHR Access" }
    })
}

/// The committer `PARTY_PROXY` for an audit, taken from the authenticated
/// principal of the current request (published by the auth middleware). Writes
/// with no authenticated principal (auth disabled, or internal/system writes)
/// are attributed to the system identity.
pub(super) fn committer() -> Value {
    match ehrbase_rest::access::authn::current_principal() {
        Some(principal) => {
            let id_type = match principal.method {
                ehrbase_rest::AuthMethod::Basic => "basic",
                ehrbase_rest::AuthMethod::Bearer => "oauth2",
            };
            json!({
                "_type": "PARTY_IDENTIFIED",
                "name": principal.subject.clone(),
                "identifiers": [{
                    "_type": "DV_IDENTIFIER",
                    "id": principal.subject,
                    "issuer": "ehrbase-rs",
                    "type": id_type
                }]
            })
        }
        None => json!({ "_type": "PARTY_IDENTIFIED", "name": "EHRbase" }),
    }
}

/// Structurally validate an `EHR_STATUS` before it is committed (on EHR create,
/// `EHR_STATUS` update, or a CONTRIBUTION). Rejects every malformed data set the
/// CNF `master06 §Test Data Sets (INVALID class 2)` enumerates with a `422`.
///
/// Rules — RM ehr §`EHR_STATUS` + inherited `LOCATABLE`
/// (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.ehr.ehr_status.adoc`,
/// `…rm.common.locatable.adoc`):
/// - `_type` present and equal to `EHR_STATUS` (the concrete versioned-object
///   root the endpoint commits);
/// - `name` present (`LOCATABLE.name 1..1`);
/// - `archetype_node_id` present and non-empty (`Archetype_node_id_valid`);
/// - `is_queryable` / `is_modifiable` present booleans (both `1..1`);
/// - `subject` present and typed `PARTY_SELF` (`EHR_STATUS.subject 1..1
///   PARTY_SELF`). `PARTY_SELF` is monomorphic (no subtypes), so a foreign
///   concrete `_type` in this slot (e.g. `PARTY_IDENTIFIED`) is invalid; the
///   slot is enforced through the generated `PartySelf` type's own
///   `#[derive(OpenEhrType)]` `_type` check rather than a hand-rolled string
///   compare. An empty `{}` subject is a **valid anonymous** subject — RM ehr
///   `master04 §EHR Status`: "the subject is represented by a `PARTY_SELF`
///   object, enabling it to be made completely anonymous" — so it is accepted;
/// - when `subject.external_ref` is present it is a valid `PARTY_REF`
///   (`OBJECT_REF`): a non-empty `id.value` (`Id_exists`) and a non-empty
///   `namespace` (`Namespace_valid`). A `NULL` `external_ref` is permitted (CNF
///   master08 `EHR_STATUS` combinations accept `subject.external_ref = NULL`).
pub(super) fn validate_ehr_status(status: &Value) -> Result<(), ServiceError> {
    let unproc = |m: String| ServiceError::Unprocessable(m);
    let obj = status
        .as_object()
        .ok_or_else(|| unproc("EHR_STATUS must be a JSON object".to_owned()))?;

    match obj.get("_type").and_then(Value::as_str) {
        Some("EHR_STATUS") => {}
        Some(other) => {
            return Err(unproc(format!(
                "EHR_STATUS _type must be \"EHR_STATUS\", got {other:?}"
            )));
        }
        None => {
            return Err(unproc(
                "EHR_STATUS is missing its _type discriminator".to_owned(),
            ));
        }
    }

    if !obj.contains_key("name") {
        return Err(unproc(
            "EHR_STATUS.name is mandatory (LOCATABLE.name 1..1)".to_owned(),
        ));
    }
    match obj.get("archetype_node_id").and_then(Value::as_str) {
        Some(s) if !s.is_empty() => {}
        _ => {
            return Err(unproc(
                "EHR_STATUS.archetype_node_id is mandatory and non-empty \
                 (LOCATABLE.Archetype_node_id_valid)"
                    .to_owned(),
            ));
        }
    }
    if !obj.get("is_queryable").is_some_and(Value::is_boolean) {
        return Err(unproc(
            "EHR_STATUS.is_queryable is mandatory (1..1 Boolean)".to_owned(),
        ));
    }
    if !obj.get("is_modifiable").is_some_and(Value::is_boolean) {
        return Err(unproc(
            "EHR_STATUS.is_modifiable is mandatory (1..1 Boolean)".to_owned(),
        ));
    }

    let subject = obj
        .get("subject")
        .filter(|v| v.is_object())
        .ok_or_else(|| unproc("EHR_STATUS.subject is mandatory (1..1 PARTY_SELF)".to_owned()))?;

    // `EHR_STATUS.subject` is typed `PARTY_SELF` (RM ehr master04 §EHR Status:
    // "the subject is represented by a PARTY_SELF object"). `PARTY_SELF` is
    // monomorphic — it has no subtypes — so a foreign concrete `_type` in this
    // slot (e.g. `PARTY_IDENTIFIED`) is invalid. Enforce this through the
    // generated type itself: `PartySelf`'s `#[derive(OpenEhrType)]` `Deserialize`
    // rejects a mismatched `_type` (openehr-derive). An absent `_type` defaults
    // to `PARTY_SELF` and an empty `{}` deserialises to an anonymous `PARTY_SELF`
    // (`external_ref: None`) — the spec's "completely anonymous" subject — both
    // of which this accepts.
    //
    // PORT NOTE (Fix 2/B1): scoped to `EHR_STATUS.subject` rather than a
    // whole-`EhrStatus` typed deserialize — this keeps the RM-1.2.0-vs-corpus
    // version skew off the commit-path guard (the surrounding structural and
    // `PARTY_REF` invariant checks cover the other attributes), matching the
    // demographic `typed_check` pattern (`service::demographic`).
    serde_json::from_value::<openehr_rm::prelude::PartySelf>(subject.clone()).map_err(|e| {
        unproc(format!(
            "EHR_STATUS.subject must be a PARTY_SELF (RM ehr master04 §EHR Status): {e}"
        ))
    })?;

    let external_ref = subject
        .as_object()
        .and_then(|s| s.get("external_ref"))
        .filter(|v| !v.is_null());
    if let Some(external_ref) = external_ref {
        let ext = external_ref.as_object().ok_or_else(|| {
            unproc("EHR_STATUS.subject.external_ref must be a PARTY_REF object".to_owned())
        })?;
        match ext.get("id").and_then(Value::as_object) {
            Some(id)
                if id
                    .get("value")
                    .and_then(Value::as_str)
                    .is_some_and(|v| !v.is_empty()) => {}
            _ => {
                return Err(unproc(
                    "EHR_STATUS.subject.external_ref.id.value is mandatory and non-empty \
                     (OBJECT_REF.Id_exists)"
                        .to_owned(),
                ));
            }
        }
        match ext.get("namespace").and_then(Value::as_str) {
            Some(ns) if !ns.is_empty() => {}
            _ => {
                return Err(unproc(
                    "EHR_STATUS.subject.external_ref.namespace is mandatory and non-empty \
                     (OBJECT_REF.Namespace_valid)"
                        .to_owned(),
                ));
            }
        }
    }

    // `EHR_STATUS.other_details` (0..1) is typed `ITEM_STRUCTURE` — an abstract
    // slot whose concrete
    // slot whose concrete subtypes are ITEM_TREE / ITEM_LIST / ITEM_SINGLE /
    // ITEM_TABLE (RM ehr `ehr_status.adoc` other_details; RM data_structures
    // master04 §Item structure). An abstract-typed slot requires the concrete
    // `_type` on the wire; a foreign `_type` (e.g. a DATA_VALUE) is invalid.
    if let Some(other) = obj.get("other_details").filter(|v| !v.is_null()) {
        let ty = other.get("_type").and_then(Value::as_str);
        match ty {
            Some("ITEM_TREE" | "ITEM_LIST" | "ITEM_SINGLE" | "ITEM_TABLE") => {}
            other_ty => {
                return Err(unproc(format!(
                    "EHR_STATUS.other_details must be an ITEM_STRUCTURE \
                     (ITEM_TREE/ITEM_LIST/ITEM_SINGLE/ITEM_TABLE), got _type {other_ty:?}"
                )));
            }
        }
    }
    Ok(())
}

/// Validate a client-supplied `EHR_ACCESS` before it is committed (via a
/// CONTRIBUTION — there is no direct ITS-REST EHR_ACCESS write). RM ehr
/// `org.openehr.rm.ehr.ehr_access.adoc`:
///
/// - a LOCATABLE: `name` (1..1) and a non-empty `archetype_node_id`
///   (`Archetype_node_id_valid`);
/// - a foreign `_type` in this slot is invalid (the container holds
///   `EHR_ACCESS` only);
/// - `settings` (0..1) is a subtype of the ABSTRACT `ACCESS_CONTROL_SETTINGS`
///   — the RM defines no concrete scheme, so a present `settings` must carry
///   a non-empty concrete `_type`, which is what `scheme()` names
///   (`Scheme_valid`: `not scheme.is_empty`).
pub(super) fn validate_ehr_access(access: &Value) -> Result<(), ServiceError> {
    let unproc = |m: String| ServiceError::Unprocessable(m);
    let obj = access
        .as_object()
        .ok_or_else(|| unproc("EHR_ACCESS must be a JSON object".to_owned()))?;
    match obj.get("_type").and_then(Value::as_str) {
        None | Some("EHR_ACCESS") => {}
        Some(other) => {
            return Err(unproc(format!(
                "expected an EHR_ACCESS, got _type {other:?}"
            )));
        }
    }
    if obj.get("name").filter(|v| !v.is_null()).is_none() {
        return Err(unproc(
            "EHR_ACCESS.name is mandatory (LOCATABLE.name 1..1)".to_owned(),
        ));
    }
    if !obj
        .get("archetype_node_id")
        .and_then(Value::as_str)
        .is_some_and(|s| !s.is_empty())
    {
        return Err(unproc(
            "EHR_ACCESS.archetype_node_id is mandatory and non-empty \
             (LOCATABLE.Archetype_node_id_valid)"
                .to_owned(),
        ));
    }
    if let Some(settings) = obj.get("settings").filter(|v| !v.is_null())
        && !settings
            .get("_type")
            .and_then(Value::as_str)
            .is_some_and(|t| !t.is_empty())
    {
        return Err(unproc(
            "EHR_ACCESS.settings must be a concrete ACCESS_CONTROL_SETTINGS subtype \
             carrying its _type — the scheme name (EHR_ACCESS.Scheme_valid)"
                .to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `EHR_STATUS.other_details` must be a concrete `ITEM_STRUCTURE`
    /// (RM ehr `ehr_status.adoc`; A1 rm-ehr-R15): the four concrete subtypes
    /// pass, a foreign or missing `_type` rejects.
    #[test]
    fn ehr_status_other_details_type_is_enforced() {
        let with_other = |other: Value| {
            let mut st = default_ehr_status();
            st.as_object_mut()
                .unwrap()
                .insert("other_details".into(), other);
            st
        };
        for ty in ["ITEM_TREE", "ITEM_LIST", "ITEM_SINGLE", "ITEM_TABLE"] {
            validate_ehr_status(&with_other(json!({ "_type": ty, "name": { "_type": "DV_TEXT", "value": "d" }, "archetype_node_id": "at0001" })))
                .unwrap_or_else(|e| panic!("{ty} other_details must be accepted: {e}"));
        }
        for bad in [
            json!({ "_type": "DV_TEXT", "value": "x" }),
            json!({ "value": "x" }),
        ] {
            let err = validate_ehr_status(&with_other(bad))
                .expect_err("non-ITEM_STRUCTURE other_details must be rejected");
            assert!(err.to_string().contains("ITEM_STRUCTURE"), "got {err}");
        }
    }

    /// `EHR_ACCESS` commit validation (RM ehr `ehr_access.adoc`; A1
    /// rm-ehr-R20/R22): LOCATABLE structure enforced, a present `settings`
    /// must be a concrete `ACCESS_CONTROL_SETTINGS` subtype (its `_type` is
    /// the scheme name — `Scheme_valid`).
    #[test]
    fn ehr_access_commit_validation() {
        validate_ehr_access(&default_ehr_access()).expect("the default EHR_ACCESS is valid");
        let err = validate_ehr_access(&json!({ "_type": "EHR_STATUS" }))
            .expect_err("foreign _type rejected");
        assert!(err.to_string().contains("EHR_ACCESS"), "got {err}");
        let err = validate_ehr_access(&json!({
            "_type": "EHR_ACCESS", "archetype_node_id": "openEHR-EHR-EHR_ACCESS.generic.v1"
        }))
        .expect_err("missing name rejected");
        assert!(err.to_string().contains("name"), "got {err}");
        let err = validate_ehr_access(&json!({
            "_type": "EHR_ACCESS",
            "name": { "_type": "DV_TEXT", "value": "EHR Access" },
            "archetype_node_id": "openEHR-EHR-EHR_ACCESS.generic.v1",
            "settings": { "scheme": "acme" }
        }))
        .expect_err("settings without a concrete _type rejected (Scheme_valid)");
        assert!(err.to_string().contains("Scheme_valid"), "got {err}");
    }

    #[test]
    fn default_status_decomposes() {
        // The default EHR_STATUS must be a valid structure root for the codec.
        let rows = crate::storage::decompose(default_ehr_status()).expect("decompose");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].rm_type, "EHR_STATUS");
    }

    #[test]
    fn default_and_typical_ehr_status_are_accepted() {
        // The server's own default and a fully-identified subject both validate.
        validate_ehr_status(&default_ehr_status()).expect("default EHR_STATUS");
        // A subject identified via `external_ref` is still a `PARTY_SELF`
        // (RM ehr master04 §EHR Status: "the subject is represented by a
        // PARTY_SELF object … or alternatively to include a patient identifier").
        let identified = json!({
            "_type": "EHR_STATUS",
            "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
            "name": { "_type": "DV_TEXT", "value": "EHR Status" },
            "subject": {
                "_type": "PARTY_SELF",
                "external_ref": {
                    "_type": "PARTY_REF",
                    "namespace": "conformance",
                    "type": "PERSON",
                    "id": { "_type": "GENERIC_ID", "value": "subj-1", "scheme": "id_scheme" }
                }
            },
            "is_queryable": true,
            "is_modifiable": false
        });
        validate_ehr_status(&identified).expect("identified PARTY_SELF EHR_STATUS");
    }

    /// A subject typed with a foreign concrete `PARTY_PROXY` subtype
    /// (`PARTY_IDENTIFIED`) is rejected — `EHR_STATUS.subject` is monomorphic
    /// `PARTY_SELF` (RM ehr master04 §EHR Status). Regression for the upstream
    /// diff finding B1 (`docs/conformance/upstream-ehrbase/TRIAGE.md`).
    #[test]
    fn ehr_status_subject_wrong_type_is_rejected() {
        let bad = json!({
            "_type": "EHR_STATUS",
            "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
            "name": { "_type": "DV_TEXT", "value": "EHR Status" },
            "subject": {
                "_type": "PARTY_IDENTIFIED",
                "external_ref": {
                    "_type": "PARTY_REF",
                    "namespace": "conformance",
                    "type": "PERSON",
                    "id": { "_type": "GENERIC_ID", "value": "subj-1", "scheme": "id_scheme" }
                }
            },
            "is_queryable": true,
            "is_modifiable": true
        });
        let err = validate_ehr_status(&bad).expect_err("PARTY_IDENTIFIED subject must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("PARTY_SELF") && msg.contains("PARTY_IDENTIFIED"),
            "rejection should name the type mismatch, got: {msg}"
        );
    }

    /// An anonymous subject — empty `PARTY_SELF` (`{}`) or an explicit
    /// `{"_type":"PARTY_SELF"}` with no `external_ref` — is accepted. RM ehr
    /// master04 §EHR Status: `PARTY_SELF` "enabling it to be made completely
    /// anonymous". Regression for the upstream diff finding B2.
    #[test]
    fn anonymous_ehr_status_subject_is_accepted() {
        for subject in [json!({}), json!({ "_type": "PARTY_SELF" })] {
            let status = json!({
                "_type": "EHR_STATUS",
                "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
                "name": { "_type": "DV_TEXT", "value": "EHR Status" },
                "subject": subject,
                "is_queryable": true,
                "is_modifiable": true
            });
            validate_ehr_status(&status).expect("anonymous PARTY_SELF EHR_STATUS");
        }
    }

    /// Every vendored `EHR_STATUS` data set the CNF corpus labels invalid
    /// (`master06 §Test Data Sets`, INVALID class 2) must be rejected — with one
    /// spec-cited exception.
    ///
    /// `001_ehr_status_subject_empty.json` (`subject: {}`) is labelled "invalid"
    /// by the corpus but is **spec-valid**: RM ehr master04 §EHR Status makes an
    /// empty `PARTY_SELF` a *completely anonymous* subject. Per the vendored spec
    /// oracle (the authority, ADR-008) this is a corpus mislabelling, handled here
    /// as a documented adjudication (the fixture is asserted *accepted*, not
    /// silently skipped). See `docs/conformance/upstream-ehrbase/TRIAGE.md` §B2.
    /// The other ten fixtures stay asserted-rejected.
    #[test]
    fn every_invalid_ehr_status_fixture_is_rejected() {
        // Corpus-vs-spec adjudication: an empty PARTY_SELF is a valid anonymous
        // subject (master04), so this "invalid"-labelled fixture is spec-valid.
        const SPEC_VALID_ANONYMOUS: &str = "001_ehr_status_subject_empty.json";
        let dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/specs/openehr/CNF/tests/platform/robot/_resources/test_data_sets/ehr/invalid"
        );
        let mut checked = 0u32;
        for entry in std::fs::read_dir(dir).expect("read ehr/invalid") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("read fixture");
            let status: Value = serde_json::from_str(&text).expect("parse fixture");
            let is_anon = path.file_name().and_then(|n| n.to_str()) == Some(SPEC_VALID_ANONYMOUS);
            if is_anon {
                validate_ehr_status(&status).unwrap_or_else(|e| {
                    panic!(
                        "spec-valid anonymous EHR_STATUS ({SPEC_VALID_ANONYMOUS}) was rejected: {e}"
                    )
                });
            } else {
                assert!(
                    validate_ehr_status(&status).is_err(),
                    "invalid EHR_STATUS fixture was accepted: {}",
                    path.display()
                );
            }
            checked += 1;
        }
        assert_eq!(checked, 11, "expected 11 invalid EHR_STATUS fixtures");
    }
}
