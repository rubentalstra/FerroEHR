//! EHR Extract export (SM `I_EHR_EXTRACT_SERVICE.export_ehrs` /
//! `export_ehr_extracts`) over the greenfield versioned store (ADR-008).
//!
//! Spec: SM `docs/specs/openehr/SM/docs/UML/classes/i_ehr_extract_service.adoc`;
//! RM EHR Extract IM `docs/specs/openehr/RM/docs/ehr_extract/` — master09
//! §Creation Semantics (the extract-building algorithm) and master05
//! (`X_VERSIONED_OBJECT` / the five `X_VERSIONED_*` wrappers); the generated RM
//! types in `crates/openehr-rm/src/ehr_extract/`. Design digest:
//! `docs/design/sm-platform/10-message-integration.md` §2 (the `ehrbase`
//! component's `message`/`extract` module).
//!
//! An exported `EXTRACT` is a canonical-JSON `Value` built directly over the
//! stored versions — the same idiom every other read surface uses
//! (`service::versioned` builds `ORIGINAL_VERSION`/`VERSIONED_OBJECT`/
//! `REVISION_HISTORY` as `json!` values). Each versioned object of the EHR
//! becomes one `OPENEHR_CONTENT_ITEM` wrapping an `X_VERSIONED_<kind>`, whose
//! `versions` are the exact `ORIGINAL_VERSION`s the read path serves (so the
//! extract content is byte-identical to what `GET .../versioned_*` returns).
//!
//! The **import** side (`import_ehr`/`import_ehr_extract`) is the inverse: it
//! replays each received `X_VERSIONED_*`'s `ORIGINAL_VERSION`s through the
//! versioned-object commit machinery ([`super::vobject::commit_import`]) as
//! `IMPORTED_VERSION`s — a fresh local import CONTRIBUTION records the local act
//! of committal (`249|creation|`), while the wrapped original's identity, commit
//! audit, lifecycle, data and signature are preserved verbatim (RM common
//! master06 §Copying/§Committal; the `commit_import` PORT NOTEs record the exact
//! `IMPORTED_VERSION` representation + version-branching = typed rejection).
//! `import_ehr` clones a whole EHR into an empty target (fixed id, else the
//! source EHR id reused — master06 §Copying Case 1); `import_ehr_extract` lands
//! versioned objects into an existing EHR (Cases 2/3).
//!
//! PORT NOTE (export refinements not yet applied): master09's demographic-chapter
//! resolution (`PARTY` `OBJECT_REF` following), `DV_MULTIMEDIA` include/exclude,
//! and `link_depth` `DV_LINK` following: every openEHR versioned object of the
//! EHR goes into a single `EXTRACT_CHAPTER` as a primary content item
//! (`is_primary = true`). These master09 refinements land with the
//! demographic/query-integration waves.
//!
//! PORT NOTE (import scope): COMPOSITION import does not re-link its OPT
//! (`vo_version.template_id` stays NULL — the OPT must already be provisioned in
//! the target through the DEFINITION API; imported content is stored verbatim
//! without re-validation, like the admin dump/load path). Demographic
//! (`X_VERSIONED_PARTY`) and generic (`GENERIC_CONTENT_ITEM`, ISO 13606/CDA)
//! content is not importable through the EHR surface; the promoted
//! `ehr.subject_id` column is left unset for an imported EHR (a clone shares the
//! source subject, which the one-EHR-per-subject index cannot represent — the
//! subject is preserved inside the `EHR_STATUS` content).
//!
//! PORT NOTE (synthetic archetype ids): the openEHR extract wrapper classes
//! (`EXTRACT`, `EXTRACT_CHAPTER`, `OPENEHR_CONTENT_ITEM`) are `LOCATABLE`s whose
//! `archetype_node_id` (1..1) must be present, yet this server *synthesizes*
//! these structural nodes with no generating archetype. We emit the RM class
//! token (`"EXTRACT"` etc.) as a self-descriptive placeholder rather than
//! fabricate a fake archetype identifier — a deliberate deviation, since no
//! archetype exists for a programmatically-built extract skeleton.

use std::collections::BTreeMap;

use async_trait::async_trait;
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use ehrbase_sm::{CallStatusType, EhrExtractService, SmError};
use openehr_rm::ehr_extract::common::extract::Extract;
use openehr_rm::ehr_extract::common::extract_spec::ExtractSpec;

use super::codes::{self, change_type};
use super::version_id;
use super::vobject::{self, AuditInput, ImportContainer, ImportVersion, Kind};
use crate::service::{EhrbaseService, ServiceError};

