//! sea-query identifier enums for the `EHRbase` v2 `PostgreSQL` schema.
//!
//! Each enum models one table that EXISTS at the end state of the vendored
//! Flyway migrations (`crates/ehrbase/migrations/ehr/*.sql` +
//! `crates/ehrbase/migrations/ext/*.sql`), i.e. the schema as it stands **after
//! migration `2700_add_template_details.sql`**. The `Table` variant renders the
//! literal table name and every other variant renders a literal column name.
//!
//! This is the hand-written, jOOQ-replacement identifier layer (ADR-006): the
//! `sqlx` + `sea-query` persistence stack has no code generation, so these
//! `Iden` definitions stand in for jOOQ's generated table/column constants. They
//! are reused throughout persistence and, above all, by the AQL engine (P16),
//! which builds JSONB-path SQL dynamically against `comp_data` and friends.
//!
//! Naming: an enum's `Table` variant carries an explicit `#[iden = "..."]` so
//! the SQL table name is always literal and never depends on `PascalCase` →
//! `snake_case` inference; column variants rely on the derive's `snake_case`
//! conversion except where that would not reproduce the SQL name, which then
//! carry their own `#[iden = "..."]`.
//!
//! Schema shaping notes captured while deriving the final state:
//! - Multi-tenancy was removed by `0501`-`0504`: the `tenant` table and every
//!   `sys_tenant` column are gone, so no enum models them.
//! - `1100_drop_system.sql` dropped the `system` table and
//!   `audit_details.system_id`.
//! - `2500_merge_version_and_data_history_tables.sql` dropped the per-locatable
//!   `*_data_history` tables entirely and folded their content into the
//!   `*_version_history` tables via the `ov_ref` / `ov_data` (and folder-only
//!   `ov_item_uuids`) columns — hence there is no `CompDataHistory` etc.

/// `ehr` — the EHR aggregate root. Created by `0100`; lost `sys_tenant` at `0503`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum Ehr {
    #[iden = "ehr"]
    Table,
    Id,
    CreationDate,
}

/// `comp_data` — row-per-locatable current composition data. Was `comp` (`0100`),
/// renamed + slimmed at `0604`, gained `parent_num`/`num_cap` and lost the
/// `entity_path*`/`entity_idx_cap` columns at `1500`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum CompData {
    #[iden = "comp_data"]
    Table,
    VoId,
    Num,
    CitemNum,
    RmEntity,
    EntityConcept,
    EntityName,
    EntityAttribute,
    EntityIdx,
    EntityIdxLen,
    Data,
    ParentNum,
    NumCap,
}

/// `comp_version` — current composition version heads. Created by `0601`; gained
/// the `root_concept` column at `0901`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum CompVersion {
    #[iden = "comp_version"]
    Table,
    VoId,
    EhrId,
    ContributionId,
    AuditId,
    TemplateId,
    SysVersion,
    SysPeriodLower,
    RootConcept,
}

/// `comp_version_history` — composition version history. Created by `0601`; at
/// `2500` lost `root_concept` and absorbed the old `comp_data_history` via
/// `ov_ref`/`ov_data`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum CompVersionHistory {
    #[iden = "comp_version_history"]
    Table,
    VoId,
    EhrId,
    ContributionId,
    AuditId,
    TemplateId,
    SysVersion,
    SysPeriodLower,
    SysPeriodUpper,
    SysDeleted,
    OvRef,
    OvData,
}

/// `ehr_status_data` — current `EHR_STATUS` locatable data. Was `ehr_status`
/// (`0300`), renamed + slimmed at `0604`, gained `parent_num`/`num_cap` and lost
/// the `entity_path*`/`entity_idx_cap` columns at `1500`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum EhrStatusData {
    #[iden = "ehr_status_data"]
    Table,
    VoId,
    Num,
    EhrId,
    CitemNum,
    RmEntity,
    EntityConcept,
    EntityName,
    EntityAttribute,
    EntityIdx,
    EntityIdxLen,
    Data,
    ParentNum,
    NumCap,
}

/// `ehr_status_version` — current `EHR_STATUS` version heads. Created by `0601`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum EhrStatusVersion {
    #[iden = "ehr_status_version"]
    Table,
    VoId,
    EhrId,
    ContributionId,
    AuditId,
    SysVersion,
    SysPeriodLower,
}

/// `ehr_status_version_history` — `EHR_STATUS` version history. Created by `0601`;
/// at `2500` absorbed the old `ehr_status_data_history` via `ov_ref`/`ov_data`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum EhrStatusVersionHistory {
    #[iden = "ehr_status_version_history"]
    Table,
    VoId,
    EhrId,
    ContributionId,
    AuditId,
    SysVersion,
    SysPeriodLower,
    SysPeriodUpper,
    SysDeleted,
    OvRef,
    OvData,
}

