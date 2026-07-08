//! master12 — ADMIN (physical delete) cases — **runner-defined** (design §4.6).
//!
//! The upstream CNF `master12-func_tc_admin.adoc` chapter ships only placeholder
//! `aaaa`/`bbbb` headings (no concrete cases), so there is nothing to transcribe.
//! These `ADMIN-*` cases are our own spec-grounded functional cases against the
//! **ITS-REST admin API**, which realizes SM `I_ADMIN_SERVICE.physical_ehr_delete`
//! (`docs/specs/openehr/SM/...`) and the upstream Robot prior art
//! (`CNF/tests/platform/robot/I_ADMIN_SERVICE/001-EHR.robot`): a full physical
//! cascade delete. The whole admin surface is exactly two operations —
//! `DELETE /admin/ehr/{ehr_id}` and `DELETE /admin/ehr/all{?ehr_id*}` — so these
//! cases fully cover it.
//!
//! Contract: `204` physical delete of an existing EHR; `404` for an unknown EHR
//! (and for a re-delete — idempotent); `200 {"deleted": n}` for the bulk delete
//! (partial success: missing ids skipped); `400` for an empty bulk request. The
//! admin group is config-gated (`RestConfig::admin.enabled`); the self-hosted SUT
//! enables it (`sut::self_host`), so these exercise the *active* surface. They
//! carry [`Provenance::RunnerDefined`] and sit outside the 322-case inventory.

use serde_json::Value;
use uuid::Uuid;

use crate::assert;
use crate::case::{Capability, CaseMeta, Chapter, Compare, Format, Profile, Provenance};
use crate::harness::{
    CaseError, CaseFuture, CaseRun, DataSetReport, HttpRequest, Method, RunContext,
};
use crate::registry::CaseEntry;
use crate::suites::support;

/// The implemented master12 (runner-defined) case entries.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        entry("ADMIN-ehr-delete", run_delete),
        entry("ADMIN-ehr-delete_absent", run_delete_absent),
        entry("ADMIN-ehr-delete_idempotent", run_delete_idempotent),
        entry("ADMIN-ehr-delete_all", run_delete_all),
        entry("ADMIN-ehr-delete_all_partial", run_delete_all_partial),
        entry("ADMIN-ehr-delete_all_empty", run_delete_all_empty),
    ]
}

fn entry(id: &'static str, run: CaseRun) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            chapter: Chapter::Master12,
            capability: Capability::AdminApi,
            profiles: &[Profile::Options],
            formats: &[Format::Json],
            provenance: Provenance::RunnerDefined,
            schedule_ref: "master12-func_tc_admin.adoc (upstream placeholder) — runner-defined vs SM \
                 I_ADMIN_SERVICE.physical_ehr_delete + ITS-REST admin API",
            upstream_tags: &[],
            compare: Compare::Superset,
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
            .send(HttpRequest::new(
                Method::Delete,
                format!("/admin/ehr/{ehr_id}"),
            ))
            .await?;
        assert::status(&resp, 204)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// `DELETE /admin/ehr/{unknown}` → `404`.
fn run_delete_absent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(HttpRequest::new(
                Method::Delete,
                format!("/admin/ehr/{}", Uuid::new_v4()),
            ))
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
            .send(HttpRequest::new(
                Method::Delete,
                format!("/admin/ehr/{ehr_id}"),
            ))
            .await?;
        assert::status(&first, 204)?;
        let second = ctx
            .send(HttpRequest::new(
                Method::Delete,
                format!("/admin/ehr/{ehr_id}"),
            ))
            .await?;
        assert::status(&second, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// `DELETE /admin/ehr/all?ehr_id=a&ehr_id=b` → `200 {"deleted": 2}`.
fn run_delete_all<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let a = support::create_ehr(ctx).await?;
        let b = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(HttpRequest::new(
                Method::Delete,
                format!("/admin/ehr/all?ehr_id={a}&ehr_id={b}"),
            ))
            .await?;
        assert::status(&resp, 200)?;
        let body: Value = resp.json()?;
        if body["deleted"].as_u64() == Some(2) {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(format!(
                "expected {{\"deleted\": 2}}, got {body}"
            )))
        }
    })
}

/// Bulk delete is partial-success: a real + a missing id → `200 {"deleted": 1}`.
fn run_delete_all_partial<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let a = support::create_ehr(ctx).await?;
        let missing = Uuid::new_v4();
        let resp = ctx
            .send(HttpRequest::new(
                Method::Delete,
                format!("/admin/ehr/all?ehr_id={a}&ehr_id={missing}"),
            ))
            .await?;
        assert::status(&resp, 200)?;
        let body: Value = resp.json()?;
        if body["deleted"].as_u64() == Some(1) {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(format!(
                "expected {{\"deleted\": 1}} (partial success), got {body}"
            )))
        }
    })
}

/// An empty bulk request (no `ehr_id`) is refused with `400` (no implicit
/// delete-everything).
fn run_delete_all_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(HttpRequest::new(Method::Delete, "/admin/ehr/all"))
            .await?;
        assert::status(&resp, 400)?;
        Ok(DataSetReport::SINGLE)
    })
}