/// Which versions of a versioned object an extract includes, resolved from an
/// `EXTRACT_VERSION_SPEC` (`extract_version_spec.adoc`). The default (no spec)
/// is latest-only with data and no revision history.
#[derive(Debug, Clone, Copy)]
struct VersionSelection {
    /// `include_all_versions` — include every version, not just the latest.
    include_all: bool,
    /// `include_revision_history` — attach the full `REVISION_HISTORY`.
    include_revision_history: bool,
    /// `include_data` — include the version data; `false` ⇒ revision history
    /// only, `versions` empty (`X_VERSIONED_OBJECT.extract_version_count = 0`).
    include_data: bool,
}

impl VersionSelection {
    /// The `EXTRACT_VERSION_SPEC` default: latest-only, data included, no
    /// revision history (`extract_version_spec.adoc`: "By default, only latest
    /// versions are included").
    const fn latest_only() -> Self {
        Self {
            include_all: false,
            include_revision_history: false,
            include_data: true,
        }
    }
}

/// The `X_VERSIONED_*` `_type` for a stored versioned-object `kind` (master05
/// §`openehr_extract` package: the five data-oriented `VERSIONED_OBJECT`
/// wrappers). A kind with no dedicated wrapper (e.g. `PARTY_RELATIONSHIP`, which
/// is never EHR-scoped) falls back to the generic `X_VERSIONED_OBJECT`.
fn x_versioned_type(kind: &str) -> &'static str {
    match kind {
        "COMPOSITION" => "X_VERSIONED_COMPOSITION",
        "EHR_STATUS" => "X_VERSIONED_EHR_STATUS",
        "EHR_ACCESS" => "X_VERSIONED_EHR_ACCESS",
        "FOLDER" => "X_VERSIONED_FOLDER",
        "AGENT" | "GROUP" | "ORGANISATION" | "PERSON" | "ROLE" => "X_VERSIONED_PARTY",
        _ => "X_VERSIONED_OBJECT",
    }
}

