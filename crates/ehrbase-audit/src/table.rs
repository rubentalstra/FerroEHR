//! The data-driven audit table: generated operation id → audit classification.
//!
//! Keyed by the ITS-REST operation ids (`openehr-its::rest::generated`), per the
//! binding doc §2 scope. Every operation is **explicitly** classified — either
//! [`Classification::Audited`] with its `(EventActionCode, ObjectClass)` or
//! [`Classification::Unaudited`] (non-clinical / out-of-§2-scope surface). A new
//! generated operation that is neither returns `None` from [`classify`], which
//! the total-coverage guard test turns into a build failure until it is
//! classified — the same discipline as the codegen drift checks (§8.3).

use crate::event::{EventActionCode, ObjectClass};

/// The audit classification of an ITS-REST operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    /// The operation emits an audit record with this action + object class.
    Audited {
        /// The CRUD/execute action code.
        action: EventActionCode,
        /// The touched resource class.
        object: ObjectClass,
    },
    /// The operation is deliberately not audited (non-clinical or out of §2 scope).
    Unaudited,
}

impl Classification {
    /// A convenience constructor for an audited entry.
    const fn audited(action: EventActionCode, object: ObjectClass) -> Self {
        Classification::Audited { action, object }
    }
}

use EventActionCode::{Create, Delete, Execute, Read, Update};
use ObjectClass::{Composition, Contribution, Demographic, Directory, Ehr, Query, Template};

/// Classify an ITS-REST operation id. `None` means **unknown** — a generated
/// operation that has not been classified (fails the coverage guard).
#[must_use]
#[allow(clippy::match_same_arms)] // grouped by resource; explicitness is the point
pub fn classify(op: &str) -> Option<Classification> {
    let c = match op {
        // ── EHR ──────────────────────────────────────────────────────────────
        "ehr_get_by_subject" => Classification::audited(Read, Ehr),
        "ehr_create" => Classification::audited(Create, Ehr),
        "ehr_get_by_id" => Classification::audited(Read, Ehr),
        "ehr_create_with_id" => Classification::audited(Create, Ehr),

        // ── EHR_STATUS (a facet of the EHR / Patient Record) ─────────────────
        "ehr_status_get_by_version_id" => Classification::audited(Read, Ehr),
        "ehr_status_get_at_time" => Classification::audited(Read, Ehr),
        "ehr_status_update" => Classification::audited(Update, Ehr),
        "versioned_ehr_status_get" => Classification::audited(Read, Ehr),
        "versioned_ehr_status_revision_history" => Classification::audited(Read, Ehr),
        "versioned_ehr_status_version_get_at_time" => Classification::audited(Read, Ehr),
        "versioned_ehr_status_version_get_by_id" => Classification::audited(Read, Ehr),

        // ── COMPOSITION ──────────────────────────────────────────────────────
        "composition_create" => Classification::audited(Create, Composition),
        "composition_get" => Classification::audited(Read, Composition),
        "composition_update" => Classification::audited(Update, Composition),
        "composition_delete" => Classification::audited(Delete, Composition),
        "versioned_composition_get" => Classification::audited(Read, Composition),
        "versioned_composition_revision_history" => Classification::audited(Read, Composition),
        "versioned_composition_version_get_at_time" => Classification::audited(Read, Composition),
        "versioned_composition_version_get_by_id" => Classification::audited(Read, Composition),

        // ── DIRECTORY (FOLDER) ───────────────────────────────────────────────
        "directory_create" => Classification::audited(Create, Directory),
        "directory_update" => Classification::audited(Update, Directory),
        "directory_delete" => Classification::audited(Delete, Directory),
        "directory_get_at_time" => Classification::audited(Read, Directory),
        "directory_get_by_version_id" => Classification::audited(Read, Directory),

        // ── CONTRIBUTION (op ids shared by the ehr + demographic groups) ─────
        "contribution_create" => Classification::audited(Create, Contribution),
        "contribution_get" => Classification::audited(Read, Contribution),

        // ── Item tags (clinical-resource metadata access — audited on the
        //    parent resource; beyond §2's table but PHI-adjacent, so audited) ──
        "ehr_tags_get" => Classification::audited(Read, Ehr),
        "ehr_status_tags_get" => Classification::audited(Read, Ehr),
        "ehr_status_tags_update" => Classification::audited(Update, Ehr),
        "ehr_status_tags_delete" => Classification::audited(Delete, Ehr),
        "composition_tags_get" => Classification::audited(Read, Composition),
        "composition_tags_update" => Classification::audited(Update, Composition),
        "composition_tags_delete" => Classification::audited(Delete, Composition),

        // ── DEFINITION: stored queries (§2 "Definition / stored query") ──────
        "definition_query_list" => Classification::audited(Read, Query),
        "definition_query_version_get" => Classification::audited(Read, Query),
        // NOTE: the ".yaml" suffix is the codegen operationId verbatim.
        "definition_query_store.yaml" => Classification::audited(Create, Query),
        "definition_query_version_store.yaml" => Classification::audited(Create, Query),

        // ── QUERY execution (§2 "Query") ─────────────────────────────────────
        "query_execute_adhoc_query" => Classification::audited(Execute, Query),
        "query_execute_adhoc_query_body" => Classification::audited(Execute, Query),
        "query_execute_stored_query" => Classification::audited(Execute, Query),
        "query_execute_stored_query_body" => Classification::audited(Execute, Query),
        "query_execute_stored_query_version" => Classification::audited(Execute, Query),
        "query_execute_stored_query_version_body" => Classification::audited(Execute, Query),

        // ── ADMIN (§2 "Admin"; only the two generated admin routes exist) ────
        "admin_ehr_delete" => Classification::audited(Delete, Ehr),
        "admin_ehr_delete_all" => Classification::audited(Delete, Ehr),

        // ── DEFINITION: operational templates (OPT provisioning; beyond §2's
        //    table, but the owner mandates total coverage — audited as the
        //    Template class: upload → C, list/get/example/version → R) ────────
        "definition_template_adl1.4_upload" | "definition_template_adl2_upload" => {
            Classification::audited(Create, Template)
        }
        "definition_template_adl1.4_list"
        | "definition_template_adl1.4_get"
        | "definition_template_adl1.4_example_get"
        | "definition_template_adl2_list"
        | "definition_template_adl2_get"
        | "definition_template_adl2_example_get"
        | "definition_template_adl2_version_get" => Classification::audited(Read, Template),

        // ── DEMOGRAPHIC (beyond §2's table; person-identifiable data, so it is
        //    audited in full — currently unimplemented (501), which simply
        //    yields failure outcomes until the API lands) ─────────────────────
        "agent_create"
        | "group_create"
        | "organisation_create"
        | "person_create"
        | "role_create" => Classification::audited(Create, Demographic),
        "agent_get"
        | "group_get"
        | "organisation_get"
        | "person_get"
        | "role_get"
        | "versioned_party_get"
        | "versioned_party_revision_history"
        | "versioned_party_version_get_at_time"
        | "versioned_party_version_get_by_id"
        | "demographic_tags_get"
        | "agent_tags_get"
        | "group_tags_get"
        | "organisation_tags_get"
        | "person_tags_get"
        | "role_tags_get" => Classification::audited(Read, Demographic),
        "agent_update"
        | "group_update"
        | "organisation_update"
        | "person_update"
        | "role_update"
        | "agent_tags_update"
        | "group_tags_update"
        | "organisation_tags_update"
        | "person_tags_update"
        | "role_tags_update" => Classification::audited(Update, Demographic),
        "agent_delete"
        | "group_delete"
        | "organisation_delete"
        | "person_delete"
        | "role_delete"
        | "agent_tags_delete"
        | "group_tags_delete"
        | "organisation_tags_delete"
        | "person_tags_delete"
        | "role_tags_delete" => Classification::audited(Delete, Demographic),

        _ => return None,
    };
    Some(c)
}

