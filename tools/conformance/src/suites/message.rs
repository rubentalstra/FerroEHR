//! MESSAGING (EHR Extract + TDS) cases — the master13 spine.
//!
//! master13-func_tc_messaging.adoc ships **no concrete test case** (all
//! SM-operation subsections are `TBD` stubs), and it carries a
//! schedule defect: `I_EHR_EXTRACT.export_ehr()` appears **twice** (an authoring
//! duplicate). So every case is [`ScheduleTrace::EccOriginal`], stub-derived
//! (owner ruling 2026-07-13). openEHR Messaging is an OPTIONS-profile capability
//! with **no ITS-REST 1.1.0 binding**: EHR Extract / TDD are realized on the
//! `ehrbase-sm` native API only (`I_EHR_EXTRACT_SERVICE` / `I_TDD_SERVICE`), and
//! there is no REST route in `ehrbase-rest` that reaches export/import/TDD. The
//! ECC drives SUTs over HTTP only, so no part of Messaging is wire-exercisable.
//!
//! **Disposition: first-class `NotApplicable` (native-API-only), every case**
//! — Messaging has no ITS-REST wire binding anywhere, so the HTTP-only
//! instrument cannot exercise it against any SUT; per the owner ruling (a case
//! passes, fails, errors, or is N/A — never "skipped") each is a cited N/A, not
//! a skip. Each cites the real `app/ehrbase` testcontainer integration test
//! that proves the operation, so the capability's evidence is traceable off the
//! wire, never fabricated.
//! The `schedule` reproduces the chapter's literal interface
//! name (`I_EHR_EXTRACT` / `I_TDD`) while the `binding` keeps the SM-trait name
//! (`I_EHR_EXTRACT_SERVICE` / `I_TDD_SERVICE`) — both correct at their own layer;
//! the divergence is a schedule authoring quirk. An N/A fn
//! embeds its evidence pointer as a literal (a `CaseRun` is a bare `fn`
//! pointer).

use crate::engine::harness::{CaseError, CaseFuture, CaseRun, DataSetReport, RunContext};
use crate::engine::registry::CaseEntry;
use crate::model::case::{Binding, Capability, CaseMeta, Compare, Format, ScheduleTrace};
use crate::model::catalog::Area;

/// JSON is the (nominal) wire format the MESSAGING cases would run under.
const JSON: &[Format] = &[Format::Json];

