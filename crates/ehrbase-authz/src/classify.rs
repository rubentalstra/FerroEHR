//! Operation classification: a generated ITS-REST operation id → its coarse
//! [`OperationClass`] (§5.2 of `docs/enterprise/access-control.md`).
//!
//! Keyed by the ITS-REST operation ids (`openehr-its::rest::generated`). Every
//! generated operation is **explicitly** classified; a new generated operation
//! that is not returns `None` from [`class_of`], which the total-coverage guard
//! test turns into a build failure until it is classified — the same discipline
//! as `ehrbase-audit/src/table.rs` (§8.3 of the ATNA doc).
//!
//! Among the generated routes only the two `admin_*` operations are
//! [`OperationClass::Admin`]; everything else is [`OperationClass::Clinical`]
//! (any authenticated principal with a role), matching v1's rule that only
//! `/rest/admin/**` and the management endpoints require `ADMIN` while
//! "everything else → any authenticated user" (§2.4). [`OperationClass::Public`]
//! and [`OperationClass::Management`] are used by the REST layer to classify the
//! *non-generated* surface (status/health/swagger/management), which never
//! reaches [`class_of`].

/// The coarse authorization class of an operation (§5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationClass {
    /// No authorization check (root/status/health per the router).
    Public,
    /// Any authenticated principal with at least one role.
    Clinical,
    /// The management surface — gated by the `rbac.management_access` tri-state.
    Management,
    /// Administrative operations — require `rbac.admin_role`.
    Admin,
}

