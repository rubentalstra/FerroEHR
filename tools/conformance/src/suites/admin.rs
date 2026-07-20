//! ADMIN cases — the master12 spine.
//!
//! master12-func_tc_admin.adoc ships **no concrete test case** (all 9
//! SM-operation subsections are `TBD` stubs), so every case is
//! [`ScheduleTrace::EccOriginal`], stub-derived: the honest provenance is the SM
//! operation heading + the ADMIN OAS, never a schedule-conformant claim (owner
//! ruling 2026-07-13). Admin is wholly OPTIONS (`master03-profiles.adoc`) — its
//! absence never dents CORE/STANDARD, and for a foreign SUT it is a per-case
//! fairness decision.
//!
//! The implemented ITS-REST admin wire is exactly two routes —
//! `DELETE /admin/ehr/{ehr_id}` and `DELETE /admin/ehr/all{?ehr_id*}`
//! (`crates/openehr-its/vendor/rest-oas/admin-codegen.openapi.yaml`), realizing
//! `physical_ehr_delete`. The other eight SM operations have no ITS-REST route
//! the HTTP-only ECC can reach: six are native-API-only skip-with-reason cases
//! citing the `app/ehrbase` integration test that proves them (the Messaging
//! precedent); two act on demographic PARTYs and are
//! `NoRestBinding`. Each skip fn embeds its reason as a
//! literal — a `CaseRun` is a bare `fn` pointer and cannot close over one.

use uuid::Uuid;

use crate::edition::Edition;
use crate::engine::assert;
use crate::engine::harness::{
    AuthSlot, CaseError, CaseFuture, CaseRun, DataSetReport, HttpRequest, RunContext,
};
use crate::engine::registry::CaseEntry;
use crate::model::case::{Binding, Capability, CaseMeta, Compare, Format, ScheduleTrace};
use crate::model::catalog::Area;
use crate::suites::support;
use crate::sut::descriptor::SutKind;

/// JSON is the wire format the ADMIN cases run under.
const JSON: &[Format] = &[Format::Json];

/// Single-rung ladders: the ADMIN API is a DEVELOPMENT-status ITS-REST API
/// (no Release-1.0.3 rung).
const DELETED: &[(Edition, u16)] = &[(Edition::Development, 204)];
const ABSENT: &[(Edition, u16)] = &[(Edition::Development, 404)];