/// Every registered MESSAGING case (10, all native-API-only).
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // ── I_EHR_EXTRACT_SERVICE — export ──────────────────────────────────
        case(
            "msg/export-ehrs",
            "EHR Extract — export whole EHR (export_ehrs)",
            Capability::MessagingEhrExtract,
            "SM I_EHR_EXTRACT_SERVICE.export_ehrs; RM EHR Extract IM (X_VERSIONED_*); CNF master13 §I_EHR_EXTRACT.export_ehr (TBD stub, listed twice — authoring duplicate)",
            "schedule stub (master13 §I_EHR_EXTRACT.export_ehr TBD, listed twice — authoring duplicate); derived from SM I_EHR_EXTRACT_SERVICE.export_ehrs",
            "I_EHR_EXTRACT_SERVICE.export_ehrs",
            skip_export_ehrs,
        ),
        case(
            "msg/export-ehr-extracts",
            "EHR Extract — spec-driven export (export_ehr_extracts)",
            Capability::MessagingEhrExtract,
            "SM I_EHR_EXTRACT_SERVICE.export_ehr_extracts (EXTRACT_ENTITY_MANIFEST + EXTRACT_VERSION_SPEC); CNF master13 §I_EHR_EXTRACT.export_ehr_extracts (TBD stub)",
            "schedule stub (master13 §I_EHR_EXTRACT.export_ehr_extracts TBD); derived from SM I_EHR_EXTRACT_SERVICE.export_ehr_extracts",
            "I_EHR_EXTRACT_SERVICE.export_ehr_extracts",
            skip_export_ehr_extracts,
        ),
        case(
            "msg/export-unknown-ehr",
            "EHR Extract — export of unknown EHR fails",
            Capability::MessagingEhrExtract,
            "SM I_EHR_EXTRACT_SERVICE.export_ehrs (ehr_id_does_not_exist precondition); CNF master13 §I_EHR_EXTRACT.export_ehr (TBD stub)",
            "schedule stub (master13 §I_EHR_EXTRACT.export_ehr TBD); derived from SM I_EHR_EXTRACT_SERVICE.export_ehrs (unknown EHR)",
            "I_EHR_EXTRACT_SERVICE.export_ehrs",
            skip_export_unknown_ehr,
        ),
        // ── I_EHR_EXTRACT_SERVICE — import (RM common master06 §Copying) ────
        case(
            "msg/import-ehr-clone",
            "EHR Extract — import whole-EHR clone reusing source id",
            Capability::MessagingEhrExtract,
            "SM I_EHR_EXTRACT_SERVICE.import_ehr; RM common master06 §Copying Case 1 (reuse source EHR identifier); CNF master13 (import subsection absent — RM-backed)",
            "schedule silent on import (master13 lists export + TDD only); RM common master06 §Copying Case 1; derived from SM I_EHR_EXTRACT_SERVICE.import_ehr",
            "I_EHR_EXTRACT_SERVICE.import_ehr",
            skip_import_ehr_clone,
        ),
        case(
            "msg/import-ehr-fixed-id",
            "EHR Extract — import whole EHR into a caller-fixed id",
            Capability::MessagingEhrExtract,
            "SM I_EHR_EXTRACT_SERVICE.import_ehr (same patient in another EHR service); RM common master06 §Copying; CNF master13 (import subsection absent — RM-backed)",
            "schedule silent on import (master13 lists export + TDD only); RM common master06 §Copying; derived from SM I_EHR_EXTRACT_SERVICE.import_ehr",
            "I_EHR_EXTRACT_SERVICE.import_ehr",
            skip_import_ehr_fixed_id,
        ),
        case(
            "msg/import-ehr-duplicate",
            "EHR Extract — import into a duplicate target id fails",
            Capability::MessagingEhrExtract,
            "SM I_EHR_EXTRACT_SERVICE.import_ehr (ehr_create_fail_duplicate_id); RM common master06 §Copying; CNF master13 (import subsection absent — RM-backed)",
            "schedule silent on import (master13 lists export + TDD only); RM common master06 §Copying; derived from SM I_EHR_EXTRACT_SERVICE.import_ehr (duplicate target)",
            "I_EHR_EXTRACT_SERVICE.import_ehr",
            skip_import_ehr_duplicate,
        ),
        case(
            "msg/import-ehr-extract",
            "EHR Extract — import extract into an existing EHR",
            Capability::MessagingEhrExtract,
            "SM I_EHR_EXTRACT_SERVICE.import_ehr_extract; RM common master06 §Copying Case 2 (first receipt clones VERSIONED_OBJECT; re-import is a conflict); CNF master13 (import subsection absent — RM-backed)",
            "schedule silent on import (master13 lists export + TDD only); RM common master06 §Copying Case 2; derived from SM I_EHR_EXTRACT_SERVICE.import_ehr_extract",
            "I_EHR_EXTRACT_SERVICE.import_ehr_extract",
            skip_import_ehr_extract,
        ),
        // ── I_TDD_SERVICE — TDD import ──────────────────────────────────────
        case(
            "msg/tdd-import-commits",
            "TDD — import a TDD as a committed COMPOSITION",
            Capability::MessagingTds,
            "SM I_TDD_SERVICE.import_tdd; TDD → COMPOSITION over OPT/WebTemplate (openehr_flat::tdd::from_tdd); CNF master13 §I_TDD.import_tdd (TBD stub)",
            "schedule stub (master13 §I_TDD.import_tdd TBD); derived from SM I_TDD_SERVICE.import_tdd",
            "I_TDD_SERVICE.import_tdd",
            skip_tdd_import_commits,
        ),
        case(
            "msg/tdd-import-rejects",
            "TDD — import rejects malformed / non-TDD / unknown EHR / unknown template",
            Capability::MessagingTds,
            "SM I_TDD_SERVICE.import_tdd (typed envelope rejections); CNF master13 §I_TDD.import_tdd (TBD stub)",
            "schedule stub (master13 §I_TDD.import_tdd TBD); derived from SM I_TDD_SERVICE.import_tdd (typed rejections)",
            "I_TDD_SERVICE.import_tdd",
            skip_tdd_import_rejects,
        ),
        case(
            "msg/tdd-import-tdds-batch",
            "TDD — batch import commits all, fail-fast on error",
            Capability::MessagingTds,
            "SM I_TDD_SERVICE.import_tdds; CNF master13 §I_TDD.import_tdds (TBD stub)",
            "schedule stub (master13 §I_TDD.import_tdds TBD); derived from SM I_TDD_SERVICE.import_tdds",
            "I_TDD_SERVICE.import_tdds",
            skip_tdd_import_tdds_batch,
        ),
    ]
}