/// `ehr_folder_data` — current FOLDER locatable data. Was `ehr_folder` (`0300`),
/// renamed + slimmed at `0604`, gained `parent_num`/`num_cap` and lost the
/// `entity_path*`/`entity_idx_cap` columns at `1500`, gained `item_uuids` at
/// `1700`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum EhrFolderData {
    #[iden = "ehr_folder_data"]
    Table,
    VoId,
    Num,
    EhrId,
    EhrFoldersIdx,
    CitemNum,
    RmEntity,
    EntityConcept,
    EntityName,
    EntityAttribute,
    EntityIdx,
    EntityIdxLen,
    Data,
    ParentNum,
    NumCap,
    ItemUuids,
}

/// `ehr_folder_version` — current FOLDER version heads. Created by `0601`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum EhrFolderVersion {
    #[iden = "ehr_folder_version"]
    Table,
    VoId,
    EhrId,
    ContributionId,
    AuditId,
    SysVersion,
    SysPeriodLower,
    EhrFoldersIdx,
}

/// `ehr_folder_version_history` — FOLDER version history. Created by `0601`; at
/// `2500` absorbed the old `ehr_folder_data_history` via
/// `ov_item_uuids`/`ov_ref`/`ov_data`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum EhrFolderVersionHistory {
    #[iden = "ehr_folder_version_history"]
    Table,
    VoId,
    EhrId,
    ContributionId,
    AuditId,
    SysVersion,
    SysPeriodLower,
    EhrFoldersIdx,
    SysPeriodUpper,
    SysDeleted,
    OvItemUuids,
    OvRef,
    OvData,
}

/// `contribution` — the versioning/audit envelope. Created by `0100`; lost
/// `sys_tenant` at `0503` and `state` at `1300`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum Contribution {
    #[iden = "contribution"]
    Table,
    Id,
    EhrId,
    ContributionType,
    Signature,
    HasAudit,
}

/// `audit_details` — per-version audit record. Created by `0100`; lost
/// `sys_tenant` at `0503`, lost `system_id` at `1100`, gained `target_type` at
/// `1300`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum AuditDetails {
    #[iden = "audit_details"]
    Table,
    Id,
    ChangeType,
    Description,
    TimeCommitted,
    Committer,
    UserId,
    TargetType,
}

/// `template_store` — OPT/template registry. Created by `0100`; lost
/// `sys_tenant` at `0503`, gained `concept`/`root_archetype` at `2700`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum TemplateStore {
    #[iden = "template_store"]
    Table,
    Id,
    TemplateId,
    Content,
    CreationTime,
    Concept,
    RootArchetype,
}

/// `stored_query` — persisted AQL queries. Created by `0100`; lost `sys_tenant`
/// at `0503`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum StoredQuery {
    #[iden = "stored_query"]
    Table,
    ReverseDomainName,
    SemanticId,
    Semver,
    QueryText,
    Type,
    CreationDate,
}

/// `ehr_item_tag` — item-tag store for all `VERSIONED_OBJECT` types. Created by
/// `1600`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum EhrItemTag {
    #[iden = "ehr_item_tag"]
    Table,
    Id,
    EhrId,
    TargetVoId,
    TargetType,
    Key,
    Value,
    TargetPath,
    CreationDate,
    SysPeriodLower,
}

/// `users` — committer/user records. Created by `0100`; lost `sys_tenant` at
/// `0503`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum Users {
    #[iden = "users"]
    Table,
    Id,
    Username,
}