/// Every registered ADMIN case (6 physical-delete + 8 missing SM ops).
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "the registered ECC case table is inherently enumerative"
)]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // ── I_ADMIN_SERVICE.physical_ehr_delete (the one bound operation) ───
        case(
            "adm/ehr-delete",
            "Admin EHR delete",
            Capability::AdminPhysicalDeletion,
            "CNF master12 §physical_ehr_delete (TBD stub); ITS-REST DEVELOPMENT admin admin_ehr_delete.yaml 204; SM I_ADMIN_SERVICE.physical_ehr_delete",
            pd_stub(),
            Binding::Rest("DELETE /admin/ehr/{ehr_id}"),
            run_delete,
        ),
        case(
            "adm/ehr-delete-absent",
            "Admin EHR delete absent",
            Capability::AdminPhysicalDeletion,
            "CNF master12 §physical_ehr_delete (TBD stub); ITS-REST DEVELOPMENT admin admin_ehr_delete.yaml 404; SM I_ADMIN_SERVICE.physical_ehr_delete",
            pd_stub(),
            Binding::Rest("DELETE /admin/ehr/{ehr_id}"),
            run_delete_absent,
        ),
        case(
            "adm/ehr-delete-idempotent",
            "Admin EHR delete idempotent",
            Capability::AdminPhysicalDeletion,
            "CNF master12 §physical_ehr_delete (TBD stub); ITS-REST DEVELOPMENT admin admin_ehr_delete.yaml 204 then 404 (physical delete leaves no trace); SM I_ADMIN_SERVICE.physical_ehr_delete",
            pd_stub(),
            Binding::Rest("DELETE /admin/ehr/{ehr_id}"),
            run_delete_idempotent,
        ),
        case(
            "adm/ehr-delete-all",
            "Admin EHR delete all",
            Capability::AdminPhysicalDeletion,
            "CNF master12 §physical_ehr_delete (TBD stub); ITS-REST DEVELOPMENT admin admin_ehr_delete_all.yaml 204 (204_deleted_hard, bodyless); SM I_ADMIN_SERVICE.physical_ehr_delete",
            pd_stub(),
            Binding::Rest("DELETE /admin/ehr/all?ehr_id*"),
            run_delete_all,
        ),
        case(
            "adm/ehr-delete-all-partial",
            "Admin EHR delete all partial",
            Capability::AdminPhysicalDeletion,
            "CNF master12 §physical_ehr_delete (TBD stub); ITS-REST DEVELOPMENT admin admin_ehr_delete_all.yaml — instrument-encodes-server-behaviour: a bulk set including a missing id still 204s (OAS declares no per-id failure)",
            pd_stub(),
            Binding::Rest("DELETE /admin/ehr/all?ehr_id*"),
            run_delete_all_partial,
        ),
        case(
            "adm/ehr-delete-all-empty",
            "Admin EHR delete all (empty selector)",
            Capability::AdminPhysicalDeletion,
            "CNF master12 §physical_ehr_delete (TBD stub); ITS-REST DEVELOPMENT admin admin_ehr_delete_all.yaml + ehr_id_Admin.yaml optional selector — instrument-encodes-server-behaviour: an absent ehr_id deletes ALL EHRs (a globally destructive design reading, gated to disposable SUTs)",
            pd_stub(),
            Binding::Rest("DELETE /admin/ehr/all"),
            run_delete_all_empty,
        ),
        // ── I_ADMIN_SERVICE.* Activity Report — native-API-only ───────
        native(
            "adm/list-contributions",
            "Admin list contributions",
            Capability::AdminActivityReport,
            "CNF master12 §list_contributions (TBD stub); SM I_ADMIN_SERVICE.list_contributions — no ITS-REST admin route",
            "schedule stub (master12 §list_contributions TBD); derived from SM I_ADMIN_SERVICE.list_contributions — native-API-only",
            "I_ADMIN_SERVICE.list_contributions",
            skip_list_contributions,
        ),
        native(
            "adm/contribution-count",
            "Admin contribution count",
            Capability::AdminActivityReport,
            "CNF master12 §contribution_count (TBD stub); SM I_ADMIN_SERVICE.contribution_count — no ITS-REST admin route",
            "schedule stub (master12 §contribution_count TBD); derived from SM I_ADMIN_SERVICE.contribution_count — native-API-only",
            "I_ADMIN_SERVICE.contribution_count",
            skip_contribution_count,
        ),
        native(
            "adm/versioned-composition-count",
            "Admin versioned composition count",
            Capability::AdminActivityReport,
            "CNF master12 §versioned_composition_count (TBD stub); SM I_ADMIN_SERVICE.versioned_composition_count — no ITS-REST admin route",
            "schedule stub (master12 §versioned_composition_count TBD); derived from SM I_ADMIN_SERVICE.versioned_composition_count — native-API-only",
            "I_ADMIN_SERVICE.versioned_composition_count",
            skip_versioned_composition_count,
        ),
        native(
            "adm/composition-version-count",
            "Admin composition version count",
            Capability::AdminActivityReport,
            "CNF master12 §composition_version_count (TBD stub); SM I_ADMIN_SERVICE.composition_version_count — no ITS-REST admin route",
            "schedule stub (master12 §composition_version_count TBD); derived from SM I_ADMIN_SERVICE.composition_version_count — native-API-only",
            "I_ADMIN_SERVICE.composition_version_count",
            skip_composition_version_count,
        ),
        // ── I_ADMIN_DUMP_LOAD / I_ADMIN_ARCHIVE — native-API-only ─────
        native(
            "adm/export-ehrs",
            "Admin export EHRs (dump/load)",
            Capability::AdminEhrDumpLoad,
            "CNF master12 §export_ehrs (TBD stub); SM I_ADMIN_DUMP_LOAD.export_ehrs/load_ehrs — no ITS-REST admin route",
            "schedule stub (master12 §export_ehrs TBD); derived from SM I_ADMIN_DUMP_LOAD.export_ehrs — native-API-only",
            "I_ADMIN_DUMP_LOAD.export_ehrs",
            skip_export_ehrs,
        ),
        native(
            "adm/archive-ehrs",
            "Admin archive EHRs",
            Capability::AdminEhrArchive,
            "CNF master12 §archive_ehrs (TBD stub); SM I_ADMIN_ARCHIVE.archive_ehrs — no ITS-REST admin route",
            "schedule stub (master12 §archive_ehrs TBD); derived from SM I_ADMIN_ARCHIVE.archive_ehrs — native-API-only",
            "I_ADMIN_ARCHIVE.archive_ehrs",
            skip_archive_ehrs,
        ),
        // ── demographic-dependent SM ops — no REST binding ────────────
        no_binding(
            "adm/physical-party-delete",
            "Admin physical party delete",
            Capability::AdminPhysicalDeletion,
            "CNF master12 §physical_party_delete (TBD stub); SM I_ADMIN_SERVICE.physical_party_delete acts on demographic PARTYs (ehrbase-rs demographic extension) — no ITS-REST admin route",
            "schedule stub (master12 §physical_party_delete TBD); derived from SM I_ADMIN_SERVICE.physical_party_delete — demographic-dependent, no ITS-REST binding",
            "I_ADMIN_SERVICE.physical_party_delete",
            skip_physical_party_delete,
        ),
        no_binding(
            "adm/archive-parties",
            "Admin archive parties",
            Capability::AdminDemographicArchive,
            "CNF master12 §archive_parties (TBD stub); SM I_ADMIN_ARCHIVE.archive_parties acts on demographic PARTYs (ehrbase-rs demographic extension) — no ITS-REST admin route",
            "schedule stub (master12 §archive_parties TBD); derived from SM I_ADMIN_ARCHIVE.archive_parties — demographic-dependent, no ITS-REST binding",
            "I_ADMIN_ARCHIVE.archive_parties",
            skip_archive_parties,
        ),
    ]
}

