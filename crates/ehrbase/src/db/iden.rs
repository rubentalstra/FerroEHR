//! `sea-query` identifier definitions for the greenfield schema (ADR-008,
//! `migrations/ehr/0001_schema.sql`). One enum per table: the `Table`
//! variant renders the table name, the rest the column names. Reused by the
//! AQL SQL generator (P16).

/// `ehr` — one row per EHR.
#[derive(Debug, Clone, Copy, sea_query::Iden)]
pub enum Ehr {
    #[iden = "ehr"]
    Table,
    Id,
    TimeCreated,
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

/// `template_store` — operational templates (OPT 1.4 XML).
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
    SysPeriod,
    Deleted,
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
    Name,
    Path,
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
        assert_eq!(StoredQuery::Table.to_string(), "stored_query");
        assert_eq!(ItemTag::Table.to_string(), "item_tag");
    }

    #[test]
    fn column_names_render_exactly() {
        assert_eq!(Node::VoId.to_string(), "vo_id");
        assert_eq!(Node::SysVersion.to_string(), "sys_version");
        assert_eq!(Node::NumCap.to_string(), "num_cap");
        assert_eq!(Node::CitemNum.to_string(), "citem_num");
        assert_eq!(Node::RmType.to_string(), "rm_type");
        assert_eq!(VoVersion::SysPeriod.to_string(), "sys_period");
        assert_eq!(VoVersion::ContributionId.to_string(), "contribution_id");
        assert_eq!(Audit::TimeCommitted.to_string(), "time_committed");
        assert_eq!(
            StoredQuery::ReverseDomainName.to_string(),
            "reverse_domain_name"
        );
        assert_eq!(ItemTag::TargetVoId.to_string(), "target_vo_id");
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