impl EhrbaseService {
    /// Whether an EHR with `ehr_id` exists (the design-filled `has_ehr`
    /// precondition of both export calls).
    async fn extract_ehr_exists(&self, ehr_id: Uuid) -> Result<bool, ServiceError> {
        Ok(
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM ehr WHERE id = $1)")
                .bind(ehr_id)
                .fetch_one(&self.pool)
                .await?,
        )
    }

    /// The `(vo_id, kind)` of every versioned object currently in the EHR (one
    /// row per version container — its current, `upper_inf`, version), ordered
    /// by id for a deterministic extract.
    async fn ehr_versioned_objects(
        &self,
        ehr_id: Uuid,
    ) -> Result<Vec<(Uuid, String)>, ServiceError> {
        let rows = sqlx::query(
            "SELECT vo_id, kind FROM vo_version \
             WHERE ehr_id = $1 AND upper_inf(sys_period) AND branch_number = 0 \
             ORDER BY vo_id",
        )
        .bind(ehr_id)
        .fetch_all(&self.pool)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push((r.try_get("vo_id")?, r.try_get("kind")?));
        }
        Ok(out)
    }

    /// The kind of a version container, if it belongs to `ehr_id` (used to
    /// resolve an `EXTRACT_ENTITY_MANIFEST.item_list` reference).
    async fn ehr_vo_kind(&self, ehr_id: Uuid, vo_id: Uuid) -> Result<Option<String>, ServiceError> {
        Ok(sqlx::query_scalar(
            "SELECT kind FROM vo_version \
             WHERE vo_id = $1 AND ehr_id = $2 AND upper_inf(sys_period) \
             AND branch_number = 0",
        )
        .bind(vo_id)
        .bind(ehr_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// The `sys_version`s of a version container, in order, each flagged with
    /// whether it is the current (`upper_inf`) version.
    async fn vo_version_numbers(&self, vo_id: Uuid) -> Result<Vec<(i32, bool)>, ServiceError> {
        let rows = sqlx::query(
            "SELECT sys_version, (upper_inf(sys_period) AND branch_number = 0) AS is_current \
             FROM vo_version WHERE vo_id = $1 ORDER BY sys_version",
        )
        .bind(vo_id)
        .fetch_all(&self.pool)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push((r.try_get("sys_version")?, r.try_get("is_current")?));
        }
        Ok(out)
    }

    /// Build one `OPENEHR_CONTENT_ITEM` wrapping the `X_VERSIONED_<kind>` for a
    /// version container (master05). `versions` are the exact `ORIGINAL_VERSION`s
    /// the read path serves; `total_version_count` counts all stored versions,
    /// `extract_version_count` the included ones.
    async fn build_openehr_content_item(
        &self,
        ehr_id: Uuid,
        vo_id: Uuid,
        kind: &str,
        sel: VersionSelection,
    ) -> Result<Value, ServiceError> {
        let all = self.vo_version_numbers(vo_id).await?;
        let total = i32::try_from(all.len()).unwrap_or(i32::MAX);

        // Which versions to serialise: none if data is excluded, else all or the
        // latest only (EXTRACT_VERSION_SPEC).
        let selected: Vec<i32> = if !sel.include_data {
            Vec::new()
        } else if sel.include_all {
            all.iter().map(|(sv, _)| *sv).collect()
        } else {
            all.iter()
                .filter(|(_, cur)| *cur)
                .map(|(sv, _)| *sv)
                .collect()
        };

        let mut versions = Vec::with_capacity(selected.len());
        for sv in &selected {
            let read = super::vobject::read_version_by_ordinal(&self.pool, vo_id, *sv)
                .await?
                .ok_or_else(|| {
                    ServiceError::NotFound(format!("version {vo_id}::{sv} for extract"))
                })?;
            versions.push(self.original_version(&read)?);
        }
        let extract_version_count = i32::try_from(versions.len()).unwrap_or(i32::MAX);

        // uid / owner_id / time_created are the VERSIONED_OBJECT's own — reuse
        // the shared builder so they match the /versioned_* read surface.
        let vo = self.versioned_object(vo_id, ehr_id).await?;
        let mut x = json!({
            "_type": x_versioned_type(kind),
            "uid": vo["uid"].clone(),
            "owner_id": vo["owner_id"].clone(),
            "time_created": vo["time_created"].clone(),
            "total_version_count": total,
            "extract_version_count": extract_version_count,
            "versions": versions,
        });
        if sel.include_revision_history
            && let Value::Object(map) = &mut x
        {
            map.insert(
                "revision_history".to_owned(),
                self.revision_history(ehr_id, vo_id).await?,
            );
        }

        Ok(json!({
            "_type": "OPENEHR_CONTENT_ITEM",
            "name": { "_type": "DV_TEXT", "value": kind },
            "archetype_node_id": "OPENEHR_CONTENT_ITEM",
            "is_primary": true,
            "item": x,
        }))
    }

    /// Assemble the top-level `EXTRACT` from its content items and the
    /// `EXTRACT_SPEC` that reflects the actual content (`extract.adoc`;
    /// `specification` "might not be identical with the specification of the
    /// corresponding request").
    fn assemble_extract(
        &self,
        content_items: Vec<Value>,
        specification: Value,
        sequence_nr: i32,
    ) -> Value {
        let mut extract = json!({
            "_type": "EXTRACT",
            "name": { "_type": "DV_TEXT", "value": "EHR Extract" },
            "archetype_node_id": "EXTRACT",
            "chapters": [ {
                "_type": "EXTRACT_CHAPTER",
                "name": { "_type": "DV_TEXT", "value": "openEHR content" },
                "archetype_node_id": "EXTRACT_CHAPTER",
                "items": [],
            } ],
            "time_created": {
                "_type": "DV_DATE_TIME",
                "value": jiff::Timestamp::now().to_string(),
            },
            "system_id": { "_type": "HIER_OBJECT_ID", "value": self.effective_system_id() },
            "sequence_nr": sequence_nr,
        });
        // Move the owned content items + specification in (avoids a needless clone).
        extract["chapters"][0]["items"] = Value::Array(content_items);
        extract["specification"] = specification;
        extract
    }

    /// A synthetic `EXTRACT_SPEC` describing a whole-EHR, latest-only export (for
    /// `export_ehrs`, which takes no spec) — one entity keyed by the EHR id.
    fn whole_ehr_spec(ehr_id: Uuid) -> Value {
        json!({
            "_type": "EXTRACT_SPEC",
            "version_spec": {
                "_type": "EXTRACT_VERSION_SPEC",
                "include_all_versions": false,
                "include_revision_history": false,
                "include_data": true,
            },
            "manifest": {
                "_type": "EXTRACT_MANIFEST",
                "entities": [ {
                    "_type": "EXTRACT_ENTITY_MANIFEST",
                    "extract_id_key": ehr_id.to_string(),
                    "ehr_id": ehr_id.to_string(),
                    "other_ids": [],
                    "item_list": [],
                } ],
            },
            "extract_type": {
                "_type": "DV_CODED_TEXT",
                "value": "openehr-ehr",
                "defining_code": {
                    "_type": "CODE_PHRASE",
                    "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                    "code_string": "openehr-ehr",
                },
            },
            "include_multimedia": true,
            "priority": 0,
            "link_depth": 0,
            "criteria": [],
        })
    }

    /// Build a single whole-EHR `EXTRACT` (every versioned object of the EHR,
    /// latest version only) — the body of [`export_ehrs`](EhrExtractService::export_ehrs).
    async fn export_whole_ehr(
        &self,
        ehr_id: Uuid,
        sequence_nr: i32,
    ) -> Result<Value, ServiceError> {
        let sel = VersionSelection::latest_only();
        let vos = self.ehr_versioned_objects(ehr_id).await?;
        let mut items = Vec::with_capacity(vos.len());
        for (vo_id, kind) in vos {
            items.push(
                self.build_openehr_content_item(ehr_id, vo_id, &kind, sel)
                    .await?,
            );
        }
        Ok(self.assemble_extract(items, Self::whole_ehr_spec(ehr_id), sequence_nr))
    }
}

/// Read the `EXTRACT_VERSION_SPEC` of a request into a [`VersionSelection`],
/// rejecting the not-yet-supported `commit_time_interval` selector and enforcing
/// the spec invariant that excluding data requires including revision history
/// (`extract_version_spec.adoc`).
fn version_selection(spec: &ExtractSpec) -> Result<VersionSelection, SmError> {
    let Some(vs) = spec.version_spec.as_ref() else {
        return Ok(VersionSelection::latest_only());
    };
    if vs.commit_time_interval.is_some() {
        return Err(SmError::precondition(
            "EXTRACT_VERSION_SPEC.commit_time_interval is not supported in this stage",
        ));
    }
    // include_data = false ⇒ revision-history-only; the spec requires the
    // revision history to then be present.
    if !vs.include_data && !vs.include_revision_history {
        return Err(SmError::precondition(
            "include_data = false requires include_revision_history = true \
             (EXTRACT_VERSION_SPEC: data excluded ⇒ revision-history-only)",
        ));
    }
    Ok(VersionSelection {
        include_all: vs.include_all_versions,
        include_revision_history: vs.include_revision_history,
        include_data: vs.include_data,
    })
}

/// Resolve an `EXTRACT_ENTITY_MANIFEST` to a concrete EHR id: prefer `ehr_id`,
/// else look the EHR up by `subject_id`.
async fn resolve_entity_ehr(
    svc: &EhrbaseService,
    ehr_id: Option<&str>,
    subject_id: Option<&str>,
) -> Result<Uuid, SmError> {
    if let Some(raw) = ehr_id {
        let id: Uuid = raw.parse().map_err(|_| {
            SmError::precondition(format!(
                "EXTRACT_ENTITY_MANIFEST.ehr_id {raw:?} is not a UUID"
            ))
        })?;
        if svc.extract_ehr_exists(id).await? {
            return Ok(id);
        }
        return Err(SmError::ehr_not_found(format!("no EHR with id {id}")));
    }
    if let Some(subject) = subject_id {
        let id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM ehr WHERE subject_id = $1")
            .bind(subject)
            .fetch_optional(&svc.pool)
            .await
            .map_err(ServiceError::from)?;
        return id
            .ok_or_else(|| SmError::ehr_not_found(format!("no EHR for subject_id {subject:?}")));
    }
    Err(SmError::precondition(
        "EXTRACT_ENTITY_MANIFEST identifies no entity (neither ehr_id nor subject_id)",
    ))
}

#[async_trait]
impl EhrExtractService for EhrbaseService {
    async fn export_ehrs(&self, an_ehr_id: Uuid) -> Result<Vec<Value>, SmError> {
        if !self.extract_ehr_exists(an_ehr_id).await? {
            return Err(SmError::ehr_not_found(format!(
                "no EHR with id {an_ehr_id}"
            )));
        }
        Ok(vec![self.export_whole_ehr(an_ehr_id, 1).await?])
    }

    async fn export_ehr_extracts(&self, extract_spec: ExtractSpec) -> Result<Vec<Value>, SmError> {
        let sel = version_selection(&extract_spec)?;
        let criteria_present = !extract_spec.criteria.is_empty();
        let spec_value = serde_json::to_value(&extract_spec).map_err(ServiceError::from)?;

        let mut out = Vec::with_capacity(extract_spec.manifest.entities.len());
        for (idx, entity) in extract_spec.manifest.entities.iter().enumerate() {
            let ehr_id =
                resolve_entity_ehr(self, entity.ehr_id.as_deref(), entity.subject_id.as_deref())
                    .await?;

            // The primary set: an explicit item_list, else every VO of the EHR.
            // Criteria (AQL) primary-set selection is not applied this stage —
            // rejected rather than silently over-exported (design §2 PORT NOTE).
            let vo_kinds: Vec<(Uuid, String)> = if entity.item_list.is_empty() {
                if criteria_present {
                    return Err(SmError::precondition(
                        "EXTRACT_SPEC.criteria (AQL selection) is not supported in this stage; \
                         provide EXTRACT_ENTITY_MANIFEST.item_list instead",
                    ));
                }
                self.ehr_versioned_objects(ehr_id).await?
            } else {
                let mut resolved = Vec::with_capacity(entity.item_list.len());
                for obj_ref in &entity.item_list {
                    let value = serde_json::to_value(obj_ref).map_err(ServiceError::from)?;
                    let raw = value
                        .pointer("/id/value")
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            SmError::precondition("item_list OBJECT_REF has no id.value")
                        })?;
                    let vo_id: Uuid = raw.parse().map_err(|_| {
                        SmError::precondition(format!(
                            "item_list id {raw:?} is not a version-container UUID"
                        ))
                    })?;
                    let kind = self.ehr_vo_kind(ehr_id, vo_id).await?.ok_or_else(|| {
                        SmError::new(
                            CallStatusType::VersionedObjectDoesNotExist,
                            format!("version container {vo_id} not found in EHR {ehr_id}"),
                        )
                    })?;
                    resolved.push((vo_id, kind));
                }
                resolved
            };

            let mut items = Vec::with_capacity(vo_kinds.len());
            for (vo_id, kind) in vo_kinds {
                items.push(
                    self.build_openehr_content_item(ehr_id, vo_id, &kind, sel)
                        .await?,
                );
            }
            let seq = i32::try_from(idx + 1).unwrap_or(i32::MAX);
            out.push(self.assemble_extract(items, spec_value.clone(), seq));
        }
        Ok(out)
    }

    async fn import_ehr(
        &self,
        an_ehr_id: Option<Uuid>,
        an_extract: Extract,
    ) -> Result<(), SmError> {
        let containers = parse_import_containers(&an_extract)?;
        // A whole-EHR clone must carry an EHR_STATUS (EHR.ehr_status 1..1, RM ehr
        // §"EHR Creation") — the EHR could not otherwise be a valid EHR.
        if !containers.iter().any(|c| c.kind == Kind::EhrStatus) {
            return Err(SmError::precondition(
                "import_ehr requires the extract to carry an EHR_STATUS versioned object",
            ));
        }
        reject_duplicate_singleton_containers(&containers)?;

        // The target id: the caller's fixed id (the SM's "same patient in other
        // EHR services" case), else the source EHR id reused (master06 §Copying:
        // "the newly created EHR should re-use the EHR identifier from the source
        // system"; RM ehr §"EHR Identifier Allocation").
        let ehr_id = match an_ehr_id {
            Some(id) => id,
            None => source_ehr_id(&an_extract)?,
        };

        let mut tx = self.pool.begin().await.map_err(ServiceError::from)?;
        // Into an *empty* target: a duplicate EHR id is `ehr_create_fail_duplicate_id`.
        // The EHR is created locally, so its immutable system_id is ours (req 2.1).
        let inserted =
            sqlx::query("INSERT INTO ehr (id, system_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                .bind(ehr_id)
                .bind(self.effective_system_id())
                .execute(&mut *tx)
                .await
                .map_err(ServiceError::from)?;
        if inserted.rows_affected() == 0 {
            return Err(SmError::new(
                CallStatusType::EhrCreateFailDuplicateId,
                format!("EHR {ehr_id} already exists; import_ehr requires an empty target"),
            ));
        }
        let audit = self.audit(change_type::CREATION, "EHR Extract import");
        vobject::commit_import(&mut tx, ehr_id, &audit, containers).await?;
        tx.commit().await.map_err(ServiceError::from)?;
        Ok(())
    }

    async fn import_ehr_extract(
        &self,
        an_ehr_id: Uuid,
        an_extract: Extract,
    ) -> Result<(), SmError> {
        if !self.extract_ehr_exists(an_ehr_id).await? {
            return Err(SmError::ehr_not_found(format!(
                "no EHR with id {an_ehr_id}"
            )));
        }
        let containers = parse_import_containers(&an_extract)?;
        reject_duplicate_singleton_containers(&containers)?;

        // A *new* singleton container cannot be added when the EHR already holds
        // one of that kind under a different object id (EHR.ehr_status 1..1,
        // EHR.directory 0..1 — RM ehr, EHR class). A matching object id is an
        // append (master06 §Copying Case 3), handled in `commit_import`.
        for container in &containers {
            if matches!(
                container.kind,
                Kind::EhrStatus | Kind::EhrAccess | Kind::Folder
            ) && let Some((existing_vo, _)) = self.current_vo(an_ehr_id, container.kind).await?
                && existing_vo != container.vo_id
            {
                return Err(ServiceError::Conflict(format!(
                    "EHR {an_ehr_id} already has a {} ({existing_vo}); cannot import a \
                     second one ({})",
                    container.kind.as_str(),
                    container.vo_id
                ))
                .into());
            }
        }

        let mut tx = self.pool.begin().await.map_err(ServiceError::from)?;
        let audit = self.audit(change_type::CREATION, "EHR Extract import");
        vobject::commit_import(&mut tx, an_ehr_id, &audit, containers).await?;
        tx.commit().await.map_err(ServiceError::from)?;
        Ok(())
    }
}