/// The stub-derived schedule trace for the one bound admin operation.
fn pd_stub() -> ScheduleTrace {
    ScheduleTrace::EccOriginal(
        "schedule stub (master12 §physical_ehr_delete TBD); derived from SM I_ADMIN_SERVICE.physical_ehr_delete + ADMIN OAS admin_ehr_delete[_all].yaml",
    )
}

/// Assemble an ADMIN case entry (area [`Area::Adm`], JSON).
fn case(
    id: &'static str,
    title: &'static str,
    capability: Capability,
    citation: &'static str,
    schedule: ScheduleTrace,
    binding: Binding,
    run: CaseRun,
) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Adm,
            capability,
            formats: JSON,
            citation,
            schedule,
            binding,
            compare: Compare::None,
        },
        run,
    }
}

/// A native-API-only admin case (`Binding::NativeApiOnly`, skip-with-reason).
fn native(
    id: &'static str,
    title: &'static str,
    capability: Capability,
    citation: &'static str,
    schedule: &'static str,
    sm_op: &'static str,
    run: CaseRun,
) -> CaseEntry {
    case(
        id,
        title,
        capability,
        citation,
        ScheduleTrace::EccOriginal(schedule),
        Binding::NativeApiOnly(sm_op),
        run,
    )
}

/// A demographic-dependent SM op with no ITS-REST binding.
fn no_binding(
    id: &'static str,
    title: &'static str,
    capability: Capability,
    citation: &'static str,
    schedule: &'static str,
    sm_op: &'static str,
    run: CaseRun,
) -> CaseEntry {
    case(
        id,
        title,
        capability,
        citation,
        ScheduleTrace::EccOriginal(schedule),
        Binding::NoRestBinding(sm_op),
        run,
    )
}

macro_rules! case_body {
    ($body:block) => {
        Box::pin(async move $body)
    };
}

/// Generate a distinct skip run function embedding its reason as a literal (a
/// `CaseRun` is a bare `fn` pointer and cannot close over the reason).
macro_rules! skip_fn {
    ($name:ident, $reason:literal) => {
        fn $name<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
            Box::pin(async move { Err::<DataSetReport, _>(CaseError::Skipped($reason.to_owned())) })
        }
    };
}

