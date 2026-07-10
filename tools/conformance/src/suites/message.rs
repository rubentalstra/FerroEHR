//! The `MSG` capability cases — the Messaging service (SM-5): EHR Extract
//! export/import plus TDD import. Reference reading: the CNF schedule
//! `master13-func_tc_messaging.adoc` (the `MESSAGE_SERVICE` suite), which ships
//! placeholder `aaaa`/`bbbb` headings with `TBD` bodies, so the concrete intents
//! are the SM operations it lists: the `I_EHR_EXTRACT` export family and the
//! `I_TDD` import family.
//!
//! **Disposition: skip-with-reason (`SKIPPED(NativeApiOnly)`), every case.**
//! openEHR Messaging is an OPTIONS-profile capability with **no ITS-REST 1.0.3
//! binding** — the constraint surfaced building SM-5: EHR Extract / TDD are
//! realized on the `ehrbase-sm` native API only (SM `I_EHR_EXTRACT_SERVICE` /
//! `I_TDD_SERVICE`, `docs/design/sm-platform/10-message-integration.md`), and
//! there is no REST route that reaches export/import/TDD (verified: no messaging
//! route exists in `ehrbase-rest`). The ECC drives SUTs over HTTP only, so no
//! part of Messaging is wire-exercisable — not even a side effect: TDD import
//! *would* produce a COMPOSITION readable over REST, but there is no ingestion
//! wire to invoke it. Rather than fabricate a pass, each case reports
//! `SKIPPED(NativeApiOnly)` and cites the real `app/ehrbase` testcontainer
//! integration test that proves the operation, so the capability's evidence is
//! traceable even though it is off the ECC transport.
//!
//! Spec grounding: the SM UML classes
//! `docs/specs/openehr/SM/docs/UML/classes/i_ehr_extract_service.adoc` and
//! `i_tdd_service.adoc` (both included by `master09-message_service.adoc`); the
//! RM EHR Extract IM (`X_VERSIONED_*`) together with RM common master06
//! §Copying (`IMPORTED_VERSION` Cases one/two/three); and the design
//! `docs/design/sm-platform/10-message-integration.md`.

use crate::case::{Capability, CaseMeta, Compare, Format, Profile};
use crate::catalog::Area;
use crate::harness::{CaseError, CaseFuture, CaseRun, DataSetReport, RunContext};
use crate::registry::CaseEntry;

/// The Messaging (SM-5) case entries — all native-API-only, all `SKIPPED`.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // ── I_EHR_EXTRACT_SERVICE — export ──────────────────────────────────
        entry(
            "msg/export-ehrs",
            "EHR Extract — export whole EHR (export_ehrs)",
            "SM I_EHR_EXTRACT_SERVICE.export_ehrs; RM EHR Extract IM master05 (X_VERSIONED_*), \
             master09 (creation); CNF master13 §I_EHR_EXTRACT.export_ehrs (TBD)",
            skip_export_ehrs,
        ),
        entry(
            "msg/export-ehr-extracts",
            "EHR Extract — spec-driven export (export_ehr_extracts)",
            "SM I_EHR_EXTRACT_SERVICE.export_ehr_extracts (EXTRACT_ENTITY_MANIFEST + \
             EXTRACT_VERSION_SPEC); CNF master13 §I_EHR_EXTRACT.export_ehr_extracts (TBD)",
            skip_export_ehr_extracts,
        ),
        entry(
            "msg/export-unknown-ehr",
            "EHR Extract — export of unknown EHR fails",
            "SM I_EHR_EXTRACT_SERVICE.export_ehrs (ehr_id_does_not_exist precondition); \
             CNF master13 §I_EHR_EXTRACT.export_ehr (TBD)",
            skip_export_unknown_ehr,
        ),
        // ── I_EHR_EXTRACT_SERVICE — import ──────────────────────────────────
        entry(
            "msg/import-ehr-clone",
            "EHR Extract — import whole-EHR clone reusing source id (import_ehr)",
            "SM I_EHR_EXTRACT_SERVICE.import_ehr; RM common master06 §Copying Case 1 \
             (reuse source EHR identifier); CNF master13 (TBD)",
            skip_import_ehr_clone,
        ),
        entry(
            "msg/import-ehr-fixed-id",
            "EHR Extract — import whole EHR into a caller-fixed id (import_ehr)",
            "SM I_EHR_EXTRACT_SERVICE.import_ehr (same patient in another EHR service); \
             RM common master06 §Copying; CNF master13 (TBD)",
            skip_import_ehr_fixed_id,
        ),
        entry(
            "msg/import-ehr-duplicate",
            "EHR Extract — import into a duplicate target id fails",
            "SM I_EHR_EXTRACT_SERVICE.import_ehr (ehr_create_fail_duplicate_id); CNF master13 (TBD)",
            skip_import_ehr_duplicate,
        ),
        entry(
            "msg/import-ehr-extract",
            "EHR Extract — import extract into an existing EHR (import_ehr_extract)",
            "SM I_EHR_EXTRACT_SERVICE.import_ehr_extract; RM common master06 §Copying Case 2 \
             (first receipt clones VERSIONED_OBJECT; re-import is a conflict); CNF master13 (TBD)",
            skip_import_ehr_extract,
        ),
        // ── I_TDD_SERVICE — TDD import ──────────────────────────────────────
        entry(
            "msg/tdd-import-commits",
            "TDD — import a TDD as a committed COMPOSITION (import_tdd)",
            "SM I_TDD_SERVICE.import_tdd; TDD → COMPOSITION over OPT/WebTemplate \
             (openehr_flat::from_tdd); CNF master13 §I_TDD.import_tdd (TBD)",
            skip_tdd_import_commits,
        ),
        entry(
            "msg/tdd-import-rejects",
            "TDD — import rejects malformed / non-TDD / unknown EHR / unknown template",
            "SM I_TDD_SERVICE.import_tdd (typed envelope rejections); CNF master13 \
             §I_TDD.import_tdd (TBD)",
            skip_tdd_import_rejects,
        ),
        entry(
            "msg/tdd-import-tdds-batch",
            "TDD — batch import commits all, fail-fast on error (import_tdds)",
            "SM I_TDD_SERVICE.import_tdds; CNF master13 §I_TDD.import_tdds (TBD)",
            skip_tdd_import_tdds_batch,
        ),
    ]
}

