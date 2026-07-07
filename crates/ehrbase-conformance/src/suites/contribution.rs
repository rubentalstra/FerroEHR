//! master08 — CONTRIBUTION cases (design §4.1: `suites/contribution.rs`).
//!
//! Transcribed from `master08-func_tc_ehr_contribution.adoc`, driving the
//! ITS-REST `/ehr/{ehr_id}/contribution` surface (JSON only — a CONTRIBUTION
//! commit is a version-set + audit wrapper with no canonical-XML wire shape).
//! Consumes the vendored `contributions/{valid,invalid}` fixtures in full: the
//! valid single-commit COMPOSITION / `EHR_STATUS` / FOLDER contributions, the
//! invalid `no_versions` / `multiple_valid_and_invalid_compos` /
//! `ref_to_non_existent_OPT` fixtures, and the `*.modification` / `*.deleted`
//! fixtures for the multi-commit flows (their placeholder
//! `preceding_version_uid` is patched with the uid returned by the first commit
//! — an additive fixture adaptation, never a defect fix, design §6).
//!
//! Assertions concretize the ITS-REST contribution contract
//! (`contribution_create.yaml` 201/400/404/409 — the schedule's "negative
//! response" is a `4xx`; `contribution_get.yaml` 200/404).
//!
//! The `has_contribution-*` and `list_contributions-*` schedule cases have no
//! dedicated ITS-REST endpoint on our surface (only commit + get-by-uid), so
//! they stay `NotYetTranscribed`.

use serde_json::{Value, json};
use uuid::Uuid;

use crate::assert;
use crate::case::{Capability, CaseMeta, Chapter, Compare, Format, Profile, Provenance};
use crate::fixtures;
use crate::harness::{
    CaseError, CaseFuture, CaseRun, DataSetReport, HttpRequest, HttpResponse, RunContext,
};
use crate::registry::CaseEntry;
use crate::suites::support;

/// The implemented master08 case entries.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // ── commit_contribution ──────────────────────────────────────────────
        entry(
            "I_EHR_CONTRIBUTION.commit_contribution-valid_composition",
            run_valid_composition,
        ),
        entry(
            "I_EHR_CONTRIBUTION.commit_contribution-invalid_composition",
            run_invalid_composition,
        ),
        entry("I_EHR_CONTRIBUTION.commit_contribution-empty", run_empty),
        entry(
            "I_EHR_CONTRIBUTION.commit_contribution-valid_invalid_compositions",
            run_valid_invalid_compositions,
        ),
        entry(
            "I_EHR_CONTRIBUTION.commit_contribution-non_exiting_opt",
            run_non_existing_opt,
        ),
        entry(
            "I_EHR_CONTRIBUTION.commit_contribution-event_composition",
            run_event_composition,
        ),
        entry(
            "I_EHR_CONTRIBUTION.commit_contribution-persistent_composition",
            run_persistent_composition,
        ),
        entry("I_EHR_CONTRIBUTION.commit_contribution-delete", run_delete),
        entry(
            "I_EHR_CONTRIBUTION.commit_contribution-two_commits_second_invalid",
            run_two_commits_second_invalid,
        ),
        entry(
            "I_EHR_CONTRIBUTION.commit_contribution-two_commits_second_creation",
            run_two_commits_second_creation,
        ),
        // ── EHR_STATUS contributions ─────────────────────────────────────────
        entry(
            "I_EHR_CONTRIBUTION.commit_contribution-minimal_ehr_status",
            run_minimal_ehr_status,
        ),
        entry(
            "I_EHR_CONTRIBUTION.commit_contribution-full_ehr_status",
            run_full_ehr_status,
        ),
        entry(
            "I_EHR_CONTRIBUTION.commit_contribution-ehr_status_invalid_change_type",
            run_ehr_status_invalid_change_type,
        ),
        entry(
            "I_EHR_CONTRIBUTION.commit_contribution-invalid_ehr_status",
            run_invalid_ehr_status,
        ),
        // ── FOLDER contributions ─────────────────────────────────────────────
        entry(
            "I_EHR_CONTRIBUTION.commit_contribution-valid_directory",
            run_valid_directory,
        ),
        entry(
            "I_EHR_CONTRIBUTION.commit_contribution-fail_create_existing_directory",
            run_fail_create_existing_directory,
        ),
        entry(
            "I_EHR_CONTRIBUTION.commit_contribution-fail_modify_non_existing_directory",
            run_fail_modify_non_existing_directory,
        ),
        entry(
            "I_EHR_CONTRIBUTION.commit_contribution-update_existing_directory",
            run_update_existing_directory,
        ),
        // ── get_contribution ─────────────────────────────────────────────────
        entry(
            "I_EHR_CONTRIBUTION.get_contribution-existing",
            run_get_existing,
        ),
        entry(
            "I_EHR_CONTRIBUTION.get_contribution-empty_ehr",
            run_get_empty_ehr,
        ),
        entry(
            "I_EHR_CONTRIBUTION.get_contribution-bad_ehr",
            run_get_bad_ehr,
        ),
        entry(
            "I_EHR_CONTRIBUTION.get_contribution-bad_contribution",
            run_get_bad_contribution,
        ),
    ]
}

