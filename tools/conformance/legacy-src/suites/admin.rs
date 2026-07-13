//! ADMIN (physical delete) cases — our own ECC cases (reference:
//! `master12-func_tc_admin.adoc`, design-time reading).
//!
//! The upstream CNF `master12-func_tc_admin.adoc` chapter ships only placeholder
//! `aaaa`/`bbbb` headings (no concrete cases), so these `ADM` cases are our own
//! spec-grounded functional cases against the **ITS-REST admin API**, which
//! realizes SM `I_ADMIN_SERVICE.physical_ehr_delete` (`docs/specs/openehr/SM/...`)
//! and the upstream Robot prior art
//! (`CNF/tests/platform/robot/I_ADMIN_SERVICE/001-EHR.robot`): a full physical
//! cascade delete. The whole admin surface is exactly two operations —
//! `DELETE /admin/ehr/{ehr_id}` and `DELETE /admin/ehr/all{?ehr_id*}` — so these
//! cases fully cover it.
//!
//! Contract: `204` physical delete of an existing EHR; `404` for an unknown EHR
//! (and for a re-delete — idempotent); `204 No Content` (bodyless) for the bulk
//! delete — including a subset with missing ids (skipped) and an **absent
//! `ehr_id`**, which deletes **all** EHRs per
//! `operations/admin_ehr_delete_all.yaml` (success → `204`, `responses/204_deleted_hard.yaml`)
//! + the optional `ehr_id` selector (`parameters/query/ehr_id_Admin.yaml`). The
//!   admin group is config-gated (`RestConfig::admin.enabled`); the compose dev
//!   config enables it, so these exercise the *active* surface.

use uuid::Uuid;

use crate::assert;
use crate::case::{Capability, CaseMeta, Compare, Format, Profile};
use crate::catalog::Area;
use crate::harness::{
    AuthSlot, CaseFuture, CaseRun, DataSetReport, HttpRequest, Method, RunContext,
};
use crate::registry::CaseEntry;
use crate::suites::support;

/// The implemented ADMIN case entries.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        entry(
            "adm/ehr-delete",
            "Admin EHR delete",
            "ITS-REST 1.0.3 ADMIN API §delete EHR; SM §I_ADMIN_SERVICE.physical_ehr_delete",
            run_delete,
        ),
        entry(
            "adm/ehr-delete-absent",
            "Admin EHR delete absent",
            "ITS-REST 1.0.3 ADMIN API §delete EHR; SM §I_ADMIN_SERVICE.physical_ehr_delete",
            run_delete_absent,
        ),
        entry(
            "adm/ehr-delete-idempotent",
            "Admin EHR delete idempotent",
            "ITS-REST 1.0.3 ADMIN API §delete EHR; SM §I_ADMIN_SERVICE.physical_ehr_delete",
            run_delete_idempotent,
        ),
        entry(
            "adm/ehr-delete-all",
            "Admin EHR delete all",
            "ITS-REST 1.0.3 ADMIN API §delete EHR; SM §I_ADMIN_SERVICE.physical_ehr_delete",
            run_delete_all,
        ),
        entry(
            "adm/ehr-delete-all-partial",
            "Admin EHR delete all partial",
            "ITS-REST 1.0.3 ADMIN API §delete EHR; SM §I_ADMIN_SERVICE.physical_ehr_delete",
            run_delete_all_partial,
        ),
        entry(
            "adm/ehr-delete-all-empty",
            "Admin EHR delete all empty",
            "ITS-REST 1.0.3 ADMIN API §delete EHR; SM §I_ADMIN_SERVICE.physical_ehr_delete",
            run_delete_all_empty,
        ),
    ]
}

fn entry(id: &'static str, title: &'static str, citation: &'static str, run: CaseRun) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Adm,
            capability: Capability::AdminApi,
            profiles: &[Profile::Options],
            formats: &[Format::Json],
            citation,
            compare: Compare::Superset,
            schedule_ref: None,
        },
        run,
    }
}

macro_rules! case {
    ($body:block) => {
        Box::pin(async move { $body })
    };
}

/// `DELETE /admin/ehr/{id}` on an existing EHR → `204` physical delete.
fn run_delete<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::new(Method::Delete, format!("/admin/ehr/{ehr_id}"))
                    .with_auth(AuthSlot::Admin),
            )
            .await?;
        assert::status(&resp, 204)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// `DELETE /admin/ehr/{unknown}` → `404`.
fn run_delete_absent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(
                HttpRequest::new(Method::Delete, format!("/admin/ehr/{}", Uuid::new_v4()))
                    .with_auth(AuthSlot::Admin),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// Physical delete is idempotent-observable: a second delete of the now-gone EHR
/// is `404`.
fn run_delete_idempotent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let first = ctx
            .send(
                HttpRequest::new(Method::Delete, format!("/admin/ehr/{ehr_id}"))
                    .with_auth(AuthSlot::Admin),
            )
            .await?;
        assert::status(&first, 204)?;
        let second = ctx
            .send(
                HttpRequest::new(Method::Delete, format!("/admin/ehr/{ehr_id}"))
                    .with_auth(AuthSlot::Admin),
            )
            .await?;
        assert::status(&second, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// `DELETE /admin/ehr/all?ehr_id=a&ehr_id=b` → `204 No Content` (bodyless).
///
/// `operations/admin_ehr_delete_all.yaml`: a synchronous bulk delete succeeds
/// with `204` (`responses/204_deleted_hard.yaml`), not a `200 {"deleted": n}`
/// body (which the OAS never declares).
fn run_delete_all<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let a = support::create_ehr(ctx).await?;
        let b = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::new(
                    Method::Delete,
                    format!("/admin/ehr/all?ehr_id={a}&ehr_id={b}"),
                )
                .with_auth(AuthSlot::Admin),
            )
            .await?;
        assert::status(&resp, 204)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// A subset delete with a missing id still succeeds `204` (the absent id is
/// skipped; `operations/admin_ehr_delete_all.yaml` declares no per-id failure
/// for the bulk operation).
fn run_delete_all_partial<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let a = support::create_ehr(ctx).await?;
        let missing = Uuid::new_v4();
        let resp = ctx
            .send(
                HttpRequest::new(
                    Method::Delete,
                    format!("/admin/ehr/all?ehr_id={a}&ehr_id={missing}"),
                )
                .with_auth(AuthSlot::Admin),
            )
            .await?;
        assert::status(&resp, 204)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// An **absent** `ehr_id` selects the full EHR set: per
/// `operations/admin_ehr_delete_all.yaml` ("Deletes all or multiple EHRs") +
/// the optional `ehr_id` selector (`parameters/query/ehr_id_Admin.yaml`), the
/// bulk delete with no selector deletes **all** EHRs and succeeds `204`.
fn run_delete_all_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(HttpRequest::new(Method::Delete, "/admin/ehr/all").with_auth(AuthSlot::Admin))
            .await?;
        assert::status(&resp, 204)?;
        Ok(DataSetReport::SINGLE)
    })
}