/// A Messaging case entry (MSG area, OPTIONS-profile `Messaging` capability).
fn entry(id: &'static str, title: &'static str, citation: &'static str, run: CaseRun) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Msg,
            capability: Capability::Messaging,
            profiles: &[Profile::Options],
            formats: &[Format::Json],
            citation,
            compare: Compare::Superset,
        },
        run,
    }
}

/// Generate a `SKIPPED(NativeApiOnly)` run function. Messaging has no ITS-REST
/// binding (module docs), so the ECC transport cannot reach it; each case names
/// the real `app/ehrbase` integration test that exercises the operation so the
/// evidence is never fabricated, only relocated off the wire.
macro_rules! skip_case {
    ($name:ident, $reason:literal) => {
        fn $name<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            Box::pin(async move { Err::<DataSetReport, _>(CaseError::Skipped($reason.to_owned())) })
        }
    };
}

skip_case!(
    skip_export_ehrs,
    "NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs is exercised by \
     app/ehrbase/tests/service_extract.rs::export_ehrs_carries_every_versioned_object_latest_only \
     — Messaging has no ITS-REST binding"
);
skip_case!(
    skip_export_ehr_extracts,
    "NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehr_extracts is exercised by \
     app/ehrbase/tests/service_extract.rs::export_ehr_extracts_honours_item_list_and_all_versions \
     — Messaging has no ITS-REST binding"
);
skip_case!(
    skip_export_unknown_ehr,
    "NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs (unknown EHR) is exercised by \
     app/ehrbase/tests/service_extract.rs::export_ehrs_unknown_ehr_is_ehr_id_does_not_exist \
     — Messaging has no ITS-REST binding"
);
skip_case!(
    skip_import_ehr_clone,
    "NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr is exercised by \
     app/ehrbase/tests/service_import.rs::import_ehr_clone_into_fresh_target_reuses_source_id \
     — Messaging has no ITS-REST binding"
);
skip_case!(
    skip_import_ehr_fixed_id,
    "NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (fixed id) is exercised by \
     app/ehrbase/tests/service_import.rs::import_ehr_into_fixed_fresh_id \
     — Messaging has no ITS-REST binding"
);
skip_case!(
    skip_import_ehr_duplicate,
    "NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (duplicate id) is exercised by \
     app/ehrbase/tests/service_import.rs::import_ehr_duplicate_target_is_rejected \
     — Messaging has no ITS-REST binding"
);
skip_case!(
    skip_import_ehr_extract,
    "NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr_extract is exercised by \
     app/ehrbase/tests/service_import.rs::import_ehr_extract_adds_a_versioned_object_and_rejects_re_import \
     — Messaging has no ITS-REST binding"
);
skip_case!(
    skip_tdd_import_commits,
    "NativeApiOnly: I_TDD_SERVICE.import_tdd is exercised by \
     app/ehrbase/tests/service_tdd.rs::tdd_import_commits_composition \
     — Messaging has no ITS-REST binding"
);
skip_case!(
    skip_tdd_import_rejects,
    "NativeApiOnly: I_TDD_SERVICE.import_tdd (typed rejections) is exercised by \
     app/ehrbase/tests/service_tdd.rs::{tdd_import_rejects_malformed_payload, \
     tdd_import_rejects_non_tdd_xml, tdd_import_rejects_unknown_ehr, \
     tdd_import_rejects_unknown_template} — Messaging has no ITS-REST binding"
);
skip_case!(
    skip_tdd_import_tdds_batch,
    "NativeApiOnly: I_TDD_SERVICE.import_tdds is exercised by \
     app/ehrbase/tests/service_tdd.rs::{tdd_import_tdds_batch_commits_all, \
     tdd_import_tdds_batch_fail_fast} — Messaging has no ITS-REST binding"
);