fn entry(id: &'static str, run: CaseRun) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            chapter: Chapter::Master08,
            capability: Capability::ChangeSets,
            profiles: &[Profile::Core, Profile::Standard],
            formats: &[Format::Json],
            provenance: Provenance::Schedule,
            schedule_ref: id,
            upstream_tags: &[],
            compare: Compare::Superset,
        },
        run,
    }
}

// ── fixtures + helpers ───────────────────────────────────────────────────────

fn codec(e: fixtures::FixtureError) -> CaseError {
    CaseError::Codec(e.to_string())
}

/// Load a contribution fixture (path relative to the corpus root).
fn contribution(rel: &str) -> Result<Value, CaseError> {
    fixtures::read_json(rel).map_err(codec)
}

/// Commit a CONTRIBUTION body against `ehr_id` (JSON, `return=representation`),
/// returning the raw response.
async fn commit(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    body: &Value,
) -> Result<HttpResponse, CaseError> {
    ctx.send(
        HttpRequest::post(format!("/ehr/{ehr_id}/contribution"))
            .json_body(body)?
            .header("accept", "application/json")
            .header("prefer", "return=representation"),
    )
    .await
}

/// The `OBJECT_VERSION_ID` of the version at index `i` in a committed
/// CONTRIBUTION representation (`versions[i].id.value`).
fn version_uid_at(resp: &HttpResponse, i: usize) -> Result<String, CaseError> {
    let body = resp.json()?;
    body["versions"][i]["id"]["value"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| {
            CaseError::Assertion(format!(
                "committed CONTRIBUTION has no versions[{i}].id.value (body: {body})"
            ))
        })
}

/// The CONTRIBUTION uid of a committed CONTRIBUTION (`ETag` preferred, else
/// `uid.value`).
fn contribution_uid(resp: &HttpResponse) -> Result<String, CaseError> {
    if let Some(etag) = resp.header("etag") {
        let trimmed = etag.trim_matches('"');
        if !trimmed.is_empty() {
            return Ok(trimmed.to_owned());
        }
    }
    let body = resp.json()?;
    body["uid"]["value"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| CaseError::Assertion("committed CONTRIBUTION has no uid".to_owned()))
}

/// Set `versions[i].preceding_version_uid` to `uid` (fixture adaptation §6:
/// the vendored modification/delete fixtures carry a placeholder preceding uid
/// that must be the actual predecessor version returned by the first commit).
fn set_preceding(body: &mut Value, i: usize, uid: &str) {
    body["versions"][i]["preceding_version_uid"] =
        json!({ "_type": "OBJECT_VERSION_ID", "value": uid });
}

/// The current `EHR_STATUS` version uid (`GET /ehr/{id}/ehr_status` → `uid.value`).
async fn ehr_status_uid(ctx: &RunContext<'_>, ehr_id: &str) -> Result<String, CaseError> {
    let resp = ctx
        .send(
            HttpRequest::get(format!("/ehr/{ehr_id}/ehr_status"))
                .header("accept", "application/json"),
        )
        .await?;
    assert::status(&resp, 200)?;
    support::uid_of(&resp.json()?)
}

/// A `DV_CODED_TEXT` for an `audit_change_type` group code + rubric.
fn change_type(code: &str, rubric: &str) -> Value {
    json!({
        "_type": "DV_CODED_TEXT",
        "value": rubric,
        "defining_code": {
            "_type": "CODE_PHRASE",
            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
            "code_string": code
        }
    })
}