/// Reverse of [`x_versioned_type`]: the versioned-object [`Kind`] a
/// `X_VERSIONED_*` wrapper carries, for the EHR-scoped kinds an import replays.
/// A generic `X_VERSIONED_OBJECT` / `X_VERSIONED_PARTY` wrapper (demographic /
/// non-EHR content) is not importable through the EHR surface.
fn kind_from_x_versioned(xtype: &str) -> Option<Kind> {
    match xtype {
        "X_VERSIONED_COMPOSITION" => Some(Kind::Composition),
        "X_VERSIONED_EHR_STATUS" => Some(Kind::EhrStatus),
        "X_VERSIONED_EHR_ACCESS" => Some(Kind::EhrAccess),
        "X_VERSIONED_FOLDER" => Some(Kind::Folder),
        _ => None,
    }
}

/// The source EHR id an `import_ehr` clone reuses when no fixed id is given —
/// `EXTRACT_SPEC.manifest.entities[0].ehr_id` (a whole-EHR export always names
/// it; [`EhrbaseService::whole_ehr_spec`]).
fn source_ehr_id(extract: &Extract) -> Result<Uuid, SmError> {
    let raw = extract
        .specification
        .as_ref()
        .and_then(|s| s.manifest.entities.first())
        .and_then(|e| e.ehr_id.as_deref())
        .ok_or_else(|| {
            SmError::precondition(
                "import_ehr without a fixed ehr_id requires the extract's EXTRACT_SPEC to \
                 name the source EHR id (specification.manifest.entities[0].ehr_id)",
            )
        })?;
    raw.parse()
        .map_err(|_| SmError::precondition(format!("source ehr_id {raw:?} is not a UUID")))
}