/// `plugin` — key/value store for the plugin subsystem. Created by `0100`.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum Plugin {
    #[iden = "plugin"]
    Table,
    Id,
    #[iden = "pluginid"]
    Pluginid,
    Key,
    Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_query::{Expr, ExprTrait as _, Iden as _, PostgresQueryBuilder, Query};

    /// Every `Table` iden renders to its exact SQL table name.
    #[test]
    fn table_names_render_exactly() {
        assert_eq!(Ehr::Table.to_string(), "ehr");
        assert_eq!(CompData::Table.to_string(), "comp_data");
        assert_eq!(CompVersion::Table.to_string(), "comp_version");
        assert_eq!(
            CompVersionHistory::Table.to_string(),
            "comp_version_history"
        );
        assert_eq!(EhrStatusData::Table.to_string(), "ehr_status_data");
        assert_eq!(EhrStatusVersion::Table.to_string(), "ehr_status_version");
        assert_eq!(
            EhrStatusVersionHistory::Table.to_string(),
            "ehr_status_version_history"
        );
        assert_eq!(EhrFolderData::Table.to_string(), "ehr_folder_data");
        assert_eq!(EhrFolderVersion::Table.to_string(), "ehr_folder_version");
        assert_eq!(
            EhrFolderVersionHistory::Table.to_string(),
            "ehr_folder_version_history"
        );
        assert_eq!(Contribution::Table.to_string(), "contribution");
        assert_eq!(AuditDetails::Table.to_string(), "audit_details");
        assert_eq!(TemplateStore::Table.to_string(), "template_store");
        assert_eq!(StoredQuery::Table.to_string(), "stored_query");
        assert_eq!(EhrItemTag::Table.to_string(), "ehr_item_tag");
        assert_eq!(Users::Table.to_string(), "users");
        assert_eq!(Plugin::Table.to_string(), "plugin");
    }

    /// Column idens of the representative tables render to the exact DDL names.
    #[test]
    fn column_names_render_exactly() {
        // comp_data (final state after 0604 + 1500)
        assert_eq!(CompData::VoId.to_string(), "vo_id");
        assert_eq!(CompData::Num.to_string(), "num");
        assert_eq!(CompData::CitemNum.to_string(), "citem_num");
        assert_eq!(CompData::RmEntity.to_string(), "rm_entity");
        assert_eq!(CompData::EntityConcept.to_string(), "entity_concept");
        assert_eq!(CompData::EntityName.to_string(), "entity_name");
        assert_eq!(CompData::EntityAttribute.to_string(), "entity_attribute");
        assert_eq!(CompData::EntityIdx.to_string(), "entity_idx");
        assert_eq!(CompData::EntityIdxLen.to_string(), "entity_idx_len");
        assert_eq!(CompData::Data.to_string(), "data");
        assert_eq!(CompData::ParentNum.to_string(), "parent_num");
        assert_eq!(CompData::NumCap.to_string(), "num_cap");

        // comp_version (final state after 0901)
        assert_eq!(CompVersion::VoId.to_string(), "vo_id");
        assert_eq!(CompVersion::EhrId.to_string(), "ehr_id");
        assert_eq!(CompVersion::ContributionId.to_string(), "contribution_id");
        assert_eq!(CompVersion::AuditId.to_string(), "audit_id");
        assert_eq!(CompVersion::TemplateId.to_string(), "template_id");
        assert_eq!(CompVersion::SysVersion.to_string(), "sys_version");
        assert_eq!(CompVersion::SysPeriodLower.to_string(), "sys_period_lower");
        assert_eq!(CompVersion::RootConcept.to_string(), "root_concept");

        // audit_details (final state after 1100 + 1300)
        assert_eq!(AuditDetails::Id.to_string(), "id");
        assert_eq!(AuditDetails::ChangeType.to_string(), "change_type");
        assert_eq!(AuditDetails::Description.to_string(), "description");
        assert_eq!(AuditDetails::TimeCommitted.to_string(), "time_committed");
        assert_eq!(AuditDetails::Committer.to_string(), "committer");
        assert_eq!(AuditDetails::UserId.to_string(), "user_id");
        assert_eq!(AuditDetails::TargetType.to_string(), "target_type");

        // contribution (final state after 1300)
        assert_eq!(Contribution::Id.to_string(), "id");
        assert_eq!(Contribution::EhrId.to_string(), "ehr_id");
        assert_eq!(
            Contribution::ContributionType.to_string(),
            "contribution_type"
        );
        assert_eq!(Contribution::Signature.to_string(), "signature");
        assert_eq!(Contribution::HasAudit.to_string(), "has_audit");

        // the two idens that need an explicit override
        assert_eq!(Plugin::Pluginid.to_string(), "pluginid");
        assert_eq!(StoredQuery::Type.to_string(), "type");
    }

    /// Smoke test: build a real SELECT against `comp_data` with the Postgres
    /// backend and assert the rendered SQL (identifiers quoted, value bound).
    #[test]
    fn builds_select_from_comp_data() {
        let (sql, values) = Query::select()
            .columns([CompData::VoId, CompData::Num, CompData::Data])
            .from(CompData::Table)
            .and_where(Expr::col(CompData::VoId).eq(uuid::Uuid::nil()))
            .build(PostgresQueryBuilder);

        assert_eq!(
            sql,
            r#"SELECT "vo_id", "num", "data" FROM "comp_data" WHERE "vo_id" = $1"#
        );
        assert_eq!(values.0.len(), 1);
    }
}