/// Normalize `versions[i].commit_audit.change_type` to `251|modification|`.
///
/// PORT NOTE (fixture adaptation §6): the vendored `status.contribution.
/// modification` and `folder.contribution.modification` fixtures are labelled
/// `value: "modification"` but carry `defining_code.code_string = "249"`
/// (creation) — an internally-inconsistent RM-1.0.x-era fixture. The server
/// (correctly) treats `defining_code` as authoritative, so a modification must
/// carry code `251`; this corrects the code to the fixture's stated intent,
/// never fixing an *intended* defect.
fn normalize_modification(body: &mut Value, i: usize) {
    body["versions"][i]["commit_audit"]["change_type"] = change_type("251", "modification");
}

macro_rules! case {
    ($body:block) => {
        Box::pin(async move { $body })
    };
}

// ── commit_contribution: COMPOSITION ─────────────────────────────────────────

fn run_valid_composition<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        support::ensure_opt(ctx, "minimal/minimal_evaluation.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let body =
            contribution("contributions/valid/minimal/minimal_evaluation.contribution.json")?;
        let resp = commit(ctx, &ehr_id, &body).await?;
        assert::status(&resp, 201)?;
        assert::header_present(&resp, "etag")?;
        // A creation version's version number is 1.
        let uid = version_uid_at(&resp, 0)?;
        if uid.ends_with("::1") {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(format!(
                "expected version number 1 for a creation, got {uid:?}"
            )))
        }
    })
}

fn run_invalid_composition<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // A valid COMPOSITION contribution with a mandatory RM attribute removed
        // (composer) — invalid content the server must reject (schedule C.2;
        // COMPOSITION.composer is mandatory).
        support::ensure_opt(ctx, "minimal/minimal_evaluation.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let mut body =
            contribution("contributions/valid/minimal/minimal_evaluation.contribution.json")?;
        if let Some(data) = body["versions"][0]["data"].as_object_mut() {
            data.remove("composer");
        }
        let resp = commit(ctx, &ehr_id, &body).await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let body = contribution("contributions/invalid/no_versions.json")?;
        let resp = commit(ctx, &ehr_id, &body).await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_valid_invalid_compositions<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // Mixed valid + invalid VERSIONs → the whole commit fails atomically
        // (schedule C.4). The invalid version references a valid OPT, so provision
        // the minimal OPT set the valid version needs.
        support::ensure_opt(ctx, "minimal/minimal_evaluation.opt").await?;
        support::ensure_opt(ctx, "minimal/minimal_admin.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let body = contribution("contributions/invalid/multiple_valid_and_invalid_compos.json")?;
        let resp = commit(ctx, &ehr_id, &body).await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_non_existing_opt<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // The COMPOSITION references a template never uploaded → rejected
        // (schedule C.10; our validation → 422).
        let ehr_id = support::create_ehr(ctx).await?;
        let body = contribution("contributions/invalid/ref_to_non_existent_OPT.json")?;
        let resp = commit(ctx, &ehr_id, &body).await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_event_composition<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // Create an event COMPOSITION, then modify it in a second commit → v2
        // (schedule C.5). minimal_admin is category=event (433).
        support::ensure_opt(ctx, "minimal/minimal_admin.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let created = commit(
            ctx,
            &ehr_id,
            &contribution("contributions/valid/minimal/minimal_admin.contribution.json")?,
        )
        .await?;
        assert::status(&created, 201)?;
        let v1 = version_uid_at(&created, 0)?;

        let mut modify = contribution(
            "contributions/valid/minimal/minimal_admin.contribution.modification.complete.json",
        )?;
        set_preceding(&mut modify, 0, &v1);
        let updated = commit(ctx, &ehr_id, &modify).await?;
        assert::status(&updated, 201)?;
        let v2 = version_uid_at(&updated, 0)?;
        if v2.ends_with("::2") {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(format!(
                "expected version number 2 after modification, got {v2:?}"
            )))
        }
    })
}

fn run_persistent_composition<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        support::ensure_opt(ctx, "minimal_persistent/persistent_minimal.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let created = commit(
            ctx,
            &ehr_id,
            &contribution(
                "contributions/valid/minimal_persistent/minimal_persistent.contribution.json",
            )?,
        )
        .await?;
        assert::status(&created, 201)?;
        let v1 = version_uid_at(&created, 0)?;

        let mut modify = contribution(
            "contributions/valid/minimal_persistent/minimal_persistent.contribution.modification.json",
        )?;
        set_preceding(&mut modify, 0, &v1);
        let updated = commit(ctx, &ehr_id, &modify).await?;
        assert::status(&updated, 201)?;
        let v2 = version_uid_at(&updated, 0)?;
        if v2.ends_with("::2") {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(format!(
                "expected version number 2 after modification, got {v2:?}"
            )))
        }
    })
}