/// Classify a generated ITS-REST operation id. `None` means **unknown** — a
/// generated operation that has not been classified (fails the coverage guard).
///
/// Only the generated route surface is covered here; the non-generated surface
/// (status/health/swagger/management) is classified by route in the REST layer.
#[must_use]
#[allow(clippy::match_same_arms)] // grouped by resource family; explicitness is the point
pub fn class_of(op: &str) -> Option<OperationClass> {
    use OperationClass::{Admin, Clinical};
    let class = match op {
        // ── ADMIN API (the only Admin-class generated routes) ────────────────
        "admin_ehr_delete" | "admin_ehr_delete_all" => Admin,

        // ── EHR ──────────────────────────────────────────────────────────────
        "ehr_get_by_subject" | "ehr_create" | "ehr_get_by_id" | "ehr_create_with_id" => Clinical,

        // ── EHR_STATUS ────────────────────────────────────────────────────────
        "ehr_status_get_by_version_id"
        | "ehr_status_get_at_time"
        | "ehr_status_update"
        | "versioned_ehr_status_get"
        | "versioned_ehr_status_revision_history"
        | "versioned_ehr_status_version_get_at_time"
        | "versioned_ehr_status_version_get_by_id" => Clinical,

        // ── COMPOSITION ──────────────────────────────────────────────────────
        "composition_create"
        | "composition_get"
        | "composition_update"
        | "composition_delete"
        | "versioned_composition_get"
        | "versioned_composition_revision_history"
        | "versioned_composition_version_get_at_time"
        | "versioned_composition_version_get_by_id" => Clinical,

        // ── DIRECTORY (FOLDER) ───────────────────────────────────────────────
        "directory_get_at_time"
        | "directory_update"
        | "directory_create"
        | "directory_delete"
        | "directory_get_by_version_id" => Clinical,

        // ── CONTRIBUTION (op ids shared by the ehr + demographic groups) ─────
        "contribution_create" | "contribution_get" => Clinical,

        // ── Item tags (clinical-resource metadata) ───────────────────────────
        "ehr_tags_get"
        | "composition_tags_get"
        | "composition_tags_update"
        | "composition_tags_delete"
        | "ehr_status_tags_get"
        | "ehr_status_tags_update"
        | "ehr_status_tags_delete" => Clinical,

        // ── QUERY execution ──────────────────────────────────────────────────
        "query_execute_adhoc_query"
        | "query_execute_adhoc_query_body"
        | "query_execute_stored_query"
        | "query_execute_stored_query_body"
        | "query_execute_stored_query_version"
        | "query_execute_stored_query_version_body" => Clinical,

        // ── DEFINITION: templates + stored queries (v1: any authenticated) ───
        "definition_template_adl1.4_list"
        | "definition_template_adl1.4_upload"
        | "definition_template_adl1.4_get"
        | "definition_template_adl1.4_example_get"
        | "definition_template_adl2_list"
        | "definition_template_adl2_upload"
        | "definition_template_adl2_get"
        | "definition_template_adl2_example_get"
        | "definition_template_adl2_version_get"
        | "definition_query_list"
        | "definition_query_store.yaml"
        | "definition_query_version_get"
        | "definition_query_version_store.yaml" => Clinical,

        // ── DEMOGRAPHIC (v1 ABAC never covered these; coarse RBAC only) ──────
        "agent_create" | "agent_get" | "agent_update" | "agent_delete" => Clinical,
        "group_create" | "group_get" | "group_update" | "group_delete" => Clinical,
        "organisation_create"
        | "organisation_get"
        | "organisation_update"
        | "organisation_delete" => Clinical,
        "person_create" | "person_get" | "person_update" | "person_delete" => Clinical,
        "role_create" | "role_get" | "role_update" | "role_delete" => Clinical,
        "versioned_party_get"
        | "versioned_party_revision_history"
        | "versioned_party_version_get_at_time"
        | "versioned_party_version_get_by_id" => Clinical,
        "demographic_tags_get"
        | "agent_tags_get"
        | "agent_tags_update"
        | "agent_tags_delete"
        | "group_tags_get"
        | "group_tags_update"
        | "group_tags_delete"
        | "organisation_tags_get"
        | "organisation_tags_update"
        | "organisation_tags_delete"
        | "person_tags_get"
        | "person_tags_update"
        | "person_tags_delete"
        | "role_tags_get"
        | "role_tags_update"
        | "role_tags_delete" => Clinical,

        _ => return None,
    };
    Some(class)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The generated ITS-REST route tables, all groups.
    fn all_route_ops() -> Vec<&'static str> {
        use openehr_its::rest::generated as g;
        let mut ops = Vec::new();
        for table in [
            g::ehr::ROUTES,
            g::definition::ROUTES,
            g::demographic::ROUTES,
            g::query::ROUTES,
            g::admin::ROUTES,
        ] {
            for (_method, _path, op) in table {
                ops.push(*op);
            }
        }
        ops
    }

    /// §9.1(a): every generated operation id has an [`OperationClass`]. A new,
    /// unclassified generated operation fails this test.
    #[test]
    fn every_operation_is_classified() {
        let mut unclassified = Vec::new();
        for op in all_route_ops() {
            if class_of(op).is_none() {
                unclassified.push(op);
            }
        }
        assert!(
            unclassified.is_empty(),
            "unclassified ITS-REST operations (add to crates/ehrbase-authz/src/classify.rs): {unclassified:?}"
        );
    }

    /// The only Admin-class generated routes are the two admin-API deletes;
    /// everything else generated is Clinical (§2.4 / §5.2).
    #[test]
    fn admin_routes_are_the_only_admin_class() {
        let mut admin: Vec<&str> = all_route_ops()
            .into_iter()
            .filter(|op| class_of(op) == Some(OperationClass::Admin))
            .collect();
        admin.sort_unstable();
        admin.dedup();
        assert_eq!(admin, vec!["admin_ehr_delete", "admin_ehr_delete_all"]);
    }

    #[test]
    fn known_mappings() {
        assert_eq!(class_of("ehr_create"), Some(OperationClass::Clinical));
        assert_eq!(class_of("admin_ehr_delete"), Some(OperationClass::Admin));
        assert_eq!(
            class_of("definition_template_adl1.4_upload"),
            Some(OperationClass::Clinical)
        );
        assert_eq!(class_of("person_get"), Some(OperationClass::Clinical));
        assert_eq!(class_of("no_such_operation"), None);
    }
}