/// Assemble a native-API-only MESSAGING case entry (area [`Area::Msg`], JSON).
fn case(
    id: &'static str,
    title: &'static str,
    capability: Capability,
    citation: &'static str,
    schedule: &'static str,
    sm_op: &'static str,
    run: CaseRun,
) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Msg,
            capability,
            formats: JSON,
            citation,
            schedule: ScheduleTrace::EccOriginal(schedule),
            binding: Binding::NativeApiOnly(sm_op),
            compare: Compare::None,
        },
        run,
    }
}

/// Generate a first-class `NotApplicable` (native-API-only) run function
/// embedding its cited evidence as a literal (a `CaseRun` is a bare `fn`
/// pointer and cannot close over it). The cited `app/ehrbase` integration-test
/// names must stay in lockstep with `app/ehrbase/tests/` (a stale citation
/// silently breaks the off-wire traceability that is the whole evidentiary
/// basis).
macro_rules! na_fn {
    ($name:ident, $reason:literal) => {
        fn $name<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            Box::pin(async move {
                Err::<DataSetReport, _>(CaseError::NotApplicable($reason.to_owned()))
            })
        }
    };
}

na_fn!(
    skip_export_ehrs,
    "NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs is exercised by \
     app/ehrbase/tests/service_extract.rs::export_ehrs_carries_every_versioned_object_latest_only \
     — Messaging has no ITS-REST binding"
);
na_fn!(
    skip_export_ehr_extracts,
    "NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehr_extracts is exercised by \
     app/ehrbase/tests/service_extract.rs::export_ehr_extracts_honours_item_list_and_all_versions \
     — Messaging has no ITS-REST binding"
);
na_fn!(
    skip_export_unknown_ehr,
    "NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs (unknown EHR) is exercised by \
     app/ehrbase/tests/service_extract.rs::export_ehrs_unknown_ehr_is_ehr_id_does_not_exist \
     — Messaging has no ITS-REST binding"
);
na_fn!(
    skip_import_ehr_clone,
    "NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr is exercised by \
     app/ehrbase/tests/service_import.rs::import_ehr_clone_into_fresh_target_reuses_source_id \
     — Messaging has no ITS-REST binding"
);
na_fn!(
    skip_import_ehr_fixed_id,
    "NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (fixed id) is exercised by \
     app/ehrbase/tests/service_import.rs::import_ehr_into_fixed_fresh_id \
     — Messaging has no ITS-REST binding"
);
na_fn!(
    skip_import_ehr_duplicate,
    "NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (duplicate id) is exercised by \
     app/ehrbase/tests/service_import.rs::import_ehr_duplicate_target_is_rejected \
     — Messaging has no ITS-REST binding"
);
na_fn!(
    skip_import_ehr_extract,
    "NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr_extract is exercised by \
     app/ehrbase/tests/service_import.rs::import_ehr_extract_adds_a_versioned_object_and_rejects_re_import \
     — Messaging has no ITS-REST binding"
);
na_fn!(
    skip_tdd_import_commits,
    "NativeApiOnly: I_TDD_SERVICE.import_tdd is exercised by \
     app/ehrbase/tests/service_tdd.rs::tdd_import_commits_composition \
     — Messaging has no ITS-REST binding"
);
na_fn!(
    skip_tdd_import_rejects,
    "NativeApiOnly: I_TDD_SERVICE.import_tdd (typed rejections) is exercised by \
     app/ehrbase/tests/service_tdd.rs::{tdd_import_rejects_malformed_payload, \
     tdd_import_rejects_non_tdd_xml, tdd_import_rejects_unknown_ehr, \
     tdd_import_rejects_unknown_template} — Messaging has no ITS-REST binding"
);
na_fn!(
    skip_tdd_import_tdds_batch,
    "NativeApiOnly: I_TDD_SERVICE.import_tdds is exercised by \
     app/ehrbase/tests/service_tdd.rs::{tdd_import_tdds_batch_commits_all, \
     tdd_import_tdds_batch_fail_fast} — Messaging has no ITS-REST binding"
);
