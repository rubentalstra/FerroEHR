#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! Guard: the ECC skip-elimination reclassification (issue #146, Families A+B;
//! owner ruling — an ECC case passes, fails, errors, or is N/A, NEVER
//! "skipped").
//!
//! - **A1 (11 cases)** — SM operations our *flagged extensions* realize over
//!   the wire (`GET /ehr/{ehr_id}/contribution`, `DELETE
//!   /admin/template/{template_id}`, `GET /definition/query`): reclassified
//!   from skip-stubs to live [`Binding::Rest`] executions, each carrying the
//!   explicit "no ITS-REST binding — ehrbase-rs extension" flag in its
//!   citation.
//! - **A2/B (18 cases)** — native-API-only SM operations with no ITS-REST
//!   binding anywhere: reclassified to first-class N/A. Their run functions
//!   return `CaseError::NotApplicable` (proven end-to-end in `engine::run`'s
//!   unit tests); here we pin the registry-level shape (they keep their
//!   `NativeApiOnly` / `NoRestBinding` markers + native-test evidence).
//!
//! These lists are exhaustive for the two families the reclassification
//! touched, so the counts are pinned: any future case that regresses one of
//! these back to a skip-stub — or that silently changes a family's binding
//! class — fails here.
#![allow(clippy::expect_used)]

use conformance::case::{Binding, CaseMeta};
use conformance::registry::registry;

/// The 11 A1 cases now execute against a flagged extension → [`Binding::Rest`].
const A1_EXECUTED: &[&str] = &[
    // CTB — list_contributions via GET /ehr/{ehr_id}/contribution.
    "ctb/list-contributions-empty",
    "ctb/list-contributions-non-existing-ehr",
    "ctb/list-contributions-post-commit",
    "ctb/list-contributions-ehr-containing-directory",
    "ctb/list-contributions-ehr-containing-ehr-status",
    // TPL — delete_opt via DELETE /admin/template/{template_id}.
    "tpl/delete-opt-delete-existing",
    "tpl/delete-opt-delete-latest-version",
    "tpl/delete-opt-delete-specific-version",
    "tpl/delete-opt-delete-non-existing",
    // SQR — list_queries bare collection via GET /definition/query.
    "sqr/list-queries-empty",
    "sqr/list-queries-select-items",
];

/// The 18 A2/B cases have no ITS-REST wire anywhere → first-class N/A, keeping
/// their `NativeApiOnly` / `NoRestBinding` markers.
const A2B_NOT_APPLICABLE: &[&str] = &[
    // ADM — native-API-only + demographic-dependent SM operations.
    "adm/list-contributions",
    "adm/contribution-count",
    "adm/versioned-composition-count",
    "adm/composition-version-count",
    "adm/export-ehrs",
    "adm/archive-ehrs",
    "adm/physical-party-delete",
    "adm/archive-parties",
    // MSG — EHR Extract + TDD, native-API-only.
    "msg/export-ehrs",
    "msg/export-ehr-extracts",
    "msg/export-unknown-ehr",
    "msg/import-ehr-clone",
    "msg/import-ehr-fixed-id",
    "msg/import-ehr-duplicate",
    "msg/import-ehr-extract",
    "msg/tdd-import-commits",
    "msg/tdd-import-rejects",
    "msg/tdd-import-tdds-batch",
];

/// The registered metadata for a case slug (panics if absent — a renamed slug
/// must be reflected here in the same change).
fn meta_of(slug: &str) -> &'static CaseMeta {
    &registry()
        .entries()
        .iter()
        .find(|e| e.meta.id == slug)
        .unwrap_or_else(|| panic!("no registered ECC case with id {slug}"))
        .meta
}

#[test]
fn a1_cases_execute_against_a_flagged_extension() {
    for slug in A1_EXECUTED {
        let meta = meta_of(slug);
        assert!(
            matches!(meta.binding, Binding::Rest(_)),
            "{slug}: an A1 extension-backed case must be Binding::Rest, got {:?}",
            meta.binding
        );
        assert!(
            meta.citation.contains("ehrbase-rs extension"),
            "{slug}: an A1 case must carry the explicit \"ehrbase-rs extension\" flag in its \
             citation — got {:?}",
            meta.citation
        );
    }
}

#[test]
fn a2b_cases_are_first_class_not_applicable() {
    for slug in A2B_NOT_APPLICABLE {
        let meta = meta_of(slug);
        assert!(
            matches!(
                meta.binding,
                Binding::NativeApiOnly(_) | Binding::NoRestBinding(_)
            ),
            "{slug}: an A2/B not-applicable case must keep its native-API-only / no-binding \
             marker, got {:?}",
            meta.binding
        );
    }
}

/// The two families are disjoint and the reclassification touched exactly these
/// 29 cases — a count guard so a future edit cannot silently grow/shrink the
/// set without updating the lists (and the catalogue-count expectations).
#[test]
fn the_reclassified_families_are_exhaustive_and_disjoint() {
    assert_eq!(A1_EXECUTED.len(), 11, "A1 executed set is 11 cases");
    assert_eq!(A2B_NOT_APPLICABLE.len(), 18, "A2/B N/A set is 18 cases");
    for slug in A1_EXECUTED {
        assert!(
            !A2B_NOT_APPLICABLE.contains(slug),
            "{slug} cannot be in both families"
        );
    }
    // No former list_contributions / delete_opt / list_queries skip-stub
    // survives: every case whose slug names one of the reclassified operations
    // is now either a live REST execution (A1) or is absent — never a skip.
    for entry in registry().entries() {
        let id = entry.meta.id;
        let is_reclassified_op = id.starts_with("ctb/list-contributions")
            || id.starts_with("tpl/delete-opt")
            || id.starts_with("sqr/list-queries-empty")
            || id.starts_with("sqr/list-queries-select");
        if is_reclassified_op {
            assert!(
                matches!(entry.meta.binding, Binding::Rest(_)),
                "{id}: a reclassified A1 operation must be a live REST execution, got {:?}",
                entry.meta.binding
            );
        }
    }
}