/// The audited `(action, object)` for an operation, or `None` when the operation
/// is unaudited or unknown — the entry point the REST audit layer calls.
#[must_use]
pub fn audit_for(op: &str) -> Option<(EventActionCode, ObjectClass)> {
    match classify(op) {
        Some(Classification::Audited { action, object }) => Some((action, object)),
        _ => None,
    }
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

    /// §8.3: every generated operation id is classified (audited or unaudited).
    /// A new, unclassified generated operation fails this test.
    #[test]
    fn every_operation_is_classified() {
        let mut unclassified = Vec::new();
        for op in all_route_ops() {
            if classify(op).is_none() {
                unclassified.push(op);
            }
        }
        assert!(
            unclassified.is_empty(),
            "unclassified ITS-REST operations (add to crates/ehrbase-audit/src/table.rs): {unclassified:?}"
        );
    }

    /// Total coverage is **total**: every generated route entry is audited —
    /// the `UNAUDITED` allowlist is empty (status/health/swagger live outside
    /// the generated `ROUTES` tables and never reach the audit table).
    #[test]
    fn coverage_stats() {
        let ops = all_route_ops();
        let mut audited = 0;
        let mut unaudited = Vec::new();
        for op in &ops {
            match classify(op) {
                Some(Classification::Audited { .. }) => audited += 1,
                Some(Classification::Unaudited) => unaudited.push(*op),
                None => {}
            }
        }
        assert_eq!(
            audited,
            ops.len(),
            "every generated route entry must be audited; unaudited: {unaudited:?}"
        );
        assert!(unaudited.is_empty(), "unaudited ops: {unaudited:?}");
    }

    #[test]
    fn known_mappings() {
        assert_eq!(audit_for("ehr_create"), Some((Create, Ehr)));
        assert_eq!(audit_for("composition_delete"), Some((Delete, Composition)));
        assert_eq!(
            audit_for("query_execute_adhoc_query"),
            Some((Execute, Query))
        );
        assert_eq!(audit_for("admin_ehr_delete"), Some((Delete, Ehr)));
        // Templates + demographic are fully audited.
        assert_eq!(
            audit_for("definition_template_adl1.4_upload"),
            Some((Create, Template))
        );
        assert_eq!(
            audit_for("definition_template_adl1.4_get"),
            Some((Read, Template))
        );
        assert_eq!(audit_for("person_create"), Some((Create, Demographic)));
        assert_eq!(audit_for("person_update"), Some((Update, Demographic)));
        assert_eq!(audit_for("role_tags_delete"), Some((Delete, Demographic)));
        // Unknown ops yield no classification (fails the guard when generated).
        assert_eq!(audit_for("no_such_operation"), None);
        assert_eq!(classify("no_such_operation"), None);
    }
}