/// An EHR holds at most one of each singleton versioned object (`EHR_STATUS`,
/// `EHR_ACCESS`, directory `FOLDER` — RM ehr, EHR class); an extract that carries
/// two distinct containers of one such kind cannot be imported.
fn reject_duplicate_singleton_containers(containers: &[ImportContainer]) -> Result<(), SmError> {
    for singleton in [Kind::EhrStatus, Kind::EhrAccess, Kind::Folder] {
        if containers.iter().filter(|c| c.kind == singleton).count() > 1 {
            return Err(SmError::precondition(format!(
                "extract carries more than one {} versioned object; an EHR holds at most one",
                singleton.as_str()
            )));
        }
    }
    Ok(())
}

/// Parse a received `EXTRACT` into the set of versioned objects to import,
/// grouped by cloned `vo_id` (the received `uid.object_id()`). Each content
/// item's `X_VERSIONED_*` wrapper contributes its `ORIGINAL_VERSION`s to one
/// [`ImportContainer`]; a container's kind and originating `creating_system_id`
/// must be consistent across its versions (trunk-only, single-origin —
/// PORT NOTE F-06-09: branch / multi-system version trees are rejected).
fn parse_import_containers(extract: &Extract) -> Result<Vec<ImportContainer>, SmError> {
    let value = serde_json::to_value(extract).map_err(ServiceError::from)?;
    let empty: Vec<Value> = Vec::new();
    let mut by_container: BTreeMap<Uuid, ImportContainer> = BTreeMap::new();

    for chapter in value
        .get("chapters")
        .and_then(Value::as_array)
        .unwrap_or(&empty)
    {
        for item in chapter
            .get("items")
            .and_then(Value::as_array)
            .unwrap_or(&empty)
        {
            match item.get("_type").and_then(Value::as_str) {
                Some("OPENEHR_CONTENT_ITEM") => {}
                Some("GENERIC_CONTENT_ITEM") => {
                    return Err(SmError::precondition(
                        "generic (ISO 13606 / CDA) content import is not supported",
                    ));
                }
                // A folder structure entry carries no versioned content.
                _ => continue,
            }
            let Some(xver) = item.get("item") else {
                continue;
            };
            let xtype = xver
                .get("_type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let kind = kind_from_x_versioned(xtype).ok_or_else(|| {
                SmError::precondition(format!(
                    "cannot import {xtype:?} through the EHR surface (only COMPOSITION / \
                     EHR_STATUS / EHR_ACCESS / FOLDER)"
                ))
            })?;

            for ov in xver
                .get("versions")
                .and_then(Value::as_array)
                .unwrap_or(&empty)
            {
                // creating_system_id is per VERSION (a copied tree legitimately
                // mixes source-trunk versions with branch modifications made by
                // other systems — RM common master06 §Distributed versioning).
                let (vo_id, version) = parse_imported_version(ov)?;
                match by_container.get_mut(&vo_id) {
                    Some(existing) => {
                        if existing.kind != kind {
                            return Err(SmError::precondition(format!(
                                "versioned object {vo_id} appears as both {} and {}",
                                existing.kind.as_str(),
                                kind.as_str()
                            )));
                        }
                        existing.versions.push(version);
                    }
                    None => {
                        by_container.insert(
                            vo_id,
                            ImportContainer {
                                vo_id,
                                kind,
                                versions: vec![version],
                            },
                        );
                    }
                }
            }
        }
    }
    Ok(by_container.into_values().collect())
}