fn run_delete<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // Create then delete a COMPOSITION via CONTRIBUTIONs (schedule C.7). The
        // vendored `deleted.deleted` fixture carries data; a deleted VERSION's
        // data is Void (RM change_control §Contributions), so it is nulled — an
        // additive RM-shape adaptation (§6), and the preceding is patched.
        support::ensure_opt(ctx, "minimal/minimal_admin.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let created = commit(
            ctx,
            &ehr_id,
            &contribution("contributions/valid/minimal/minimal_admin.contribution.json")?,
        )
        .await?;
        assert::status(&created, 201)?;
        let v1 = version_uid_at(&created, 0)?;

        let mut delete = contribution(
            "contributions/valid/minimal/minimal_admin.contribution.deleted.deleted.json",
        )?;
        delete["versions"][0]["data"] = Value::Null;
        set_preceding(&mut delete, 0, &v1);
        let deleted = commit(ctx, &ehr_id, &delete).await?;
        assert::status(&deleted, 201)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_two_commits_second_invalid<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // Create a valid COMPOSITION, then a second commit modifying it with
        // invalid content → rejected; only one VERSION remains (schedule C.8).
        support::ensure_opt(ctx, "minimal/minimal_admin.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let created = commit(
            ctx,
            &ehr_id,
            &contribution("contributions/valid/minimal/minimal_admin.contribution.json")?,
        )
        .await?;
        assert::status(&created, 201)?;
        let v1 = version_uid_at(&created, 0)?;

        let mut modify = contribution(
            "contributions/valid/minimal/minimal_admin.contribution.modification.complete.json",
        )?;
        set_preceding(&mut modify, 0, &v1);
        if let Some(data) = modify["versions"][0]["data"].as_object_mut() {
            data.remove("composer"); // mandatory attribute → invalid content
        }
        let resp = commit(ctx, &ehr_id, &modify).await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_two_commits_second_creation<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // Create a valid COMPOSITION, then a second commit whose change_type is
        // `creation` while pointing at the existing object → rejected (schedule
        // C.9; only one 'create' is allowed, RM change_control §Contributions).
        support::ensure_opt(ctx, "minimal/minimal_admin.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let created = commit(
            ctx,
            &ehr_id,
            &contribution("contributions/valid/minimal/minimal_admin.contribution.json")?,
        )
        .await?;
        assert::status(&created, 201)?;
        let v1 = version_uid_at(&created, 0)?;

        let mut second = contribution(
            "contributions/valid/minimal/minimal_admin.contribution.modification.complete.json",
        )?;
        set_preceding(&mut second, 0, &v1);
        second["versions"][0]["commit_audit"]["change_type"] = change_type("249", "creation");
        let resp = commit(ctx, &ehr_id, &second).await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── commit_contribution: EHR_STATUS ──────────────────────────────────────────

fn run_minimal_ehr_status<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ commit_status_modification(ctx).await })
}

fn run_full_ehr_status<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    // The vendored status modification carries a full subject.external_ref; the
    // difference from `minimal` is only the precondition, which our per-case
    // fresh EHR satisfies either way.
    case!({ commit_status_modification(ctx).await })
}

/// Commit a valid `EHR_STATUS` modification CONTRIBUTION against the EHR's current
/// status version (schedule D.1/D.2) → 201.
async fn commit_status_modification(ctx: &RunContext<'_>) -> Result<DataSetReport, CaseError> {
    let ehr_id = support::create_ehr(ctx).await?;
    let current = ehr_status_uid(ctx, &ehr_id).await?;
    let mut body =
        contribution("contributions/valid/minimal/status.contribution.modification.json")?;
    normalize_modification(&mut body, 0);
    set_preceding(&mut body, 0, &current);
    let resp = commit(ctx, &ehr_id, &body).await?;
    assert::status(&resp, 201)?;
    Ok(DataSetReport::SINGLE)
}