skip_fn!(
    skip_list_contributions,
    "NativeApiOnly: I_ADMIN_SERVICE.list_contributions is exercised by \
     app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary \
     — no ITS-REST admin route reaches it"
);
skip_fn!(
    skip_contribution_count,
    "NativeApiOnly: I_ADMIN_SERVICE.contribution_count is exercised by \
     app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary \
     — no ITS-REST admin route reaches it"
);
skip_fn!(
    skip_versioned_composition_count,
    "NativeApiOnly: I_ADMIN_SERVICE.versioned_composition_count is exercised by \
     app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary \
     — no ITS-REST admin route reaches it"
);
skip_fn!(
    skip_composition_version_count,
    "NativeApiOnly: I_ADMIN_SERVICE.composition_version_count is exercised by \
     app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary \
     — no ITS-REST admin route reaches it"
);
skip_fn!(
    skip_export_ehrs,
    "NativeApiOnly: I_ADMIN_DUMP_LOAD.export_ehrs/load_ehrs is exercised by \
     app/ehrbase/tests/service_dump_load.rs::export_then_load_into_fresh_db_round_trips_byte_equal \
     — no ITS-REST admin route reaches it"
);
skip_fn!(
    skip_archive_ehrs,
    "NativeApiOnly: I_ADMIN_ARCHIVE.archive_ehrs is exercised by \
     app/ehrbase/tests/service_admin.rs::archive_marks_vos_idempotently_and_reads_stay_unchanged \
     — no ITS-REST admin route reaches it"
);
skip_fn!(
    skip_physical_party_delete,
    "NoRestBinding: I_ADMIN_SERVICE.physical_party_delete has no ITS-REST route and acts on the \
     demographic extension; exercised natively by \
     app/ehrbase/tests/service_admin.rs::physical_party_delete_cascades_relationships_and_spares_partner"
);
skip_fn!(
    skip_archive_parties,
    "NoRestBinding: I_ADMIN_ARCHIVE.archive_parties has no ITS-REST route and acts on the \
     demographic extension; the archive path is proven natively by \
     app/ehrbase/tests/service_admin.rs::archive_marks_vos_idempotently_and_reads_stay_unchanged"
);

// ── physical_ehr_delete runs ─────────────────────────────────────────────────

fn run_delete<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(HttpRequest::delete(format!("/admin/ehr/{ehr_id}")).with_auth(AuthSlot::Admin))
            .await?;
        assert::status_ladder(ctx, &resp, DELETED, "physical_ehr_delete 204")?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_delete_absent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let resp = ctx
            .send(
                HttpRequest::delete(format!("/admin/ehr/{}", Uuid::new_v4()))
                    .with_auth(AuthSlot::Admin),
            )
            .await?;
        assert::status_ladder(ctx, &resp, ABSENT, "physical_ehr_delete absent 404")?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_delete_idempotent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let ehr_id = support::create_ehr(ctx).await?;
        let first = ctx
            .send(HttpRequest::delete(format!("/admin/ehr/{ehr_id}")).with_auth(AuthSlot::Admin))
            .await?;
        assert::status_ladder(ctx, &first, DELETED, "physical_ehr_delete 204")?;
        // Physical delete leaves no trace: a re-delete of the gone EHR is 404.
        let second = ctx
            .send(HttpRequest::delete(format!("/admin/ehr/{ehr_id}")).with_auth(AuthSlot::Admin))
            .await?;
        assert::status_ladder(ctx, &second, ABSENT, "physical_ehr_delete re-delete 404")?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_delete_all<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        let a = support::create_ehr(ctx).await?;
        let b = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::delete(format!("/admin/ehr/all?ehr_id={a}&ehr_id={b}"))
                    .with_auth(AuthSlot::Admin),
            )
            .await?;
        assert::status_ladder(ctx, &resp, DELETED, "physical_ehr_delete bulk 204")?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_delete_all_partial<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        // instrument-encodes-server-behaviour: a bulk set with a
        // missing id still 204s (the OAS declares no per-id failure).
        let a = support::create_ehr(ctx).await?;
        let missing = Uuid::new_v4();
        let resp = ctx
            .send(
                HttpRequest::delete(format!("/admin/ehr/all?ehr_id={a}&ehr_id={missing}"))
                    .with_auth(AuthSlot::Admin),
            )
            .await?;
        assert::status_ladder(ctx, &resp, DELETED, "physical_ehr_delete partial bulk 204")?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_delete_all_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case_body!({
        // instrument-encodes-server-behaviour + globally destructive: an absent
        // ehr_id deletes ALL EHRs. Gated to disposable composed SUTs (SutKind::Ours);
        // never run against a foreign / bring-your-own endpoint whose data must
        // survive.
        if ctx.sut.kind != SutKind::Ours {
            return Err(CaseError::Skipped(
                "destructive case runs only against disposable composed SUTs (an empty ehr_id \
                 selector deletes ALL EHRs); skipped for a foreign / bring-your-own endpoint"
                    .to_owned(),
            ));
        }
        let resp = ctx
            .send(HttpRequest::delete("/admin/ehr/all").with_auth(AuthSlot::Admin))
            .await?;
        assert::status_ladder(
            ctx,
            &resp,
            DELETED,
            "physical_ehr_delete empty-selector 204",
        )?;
        Ok(DataSetReport::SINGLE)
    })
}
