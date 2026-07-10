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
//! PORT NOTE (scope, first SM-5 wave — export only): only the two `export_*`
//! calls are built here; `import_ehr`/`import_ehr_extract` return
//! `not_implemented` (the import/`IMPORTED_VERSION` stage,
//! `docs/plans/b3-sm-services.md` task 2). Within export, master09's
//! demographic-chapter resolution (`PARTY` `OBJECT_REF` following),
//! `DV_MULTIMEDIA` include/exclude, and `link_depth` `DV_LINK` following are not
//! yet applied: every openEHR versioned object of the EHR goes into a single
//! `EXTRACT_CHAPTER` as a primary content item (`is_primary = true`). These
//! master09 refinements land with the demographic/query-integration waves.
//!
//! PORT NOTE (synthetic archetype ids): the openEHR extract wrapper classes
//! (`EXTRACT`, `EXTRACT_CHAPTER`, `OPENEHR_CONTENT_ITEM`) are `LOCATABLE`s whose
//! `archetype_node_id` (1..1) must be present, yet this server *synthesizes*
//! these structural nodes with no generating archetype. We emit the RM class
//! token (`"EXTRACT"` etc.) as a self-descriptive placeholder rather than
//! fabricate a fake archetype identifier — a deliberate deviation, since no
//! archetype exists for a programmatically-built extract skeleton.

use async_trait::async_trait;
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use ehrbase_sm::{CallStatusType, EhrExtractService, SmError};
use openehr_rm::ehr_extract::common::extract::Extract;
use openehr_rm::ehr_extract::common::extract_spec::ExtractSpec;

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
             WHERE ehr_id = $1 AND upper_inf(sys_period) ORDER BY vo_id",
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
             WHERE vo_id = $1 AND ehr_id = $2 AND upper_inf(sys_period)",
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
            "SELECT sys_version, upper_inf(sys_period) AS is_current FROM vo_version \
             WHERE vo_id = $1 ORDER BY sys_version",
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
            let read = super::vobject::read_version(&self.pool, vo_id, *sv)
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
            "system_id": { "_type": "HIER_OBJECT_ID", "value": self.system_id },
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
        _an_ehr_id: Option<Uuid>,
        _an_extract: Extract,
    ) -> Result<(), SmError> {
        // PORT NOTE: import is the next SM-5 wave (docs/plans/b3-sm-services.md
        // task 2 — IMPORTED_VERSION storage, clone-EHR with a reused ehr_id).
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "import_ehr is not implemented in this stage (export-only)",
        ))
    }

    async fn import_ehr_extract(
        &self,
        _an_ehr_id: Uuid,
        _an_extract: Extract,
    ) -> Result<(), SmError> {
        // PORT NOTE: see import_ehr — import lands in the next wave.
        Err(SmError::new(
            CallStatusType::NotImplemented,
            "import_ehr_extract is not implemented in this stage (export-only)",
        ))
    }
}
