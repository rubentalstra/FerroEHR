// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! `sea-query` identifier vocabulary for the live schema
//! (`migrations/ehr/0001_baseline.sql`).
//!
//! No openEHR spec governs the SQL schema — this is our own PG18-native
//! design.
//!
//! One enum per table, in the official `sea-query` derive shape: the `Table`
//! variant carries an explicit `#[iden = "..."]` and renders the table name;
//! every other variant renders its `snake_cased` column name. This is the
//! single typed name catalog — the AQL SQL generator consumes the `Table`
//! variants, and dynamic SQL elsewhere addresses columns through these enums
//! rather than string-duplicating names. Every rendered name is pinned to the
//! deployed DDL byte-for-byte (asserted by the tests below); the catalog is
//! kept complete against the schema, because a drifted catalog forces raw
//! column strings — exactly what this file exists to prevent.

/// `ehr` — one row per EHR.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum Ehr {
    /// The `ehr` table itself.
    #[iden = "ehr"]
    Table,
    /// `id` — the EHR id (`EHR.ehr_id`), a uuidv7.
    Id,
    /// `system_id` — the system that created this EHR, stored at creation and
    /// never mutated (not the live service configuration).
    SystemId,
    /// `time_created` — the server-computed EHR creation instant.
    TimeCreated,
    /// `subject_id` — denormalized copy of the current
    /// `EHR_STATUS.subject.external_ref.id.value`, backing the
    /// one-EHR-per-subject unique index.
    SubjectId,
    /// `subject_namespace` — denormalized copy of the current
    /// `EHR_STATUS.subject.external_ref.namespace`.
    SubjectNamespace,
    /// `is_queryable` — promoted copy of the current `EHR_STATUS.is_queryable`,
    /// backing the AQL full-population gate.
    IsQueryable,
}

/// `audit` — `AUDIT_DETAILS` of every committed change.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum Audit {
    /// The `audit` table itself.
    #[iden = "audit"]
    Table,
    /// `id` — the audit row id (a uuidv7).
    Id,
    /// `time_committed` — the server-computed commit instant, never
    /// client-supplied.
    TimeCommitted,
    /// `system_id` — `AUDIT_DETAILS.system_id`.
    SystemId,
    /// `change_type` — the `audit_change_type` terminology-group code.
    ChangeType,
    /// `description` — the canonical `DV_TEXT` fragment of
    /// `AUDIT_DETAILS.description` (0..1), as JSONB.
    Description,
    /// `committer` — the canonical `PARTY_PROXY` of the committer, as JSONB.
    Committer,
    /// `attestation` — the `ATTESTATION`-declared attributes as JSONB when the
    /// commit audit is an `ATTESTATION`, else NULL.
    Attestation,
}

/// `contribution` — the change-set envelope.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum Contribution {
    /// The `contribution` table itself.
    #[iden = "contribution"]
    Table,
    /// `id` — the CONTRIBUTION uid (a uuidv7).
    Id,
    /// `ehr_id` — the owning EHR, or `NULL` for a demographic (party)
    /// contribution, which no EHR owns.
    EhrId,
    /// `audit_id` — the contribution's `AUDIT_DETAILS` row.
    AuditId,
}

/// `template_store` — operational templates (OPT 1.4 XML); dual identity
/// (`id` = SM UUID handle, `template_id` = wire address).
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum TemplateStore {
    /// The `template_store` table itself.
    #[iden = "template_store"]
    Table,
    /// `id` — the SM OPT-by-UUID handle.
    Id,
    /// `template_id` — the wire address used by the DEFINITION API and by
    /// `vo_version.template_id`; also the source of the reported template
    /// version.
    TemplateId,
    /// `concept` — the template's concept name, as declared by the OPT.
    Concept,
    /// `root_archetype` — the archetype id at the template root.
    RootArchetype,
    /// `content` — the uploaded OPT XML, stored verbatim.
    Content,
    /// `created_at` — when the template was uploaded.
    CreatedAt,
}

