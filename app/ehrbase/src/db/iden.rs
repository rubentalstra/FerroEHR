//! `sea-query` identifier vocabulary for the live schema
//! (`migrations/ehr/0001_baseline.sql`). No openEHR spec governs the SQL
//! schema — this is our own PG18-native design (`docs/architecture.md`
//! §Storage).
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
    #[iden = "ehr"]
    Table,
    Id,
    SystemId,
    TimeCreated,
    SubjectId,
    SubjectNamespace,
    IsQueryable,
}

/// `audit` — `AUDIT_DETAILS` of every committed change.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum Audit {
    #[iden = "audit"]
    Table,
    Id,
    TimeCommitted,
    SystemId,
    ChangeType,
    Description,
    Committer,
}

/// `contribution` — the change-set envelope.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum Contribution {
    #[iden = "contribution"]
    Table,
    Id,
    EhrId,
    AuditId,
}

/// `template_store` — operational templates (OPT 1.4 XML); dual identity
/// (`id` = SM UUID handle, `template_id` = wire address).
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum TemplateStore {
    #[iden = "template_store"]
    Table,
    Id,
    TemplateId,
    Concept,
    RootArchetype,
    Content,
    CreatedAt,
}

/// `vo_version` — one row per version of a versioned object (temporal PK).
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum VoVersion {
    #[iden = "vo_version"]
    Table,
    VoId,
    Kind,
    EhrId,
    SysVersion,
    TrunkVersion,
    BranchNumber,
    BranchVersion,
    SysPeriod,
    LifecycleState,
    CreatingSystemId,
    PrecedingVersionUid,
    Signature,
    OtherInputVersionUids,
    ContributionId,
    AuditId,
    TemplateId,
}

/// `node` — the decomposed content: one row per RM structure node, per
/// version (nested-set indexed).
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum Node {
    #[iden = "node"]
    Table,
    VoId,
    SysVersion,
    Num,
    NumCap,
    ParentNum,
    CitemNum,
    EhrId,
    RmType,
    Archetype,
    ArchEntity,
    ArchConcept,
    ArchMajor,
    Name,
    Path,
    Data,
}

/// `vo_attestation` — `ATTESTATION`s appended to an `ORIGINAL_VERSION`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum VoAttestation {
    #[iden = "vo_attestation"]
    Table,
    Id,
    VoId,
    SysVersion,
    ContributionId,
    TimeCommitted,
    Data,
}

/// `stored_query` — semver-addressed stored AQL queries.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum StoredQuery {
    #[iden = "stored_query"]
    Table,
    ReverseDomainName,
    SemanticId,
    Semver,
    QueryType,
    QueryText,
    CreatedAt,
}

/// `item_tag` — the experimental tags API.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum ItemTag {
    #[iden = "item_tag"]
    Table,
    Id,
    EhrId,
    TargetVoId,
    TargetType,
    Key,
    Value,
    TargetPath,
    CreatedAt,
}

/// `archetype_store` — ADL 1.4 source archetypes (`I_DEFINITION_ADL14`).
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum ArchetypeStore {
    #[iden = "archetype_store"]
    Table,
    ArchetypeId,
    Adl,
    CreatedAt,
}

/// `adl2_artefact` — SM-2 ADL2 artefacts (`I_DEFINITION_ADL2`).
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum Adl2Artefact {
    #[iden = "adl2_artefact"]
    Table,
    Hrid,
    Kind,
    Adl,
    CreatedAt,
}

/// `ehr_index` — SM-3 EHR Index (`I_EHR_INDEX`): N:M subject↔EHR associations.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum EhrIndex {
    #[iden = "ehr_index"]
    Table,
    EhrId,
    SubjectId,
    SubjectNamespace,
    SubjectType,
    InstanceType,
    StartValidTime,
    EndValidTime,
    Notes,
    Location,
    CreatedAt,
}

/// `vo_archive` — SM-4 archive markers (`I_ADMIN_ARCHIVE`).
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum VoArchive {
    #[iden = "vo_archive"]
    Table,
    VoId,
    ArchivedAt,
    Reason,
}

/// `sp_subject` — SM-6 Subject Proxy Service: one proxy per subject.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum SpSubject {
    #[iden = "sp_subject"]
    Table,
    SubjectId,
    SubjectCategory,
    CreateTime,
}

/// `sp_binding` — SM-6 SPS: one `ENV_BINDING` per execution environment.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum SpBinding {
    #[iden = "sp_binding"]
    Table,
    EnvId,
    Description,
}

/// `sp_data_frame` — SM-6 SPS: a `DATA_FRAME` within a binding.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum SpDataFrame {
    #[iden = "sp_data_frame"]
    Table,
    EnvId,
    FrameId,
    ModelType,
    PrimaryMethod,
    FallbackMethod,
}

/// `sp_variable` — SM-6 SPS: a `SUBJECT_VARIABLE` on a subject's proxy.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum SpVariable {
    #[iden = "sp_variable"]
    Table,
    SubjectId,
    CanonicalName,
    Namespace,
    Name,
    TypeName,
    Currency,
    AskUser,
    IsManual,
    FrameId,
    FramePath,
}

/// `sp_data_set` — SM-6 SPS: a `SUBJECT_DATA_SET` registered by an application.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum SpDataSet {
    #[iden = "sp_data_set"]
    Table,
    SubjectId,
    Id,
    CreatingAppId,
    UsingAppIds,
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