/// Parse one received `ORIGINAL_VERSION` into its cloned `vo_id` and the
/// [`ImportVersion`] to replay — preserving the wrapped original's full 3-part
/// identity (incl. branch `version_tree_id`s), `preceding_version_uid`,
/// `other_input_version_uids`, `commit_audit`, lifecycle, data, signature and
/// attestations verbatim (RM common master06 §Copying: "the `ORIGINAL_VERSION`
/// instance is never modified").
fn parse_imported_version(ov: &Value) -> Result<(Uuid, ImportVersion), SmError> {
    // `X_VERSIONED_OBJECT.versions` carries ORIGINAL_VERSIONs — the received
    // instance "is never modified" and is re-wrapped as IMPORTED_VERSION *by the
    // importer* (RM common master06 §Copying; ehr_extract master05
    // `X_VERSIONED_OBJECT.versions: List<ORIGINAL_VERSION>`). A member typed
    // anything else (e.g. an already-wrapped IMPORTED_VERSION) is invalid.
    match ov.get("_type").and_then(Value::as_str) {
        None | Some("ORIGINAL_VERSION") => {}
        Some(other) => {
            return Err(SmError::precondition(format!(
                "X_VERSIONED_OBJECT.versions members must be ORIGINAL_VERSION \
                 (RM ehr_extract master05), got _type {other:?}"
            )));
        }
    }
    let uid = ov
        .pointer("/uid/value")
        .and_then(Value::as_str)
        .ok_or_else(|| SmError::precondition("imported ORIGINAL_VERSION has no uid.value"))?;
    let (vo_id, creating_system_id, tree) = version_id::parse_object_version_id(uid)?;
    // preceding_version_uid + other_input_version_uids preserved verbatim
    // (master06 §Copying / §Version Merging).
    let preceding_version_uid = ov
        .pointer("/preceding_version_uid/value")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let other_input_version_uids: Vec<String> = ov
        .get("other_input_version_uids")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|u| {
                    u.as_str()
                        .or_else(|| u.pointer("/value").and_then(Value::as_str))
                        .map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default();

    // commit_audit (VERSION.commit_audit 1..1) preserved verbatim.
    let audit = ov
        .get("commit_audit")
        .ok_or_else(|| SmError::precondition("imported ORIGINAL_VERSION has no commit_audit"))?;
    let system_id = audit
        .get("system_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            SmError::precondition("imported commit_audit.system_id is required and non-empty")
        })?
        .to_owned();
    let change_token = audit
        .pointer("/change_type/defining_code/code_string")
        .and_then(Value::as_str)
        .or_else(|| audit.pointer("/change_type/value").and_then(Value::as_str))
        .ok_or_else(|| SmError::precondition("imported commit_audit.change_type is required"))?;
    let change_type = codes::change_type_code(change_token).ok_or_else(|| {
        SmError::precondition(format!(
            "imported commit_audit.change_type {change_token:?} is not an audit_change_type code"
        ))
    })?;
    let committer = audit.get("committer").cloned().ok_or_else(|| {
        SmError::precondition("imported commit_audit.committer is required (AUDIT_DETAILS 1..1)")
    })?;
    let description = audit
        .pointer("/description/value")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let time_str = audit
        .pointer("/time_committed/value")
        .and_then(Value::as_str)
        .ok_or_else(|| SmError::precondition("imported commit_audit.time_committed is required"))?;
    let commit_time: jiff::Timestamp = time_str.parse().map_err(|_| {
        SmError::precondition(format!(
            "imported commit_audit.time_committed {time_str:?} is not an ISO 8601 instant"
        ))
    })?;

    // lifecycle_state (ORIGINAL_VERSION.lifecycle_state) resolved to its code.
    let lifecycle_token = ov
        .pointer("/lifecycle_state/defining_code/code_string")
        .and_then(Value::as_str)
        .or_else(|| ov.pointer("/lifecycle_state/value").and_then(Value::as_str))
        .unwrap_or(codes::lifecycle::COMPLETE);
    let lifecycle_state = codes::lifecycle_state_code(lifecycle_token).ok_or_else(|| {
        SmError::precondition(format!(
            "imported lifecycle_state {lifecycle_token:?} is not a version_lifecycle_state code"
        ))
    })?;

    // data: Void (absent/null) exactly for a 523|deleted| version (master06
    // §"Logical Deletion").
    let data = ov
        .get("data")
        .cloned()
        .filter(|d| !d.is_null())
        .unwrap_or(Value::Null);
    let deleted = lifecycle_state == codes::lifecycle::DELETED;
    if deleted && !data.is_null() {
        return Err(SmError::precondition(
            "imported 523|deleted| version must not carry data (data is Void)",
        ));
    }
    if !deleted && data.is_null() {
        return Err(SmError::precondition(
            "imported non-deleted ORIGINAL_VERSION requires data",
        ));
    }

    let signature = ov
        .get("signature")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let attestations = ov
        .get("attestations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    Ok((
        vo_id,
        ImportVersion {
            tree,
            creating_system_id,
            preceding_version_uid,
            other_input_version_uids,
            lifecycle_state,
            commit_audit: AuditInput {
                system_id,
                change_type,
                description,
                committer,
            },
            commit_time,
            data,
            signature,
            attestations,
        },
    ))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use serde_json::json;

    use super::*;

    /// A minimal spec-shaped `ORIGINAL_VERSION` wire value for the import parser.
    fn original_version(type_field: Option<&str>) -> Value {
        let mut ov = json!({
            "uid": { "_type": "OBJECT_VERSION_ID",
                     "value": "018f4a5e-9df1-7d1e-8b6f-2b8c00000001::sysA.example.org::1" },
            "commit_audit": {
                "_type": "AUDIT_DETAILS",
                "system_id": "sysA.example.org",
                "time_committed": { "_type": "DV_DATE_TIME", "value": "2026-07-11T10:00:00Z" },
                "change_type": { "_type": "DV_CODED_TEXT", "value": "creation",
                                 "defining_code": { "_type": "CODE_PHRASE", "code_string": "249",
                                                    "terminology_id": { "_type": "TERMINOLOGY_ID",
                                                                        "value": "openehr" } } },
                "committer": { "_type": "PARTY_IDENTIFIED", "name": "Dr A" }
            },
            "lifecycle_state": { "_type": "DV_CODED_TEXT", "value": "complete",
                                 "defining_code": { "_type": "CODE_PHRASE", "code_string": "532",
                                                    "terminology_id": { "_type": "TERMINOLOGY_ID",
                                                                        "value": "openehr" } } },
            "data": { "_type": "EHR_STATUS" }
        });
        if let Some(t) = type_field {
            ov.as_object_mut().unwrap().insert("_type".into(), json!(t));
        }
        ov
    }

    /// `X_VERSIONED_OBJECT.versions` members must be `ORIGINAL_VERSION` (RM
    /// ehr_extract master05); a foreign `_type` — e.g. an already-wrapped
    /// `IMPORTED_VERSION` — is rejected, while an explicit or absent
    /// `ORIGINAL_VERSION` tag parses. Regression for A1
    /// rm-common-change-control-R20.
    #[test]
    fn imported_versions_member_type_is_enforced() {
        parse_imported_version(&original_version(None)).expect("absent _type defaults");
        parse_imported_version(&original_version(Some("ORIGINAL_VERSION")))
            .expect("explicit ORIGINAL_VERSION");
        for foreign in ["IMPORTED_VERSION", "VERSION", "ORIGINAL_VERSION2"] {
            let err = parse_imported_version(&original_version(Some(foreign)))
                .expect_err("foreign _type in versions[] must be rejected");
            assert!(
                err.message.contains("ORIGINAL_VERSION") && err.message.contains(foreign),
                "error should name the expected and offending types, got: {}",
                err.message
            );
        }
    }
}