fn run_ehr_status_invalid_change_type<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // change_type = creation on an EHR that already has an EHR_STATUS → the
        // status can't be created again (schedule D.3) → rejected.
        let ehr_id = support::create_ehr(ctx).await?;
        let mut body =
            contribution("contributions/valid/minimal/status.contribution.modification.json")?;
        if let Some(v) = body["versions"][0].as_object_mut() {
            v.remove("preceding_version_uid");
        }
        body["versions"][0]["commit_audit"]["change_type"] = change_type("249", "creation");
        let resp = commit(ctx, &ehr_id, &body).await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_invalid_ehr_status<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // A structurally valid change_type (modification against the current
        // status) but invalid EHR_STATUS content (mandatory `is_queryable`
        // removed) → rejected (schedule D.4).
        let ehr_id = support::create_ehr(ctx).await?;
        let current = ehr_status_uid(ctx, &ehr_id).await?;
        let mut body =
            contribution("contributions/valid/minimal/status.contribution.modification.json")?;
        set_preceding(&mut body, 0, &current);
        if let Some(data) = body["versions"][0]["data"].as_object_mut() {
            data.remove("is_queryable"); // mandatory EHR_STATUS attribute
        }
        let resp = commit(ctx, &ehr_id, &body).await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── commit_contribution: FOLDER (directory) ──────────────────────────────────

fn run_valid_directory<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let body = contribution("contributions/valid/minimal/folder.contribution.creation.json")?;
        let resp = commit(ctx, &ehr_id, &body).await?;
        assert::status(&resp, 201)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_fail_create_existing_directory<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let create = contribution("contributions/valid/minimal/folder.contribution.creation.json")?;
        let first = commit(ctx, &ehr_id, &create).await?;
        assert::status(&first, 201)?;
        // A second directory creation conflicts (schedule E.2).
        let second = commit(ctx, &ehr_id, &create).await?;
        support::assert_negative(&second)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_fail_modify_non_existing_directory<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // Modify a directory that doesn't exist (schedule E.3) → rejected.
        let ehr_id = support::create_ehr(ctx).await?;
        let mut body =
            contribution("contributions/valid/minimal/folder.contribution.modification.json")?;
        normalize_modification(&mut body, 0);
        set_preceding(&mut body, 0, &format!("{}::conformance::1", Uuid::new_v4()));
        let resp = commit(ctx, &ehr_id, &body).await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_update_existing_directory<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // Create a directory, then update it via a second CONTRIBUTION (schedule
        // E.4) → 201.
        let ehr_id = support::create_ehr(ctx).await?;
        let create = contribution("contributions/valid/minimal/folder.contribution.creation.json")?;
        let created = commit(ctx, &ehr_id, &create).await?;
        assert::status(&created, 201)?;
        let v1 = version_uid_at(&created, 0)?;

        let mut modify =
            contribution("contributions/valid/minimal/folder.contribution.modification.json")?;
        normalize_modification(&mut modify, 0);
        set_preceding(&mut modify, 0, &v1);
        let updated = commit(ctx, &ehr_id, &modify).await?;
        assert::status(&updated, 201)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── get_contribution ─────────────────────────────────────────────────────────

fn run_get_existing<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        support::ensure_opt(ctx, "minimal/minimal_admin.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let created = commit(
            ctx,
            &ehr_id,
            &contribution("contributions/valid/minimal/minimal_admin.contribution.json")?,
        )
        .await?;
        assert::status(&created, 201)?;
        let uid = contribution_uid(&created)?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/contribution/{uid}"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        let body = resp.json()?;
        if body["_type"] == "CONTRIBUTION" {
            Ok(DataSetReport::SINGLE)
        } else {
            Err(CaseError::Assertion(format!(
                "expected CONTRIBUTION, got {}",
                body["_type"]
            )))
        }
    })
}

fn run_get_empty_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/contribution/{}", Uuid::new_v4()))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr/{}/contribution/{}",
                    Uuid::new_v4(),
                    Uuid::new_v4()
                ))
                .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_bad_contribution<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // EHR with a real CONTRIBUTION, but a random contribution uid → 404.
        support::ensure_opt(ctx, "minimal/minimal_admin.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let created = commit(
            ctx,
            &ehr_id,
            &contribution("contributions/valid/minimal/minimal_admin.contribution.json")?,
        )
        .await?;
        assert::status(&created, 201)?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/contribution/{}", Uuid::new_v4()))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}