/// `vo_version` — one row per version of a versioned object (temporal PK).
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum VoVersion {
    /// The `vo_version` table itself.
    #[iden = "vo_version"]
    Table,
    /// `vo_id` — the versioned object's id (the `object_id` of every
    /// `OBJECT_VERSION_ID` in its tree).
    VoId,
    /// `kind` — the versioned object's RM type (`COMPOSITION`, `EHR_STATUS`,
    /// `FOLDER`, a demographic party type, …).
    Kind,
    /// `ehr_id` — the owning EHR, or `NULL` for a demographic party.
    EhrId,
    /// `sys_version` — the opaque per-object commit ordinal (1..n across trunk
    /// AND branch commits); the join key of `node` / `vo_attestation`, and NOT
    /// the wire version number.
    SysVersion,
    /// `trunk_version` — `VERSION_TREE_ID` first part: the wire version number
    /// on a trunk row, the fork point on a branch row.
    TrunkVersion,
    /// `branch_number` — `VERSION_TREE_ID` second part; `0` = trunk row.
    BranchNumber,
    /// `branch_version` — `VERSION_TREE_ID` third part; `0` = trunk row.
    BranchVersion,
    /// `sys_period` — the validity interval `[committed, superseded)`; the
    /// current trunk version is the one with an unbounded upper end.
    SysPeriod,
    /// `lifecycle_state` — the `version_lifecycle_state` code (`523` is the
    /// content-less logical delete).
    LifecycleState,
    /// `creating_system_id` — the immutable middle segment of this version's
    /// `OBJECT_VERSION_ID`, reconstructed from storage and never live config.
    CreatingSystemId,
    /// `preceding_version_uid` — `ORIGINAL_VERSION.preceding_version_uid` as a
    /// full `OBJECT_VERSION_ID`; `NULL` for a first version.
    PrecedingVersionUid,
    /// `signature` — `VERSION.signature`, opaque radix-64.
    Signature,
    /// `other_input_version_uids` — the merge provenance of an
    /// `ORIGINAL_VERSION`; `NULL` when the version is not a merge.
    OtherInputVersionUids,
    /// `contribution_id` — the CONTRIBUTION this version was committed in.
    ContributionId,
    /// `audit_id` — this version's own `AUDIT_DETAILS` row.
    AuditId,
    /// `template_id` — the OPT the content was validated against; `NULL` for
    /// template-less content.
    TemplateId,
}

/// `node` — the decomposed content: one row per RM structure node, per
/// version (nested-set indexed).
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum Node {
    /// The `node` table itself.
    #[iden = "node"]
    Table,
    /// `vo_id` — the versioned object this node belongs to.
    VoId,
    /// `sys_version` — the `vo_version` commit ordinal this node belongs to.
    SysVersion,
    /// `num` — the node's pre-order number within the version (root = 0).
    Num,
    /// `num_cap` — the highest `num` in this node's subtree, so the subtree is
    /// the closed interval `num..=num_cap` (this is what AQL CONTAINS joins on).
    NumCap,
    /// `parent_num` — the `num` of the parent structure node (the root points
    /// at itself).
    ParentNum,
    /// `citem_num` — the `num` of the nearest ancestor carrying an archetype id.
    CitemNum,
    /// `ehr_id` — the owning EHR, denormalized onto every node for
    /// EHR-scoped querying.
    EhrId,
    /// `rm_type` — the node's full RM type name (never an alias).
    RmType,
    /// `archetype` — the node's `archetype_node_id` when it carries one.
    Archetype,
    /// `arch_entity` — the `qualified_rm_entity` of a full archetype HRID,
    /// lowercased; `NULL` on at/id-code nodes.
    ArchEntity,
    /// `arch_concept` — the full `domain_concept` of a full archetype HRID
    /// (specialisation segments included), lowercased, so a parent query
    /// matches a child by prefix.
    ArchConcept,
    /// `arch_major` — the major version of a full archetype HRID; `NULL` on
    /// at/id-code nodes.
    ArchMajor,
    /// `name` — the node's `name.value`.
    Name,
    /// `path` — the materialized path from the root, whose byte order under
    /// `COLLATE "C"` equals tree order; used for reassembly, never as an AQL
    /// predicate.
    Path,
    /// `data` — the node's canonical openEHR JSON fragment verbatim, with
    /// structure children pruned.
    Data,
}

/// `vo_attestation` — `ATTESTATION`s appended to an `ORIGINAL_VERSION`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum VoAttestation {
    /// The `vo_attestation` table itself.
    #[iden = "vo_attestation"]
    Table,
    /// `id` — the attestation row id (a uuidv7).
    Id,
    /// `vo_id` — the attested versioned object.
    VoId,
    /// `sys_version` — the attested version's commit ordinal.
    SysVersion,
    /// `contribution_id` — the CONTRIBUTION that appended the attestation.
    ContributionId,
    /// `time_committed` — when the attestation was appended.
    TimeCommitted,
    /// `data` — the canonical `ATTESTATION` JSON, verbatim.
    Data,
}

/// `stored_query` — semver-addressed stored AQL queries.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum StoredQuery {
    /// The `stored_query` table itself.
    #[iden = "stored_query"]
    Table,
    /// `reverse_domain_name` — the qualified name's namespace half.
    ReverseDomainName,
    /// `semantic_id` — the qualified name's local half.
    SemanticId,
    /// `semver` — the stored version of this query name.
    Semver,
    /// `query_type` — the query formalism (`AQL`).
    QueryType,
    /// `query_text` — the query source, stored verbatim.
    QueryText,
    /// `created_at` — when this version was stored.
    CreatedAt,
}

/// `item_tag` — the store behind the RELEASED ITS-REST 1.1.0 tags API
/// (SPECITS-77) and the two `openehr-item-tag` wrapper headers.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum ItemTag {
    /// The `item_tag` table itself.
    #[iden = "item_tag"]
    Table,
    /// `id` — the tag id (a uuidv7).
    Id,
    /// `ehr_id` — the owning EHR, or `NULL` when the target is a party.
    EhrId,
    /// `target_vo_id` — the tagged versioned object; deliberately FK-less,
    /// because a tag target may address a container or one version.
    TargetVoId,
    /// `target_type` — the RM type of the tagged object.
    TargetType,
    /// `key` — the tag key.
    Key,
    /// `value` — the tag value, optional.
    Value,
    /// `target_path` — the path within the target the tag applies to, if any.
    TargetPath,
    /// `created_at` — when the tag was written.
    CreatedAt,
}

/// `archetype_store` — ADL 1.4 source archetypes (`I_DEFINITION_ADL14`).
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum ArchetypeStore {
    /// The `archetype_store` table itself.
    #[iden = "archetype_store"]
    Table,
    /// `archetype_id` — the archetype id, which is also the primary key.
    ArchetypeId,
    /// `adl` — the ADL 1.4 source text, stored verbatim.
    Adl,
    /// `created_at` — when the archetype was uploaded.
    CreatedAt,
}

/// `adl2_artefact` — SM-2 ADL2 artefacts (`I_DEFINITION_ADL2`).
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum Adl2Artefact {
    /// The `adl2_artefact` table itself.
    #[iden = "adl2_artefact"]
    Table,
    /// `hrid` — the `ARCHETYPE_HRID`, which is also the primary key.
    Hrid,
    /// `kind` — which artefact this is: `archetype`, `template`, or
    /// `operational_template`.
    Kind,
    /// `adl` — the ADL2 source text, stored verbatim.
    Adl,
    /// `parent_hrid` — the declared `specialize` parent HRID (NULL when the
    /// artefact is not specialised); the archetype-lineage edge.
    ParentHrid,
    /// `created_at` — when the artefact was uploaded.
    CreatedAt,
}

/// `ehr_index` — SM-3 EHR Index (`I_EHR_INDEX`): N:M subject↔EHR associations.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum EhrIndex {
    /// The `ehr_index` table itself.
    #[iden = "ehr_index"]
    Table,
    /// `ehr_id` — the associated EHR.
    EhrId,
    /// `subject_id` — the associated subject's id.
    SubjectId,
    /// `subject_namespace` — the issuing namespace of `subject_id`.
    SubjectNamespace,
    /// `subject_type` — the subject's `OBJECT_REF.type` (`PERSON` by default).
    SubjectType,
    /// `instance_type` — `Primary` (authoritative), `Duplicate`, or
    /// `Supplementary`.
    InstanceType,
    /// `start_valid_time` — when the association became valid, if bounded.
    StartValidTime,
    /// `end_valid_time` — when the association stopped being valid, if bounded.
    EndValidTime,
    /// `notes` — free-text notes on the association.
    Notes,
    /// `location` — the `LOCATION_DESC` of the holding system, as canonical
    /// JSON.
    Location,
    /// `created_at` — when the association was recorded.
    CreatedAt,
}

/// `vo_archive` — SM-4 archive markers (`I_ADMIN_ARCHIVE`).
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum VoArchive {
    /// The `vo_archive` table itself.
    #[iden = "vo_archive"]
    Table,
    /// `vo_id` — the archived versioned object; a per-object marker, so
    /// deliberately FK-less against the per-version `vo_version`.
    VoId,
    /// `archived_at` — when the object was marked archived.
    ArchivedAt,
    /// `reason` — the caller-supplied archival reason, if any.
    Reason,
}

/// `sp_subject` — SM-6 Subject Proxy Service: one proxy per subject.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum SpSubject {
    /// The `sp_subject` table itself.
    #[iden = "sp_subject"]
    Table,
    /// `subject_id` — the proxied subject, which is also the primary key.
    SubjectId,
    /// `subject_category` — `SUBJECT_PROXY.subject_category`, an uncontrolled
    /// string.
    SubjectCategory,
    /// `create_time` — when the proxy was created.
    CreateTime,
}

/// `sp_binding` — SM-6 SPS: one `ENV_BINDING` per execution environment.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum SpBinding {
    /// The `sp_binding` table itself.
    #[iden = "sp_binding"]
    Table,
    /// `env_id` — the execution environment, which is also the primary key.
    EnvId,
    /// `description` — free-text description of the binding.
    Description,
}

/// `sp_data_frame` — SM-6 SPS: a `DATA_FRAME` within a binding.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum SpDataFrame {
    /// The `sp_data_frame` table itself.
    #[iden = "sp_data_frame"]
    Table,
    /// `env_id` — the binding this frame belongs to.
    EnvId,
    /// `frame_id` — the frame's id, unique service-wide because `get_frame`
    /// addresses a frame without an environment.
    FrameId,
    /// `model_type` — the information model the frame retrieves from.
    ModelType,
    /// `primary_method` — the canonical JSON of the frame's retrieval method.
    PrimaryMethod,
    /// `fallback_method` — the canonical JSON of the method tried when the
    /// primary one yields nothing.
    FallbackMethod,
}

/// `sp_variable` — SM-6 SPS: a `SUBJECT_VARIABLE` on a subject's proxy.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum SpVariable {
    /// The `sp_variable` table itself.
    #[iden = "sp_variable"]
    Table,
    /// `subject_id` — the proxy this variable hangs off.
    SubjectId,
    /// `canonical_name` — the variable's canonical name, its key within the
    /// proxy.
    CanonicalName,
    /// `namespace` — the namespace half of the variable's name, if any.
    Namespace,
    /// `name` — the local half of the variable's name.
    Name,
    /// `type_name` — the RM type of the retrieved value.
    TypeName,
    /// `currency` — the ISO 8601 duration a retrieved value stays valid for;
    /// unset means "the most recent available valid value".
    Currency,
    /// `ask_user` — whether the value may be asked of the user.
    AskUser,
    /// `is_manual` — whether the value is supplied manually rather than
    /// retrieved.
    IsManual,
    /// `frame_id` — the `DATA_FRAME` this variable is retrieved through.
    FrameId,
    /// `frame_path` — the path within the frame's result that holds the value.
    FramePath,
}

/// `sp_data_set` — SM-6 SPS: a `SUBJECT_DATA_SET` registered by an application.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum SpDataSet {
    /// The `sp_data_set` table itself.
    #[iden = "sp_data_set"]
    Table,
    /// `subject_id` — the proxy the data set belongs to.
    SubjectId,
    /// `id` — the data set's id within that proxy.
    Id,
    /// `creating_app_id` — the application that registered the data set.
    CreatingAppId,
    /// `using_app_ids` — the applications currently using it, as a JSON array.
    UsingAppIds,
    /// `variables` — the data-set-local name → `SUBJECT_VARIABLE` map, stored
    /// verbatim as canonical JSON because the local aliases differ from the
    /// canonical names.
    Variables,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_query::{Expr, ExprTrait as _, Iden as _, PostgresQueryBuilder, Query};

    #[test]
    fn table_names_render_exactly() {
        assert_eq!(Ehr::Table.to_string(), "ehr");
        assert_eq!(Audit::Table.to_string(), "audit");
        assert_eq!(Contribution::Table.to_string(), "contribution");
        assert_eq!(TemplateStore::Table.to_string(), "template_store");
        assert_eq!(VoVersion::Table.to_string(), "vo_version");
        assert_eq!(Node::Table.to_string(), "node");
        assert_eq!(VoAttestation::Table.to_string(), "vo_attestation");
        assert_eq!(StoredQuery::Table.to_string(), "stored_query");
        assert_eq!(ItemTag::Table.to_string(), "item_tag");
        assert_eq!(ArchetypeStore::Table.to_string(), "archetype_store");
        assert_eq!(Adl2Artefact::Table.to_string(), "adl2_artefact");
        assert_eq!(EhrIndex::Table.to_string(), "ehr_index");
        assert_eq!(VoArchive::Table.to_string(), "vo_archive");
        assert_eq!(SpSubject::Table.to_string(), "sp_subject");
        assert_eq!(SpBinding::Table.to_string(), "sp_binding");
        assert_eq!(SpDataFrame::Table.to_string(), "sp_data_frame");
        assert_eq!(SpVariable::Table.to_string(), "sp_variable");
        assert_eq!(SpDataSet::Table.to_string(), "sp_data_set");
    }

    #[test]
    fn column_names_render_exactly() {
        assert_eq!(Ehr::SystemId.to_string(), "system_id");
        assert_eq!(Node::VoId.to_string(), "vo_id");
        assert_eq!(Node::SysVersion.to_string(), "sys_version");
        assert_eq!(Node::NumCap.to_string(), "num_cap");
        assert_eq!(Node::CitemNum.to_string(), "citem_num");
        assert_eq!(Node::RmType.to_string(), "rm_type");
        assert_eq!(Node::ArchEntity.to_string(), "arch_entity");
        assert_eq!(Node::ArchConcept.to_string(), "arch_concept");
        assert_eq!(Node::ArchMajor.to_string(), "arch_major");
        assert_eq!(VoVersion::TrunkVersion.to_string(), "trunk_version");
        assert_eq!(VoVersion::BranchNumber.to_string(), "branch_number");
        assert_eq!(VoVersion::BranchVersion.to_string(), "branch_version");
        assert_eq!(
            VoVersion::PrecedingVersionUid.to_string(),
            "preceding_version_uid"
        );
        assert_eq!(VoVersion::SysPeriod.to_string(), "sys_period");
        assert_eq!(VoVersion::ContributionId.to_string(), "contribution_id");
        assert_eq!(
            VoVersion::CreatingSystemId.to_string(),
            "creating_system_id"
        );
        assert_eq!(
            VoVersion::OtherInputVersionUids.to_string(),
            "other_input_version_uids"
        );
        assert_eq!(Audit::TimeCommitted.to_string(), "time_committed");
        assert_eq!(
            StoredQuery::ReverseDomainName.to_string(),
            "reverse_domain_name"
        );
        assert_eq!(ItemTag::TargetVoId.to_string(), "target_vo_id");
        assert_eq!(SpDataFrame::PrimaryMethod.to_string(), "primary_method");
    }

    #[test]
    fn builds_a_node_query() {
        let (sql, _) = Query::select()
            .column((Node::Table, Node::Num))
            .from(Node::Table)
            .and_where(Expr::col(Node::RmType).eq("OBSERVATION"))
            .build(PostgresQueryBuilder);
        assert_eq!(
            sql,
            r#"SELECT "node"."num" FROM "node" WHERE "rm_type" = $1"#
        );
    }
}
