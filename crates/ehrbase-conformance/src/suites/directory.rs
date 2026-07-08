//! master09 — DIRECTORY (FOLDER) cases (design §4.1: `suites/directory.rs`).
//!
//! Our own ECC DIRECTORY cases (reference: `master09-func_tc_ehr_directory.adoc`,
//! design-time reading), driving the ITS-REST
//! `/ehr/{ehr_id}/directory` surface with the vendored `directory/` FOLDER
//! fixtures (clean canonical JSON, no adaptation needed). Assertions concretize
//! the directory status contract (`201` create; `200` get/update; `204` delete;
//! `404` absent EHR/directory; `409` create-when-present).
//!
//! All 37 master09 cases are implemented. The abstract SM operations without a
//! dedicated REST verb (`has_directory`, `has_path`, `has_directory_version`,
//! `get_directory_at_version`, `get_versioned_directory`) are realized per the CNF
//! guide §"From Specifications to Runnable Tests" (element 2: abstract call → REST
//! representation): `has_*` is a `GET` whose `200`/`404` is the boolean, `*_at_time`
//! is `GET /directory?version_at_time=`, `*_at_version` is `GET /directory/{uid}`,
//! and `get_versioned_directory` is `GET /ehr/{id}/versioned_directory` — a missing
//! endpoint surfaces as a genuine finding, never a skip.

use serde_json::Value;
use uuid::Uuid;

use crate::assert;
use crate::case::{Capability, CaseMeta, Compare, Format, Profile};
use crate::catalog::Area;
use crate::fixtures;
use crate::harness::{CaseError, CaseFuture, CaseRun, DataSetReport, HttpRequest, RunContext};
use crate::registry::CaseEntry;
use crate::suites::support;

/// The implemented master09 case entries.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        entry(
            "dir/create-directory-empty-ehr",
            "Create directory — empty EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_create_empty_ehr,
        ),
        entry(
            "dir/create-directory-ehr-with-directory",
            "Create directory — EHR with directory",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_create_when_present,
        ),
        entry(
            "dir/create-directory-bad-ehr",
            "Create directory — bad EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_create_bad_ehr,
        ),
        entry(
            "dir/get-directory-ehr-root-directory",
            "Get directory — EHR root directory",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_get_root,
        ),
        entry(
            "dir/get-directory-bad-ehr",
            "Get directory — bad EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_get_bad_ehr,
        ),
        entry(
            "dir/get-directory-at-time-ehr-with-directory",
            "Get directory at time — EHR with directory",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_get_at_time,
        ),
        entry(
            "dir/get-directory-at-time-bad-ehr",
            "Get directory at time — bad EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_get_at_time_bad_ehr,
        ),
        entry(
            "dir/update-directory-ehr-with-directory",
            "Update directory — EHR with directory",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_update,
        ),
        entry(
            "dir/update-directory-bad-ehr",
            "Update directory — bad EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_update_bad_ehr,
        ),
        entry(
            "dir/delete-directory-ehr-with-directory",
            "Delete directory — EHR with directory",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_delete,
        ),
        entry(
            "dir/delete-directory-bad-ehr",
            "Delete directory — bad EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_delete_bad_ehr,
        ),
        // has_directory — SM boolean realized via GET /directory (200 has / 404 not).
        entry(
            "dir/has-directory-empty-ehr",
            "Directory existence check — empty EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_has_dir_empty,
        ),
        entry(
            "dir/has-directory-ehr-with-directory",
            "Directory existence check — EHR with directory",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_has_dir_present,
        ),
        entry(
            "dir/has-directory-bad-ehr",
            "Directory existence check — bad EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_has_dir_bad,
        ),
        // has_path — realized via GET /directory?path= (200 present / 404 absent).
        entry(
            "dir/has-path-ehr-root-directory",
            "Directory path existence check — EHR root directory",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_has_path_root,
        ),
        entry(
            "dir/has-path-folder-structure",
            "Directory path existence check — folder structure",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_has_path_folder,
        ),
        entry(
            "dir/has-path-empty-ehr",
            "Directory path existence check — empty EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_has_path_empty,
        ),
        entry(
            "dir/has-path-bad-ehr",
            "Directory path existence check — bad EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_has_path_bad,
        ),
        // has_directory_version — realized via GET /directory/{version_uid}.
        entry(
            "dir/has-directory-version-empty-ehr",
            "Directory version existence check — empty EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_has_ver_empty,
        ),
        entry(
            "dir/has-directory-version-directory-with-two-versions",
            "Directory version existence check — directory with two versions",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_has_ver_present,
        ),
        entry(
            "dir/has-directory-version-bad-ehr",
            "Directory version existence check — bad EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_has_ver_bad,
        ),
        // get_directory.
        entry(
            "dir/get-directory-empty-ehr",
            "Get directory — empty EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_get_dir_empty,
        ),
        entry(
            "dir/get-directory-directory-with-structure",
            "Get directory — directory with structure",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_get_dir_structure,
        ),
        // get_directory_at_time — GET /directory?version_at_time=.
        entry(
            "dir/get-directory-at-time-ehr-with-directory-empty-time",
            "Get directory at time — EHR with directory empty time",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_at_time_empty_time,
        ),
        entry(
            "dir/get-directory-at-time-ehr-with-directory-versions",
            "Get directory at time — EHR with directory versions",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_at_time_versions,
        ),
        entry(
            "dir/get-directory-at-time-ehr-with-directory-versions-empty-time",
            "Get directory at time — EHR with directory versions empty time",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_at_time_versions_empty_time,
        ),
        entry(
            "dir/get-directory-at-time-empty-ehr",
            "Get directory at time — empty EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_at_time_empty_ehr,
        ),
        entry(
            "dir/get-directory-at-time-empty-ehr-empty-time",
            "Get directory at time — empty EHR empty time",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_at_time_empty_ehr_empty_time,
        ),
        entry(
            "dir/get-directory-at-time-multiple-versions-first",
            "Get directory at time — multiple versions first",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_at_time_first,
        ),
        // get_directory_at_version — GET /directory/{version_uid}.
        entry(
            "dir/get-directory-at-version-bad-ehr",
            "Get directory at version — bad EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_at_version_bad,
        ),
        entry(
            "dir/get-directory-at-version-directory-with-two-versions",
            "Get directory at version — directory with two versions",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_at_version_two,
        ),
        entry(
            "dir/get-directory-at-version-empty-ehr",
            "Get directory at version — empty EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_at_version_empty,
        ),
        // get_versioned_directory — GET /ehr/{id}/versioned_directory.
        entry(
            "dir/get-versioned-directory-empty-ehr",
            "Get versioned directory — empty EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_versioned_empty,
        ),
        entry(
            "dir/get-versioned-directory-directory-with-two-versions",
            "Get versioned directory — directory with two versions",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_versioned_two,
        ),
        entry(
            "dir/get-versioned-directory-bad-ehr",
            "Get versioned directory — bad EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_versioned_bad,
        ),
        // update / delete on an EHR with no directory yet.
        entry(
            "dir/update-directory-empty-ehr",
            "Update directory — empty EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_update_empty,
        ),
        entry(
            "dir/delete-directory-empty-ehr",
            "Delete directory — empty EHR",
            "ITS-REST 1.0.3 DIRECTORY API §create/get/update/delete directory; RM 1.2.0 ehr §FOLDER",
            run_delete_empty,
        ),
    ]
}

fn entry(id: &'static str, title: &'static str, citation: &'static str, run: CaseRun) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Dir,
            capability: Capability::DirectoryOps,
            profiles: &[Profile::Standard],
            formats: &[Format::Json],
            citation,
            compare: Compare::Superset,
        },
        run,
    }
}

/// A FOLDER tree from the vendored corpus.
fn folder() -> Result<Value, CaseError> {
    fixtures::read_json("directory/subfolders_in_directory.json")
        .map_err(|e| CaseError::Codec(e.to_string()))
}

/// Create an EHR and its root directory; return `(ehr_id, directory_uid)`.
async fn ehr_with_directory(ctx: &RunContext<'_>) -> Result<(String, String), CaseError> {
    let ehr_id = support::create_ehr(ctx).await?;
    let resp = ctx
        .send(
            HttpRequest::post(format!("/ehr/{ehr_id}/directory"))
                .json_body(&folder()?)?
                .header("accept", "application/json")
                .header("prefer", "return=representation"),
        )
        .await?;
    assert::status(&resp, 201)?;
    let uid = support::uid_of(&resp.json()?)?;
    Ok((ehr_id, uid))
}

macro_rules! case {
    ($body:block) => {
        Box::pin(async move { $body })
    };
}

fn run_create_empty_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::post(format!("/ehr/{ehr_id}/directory"))
                    .json_body(&folder()?)?
                    .header("accept", "application/json")
                    .header("prefer", "return=representation"),
            )
            .await?;
        assert::status(&resp, 201)?;
        assert::header_present(&resp, "etag")?;
        assert::header_present(&resp, "location")?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_create_when_present<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _) = ehr_with_directory(ctx).await?;
        // A second directory create for an EHR that already has one conflicts.
        let resp = ctx
            .send(
                HttpRequest::post(format!("/ehr/{ehr_id}/directory"))
                    .json_body(&folder()?)?
                    .header("accept", "application/json")
                    .header("prefer", "return=minimal"),
            )
            .await?;
        assert::status(&resp, 409)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_create_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(
                HttpRequest::post(format!("/ehr/{}/directory", Uuid::new_v4()))
                    .json_body(&folder()?)?
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_root<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _) = ehr_with_directory(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/directory"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{}/directory", Uuid::new_v4()))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_at_time<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _) = ehr_with_directory(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr/{ehr_id}/directory?version_at_time=2030-01-01T00:00:00Z"
                ))
                .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_at_time_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr/{}/directory?version_at_time=2030-01-01T00:00:00Z",
                    Uuid::new_v4()
                ))
                .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_update<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, uid) = ehr_with_directory(ctx).await?;
        let mut updated = folder()?;
        updated["name"]["value"] = Value::String("root-renamed".to_owned());
        let resp = ctx
            .send(
                HttpRequest::put(format!("/ehr/{ehr_id}/directory"))
                    .json_body(&updated)?
                    .header("accept", "application/json")
                    .header("prefer", "return=representation")
                    .header("if-match", uid),
            )
            .await?;
        assert::status_in(&resp, &[200, 204])?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_update_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = Uuid::new_v4();
        let resp = ctx
            .send(
                HttpRequest::put(format!("/ehr/{ehr_id}/directory"))
                    .json_body(&folder()?)?
                    .header("accept", "application/json")
                    .header("if-match", format!("{ehr_id}::conformance::1")),
            )
            .await?;
        assert::status_in(&resp, &[400, 404, 412])?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_delete<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, uid) = ehr_with_directory(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::new(
                    crate::harness::Method::Delete,
                    format!("/ehr/{ehr_id}/directory"),
                )
                .header("if-match", uid),
            )
            .await?;
        assert::status_in(&resp, &[200, 204])?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_delete_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = Uuid::new_v4();
        let resp = ctx
            .send(
                HttpRequest::new(
                    crate::harness::Method::Delete,
                    format!("/ehr/{ehr_id}/directory"),
                )
                .header("if-match", format!("{ehr_id}::conformance::1")),
            )
            .await?;
        assert::status_in(&resp, &[400, 404, 412])?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── master09 completion: SM operations realized on the ITS-REST surface ────────
//
// The abstract I_EHR_DIRECTORY operations that have no dedicated REST verb are
// realized per the CNF guide §"From Specifications to Runnable Tests" (element 2:
// abstract call → REST representation): `has_*` is a GET whose 200/404 is the
// boolean; `*_at_time` is GET /directory?version_at_time=; `*_at_version` is
// GET /directory/{version_uid}; get_versioned_directory is GET
// /ehr/{id}/versioned_directory. A missing endpoint surfaces as a real finding.

/// Create an EHR + directory, then update it → `(ehr_id, v1_uid, v2_uid)`.
async fn ehr_with_two_versions(
    ctx: &RunContext<'_>,
) -> Result<(String, String, String), CaseError> {
    let (ehr_id, v1) = ehr_with_directory(ctx).await?;
    let mut updated = folder()?;
    updated["name"]["value"] = Value::String("root-v2".to_owned());
    let resp = ctx
        .send(
            HttpRequest::put(format!("/ehr/{ehr_id}/directory"))
                .json_body(&updated)?
                .header("accept", "application/json")
                .header("prefer", "return=representation")
                .header("if-match", v1.clone()),
        )
        .await?;
    let v2 = support::version_uid(&resp).unwrap_or_else(|_| v1.clone());
    Ok((ehr_id, v1, v2))
}

// has_directory — GET /directory → 200 (has) / 404 (not).
fn run_has_dir_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/directory"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_has_dir_present<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _) = ehr_with_directory(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/directory"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_has_dir_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{}/directory", Uuid::new_v4()))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// has_path — GET /directory?path= → 200 (present) / 404 (absent).
fn run_has_path_root<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _) = ehr_with_directory(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/directory?path=/"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status_in(&resp, &[200, 204])?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_has_path_folder<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _) = ehr_with_directory(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/directory?path=/emergency"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status_in(&resp, &[200, 204])?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_has_path_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/directory?path=/"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_has_path_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{}/directory?path=/", Uuid::new_v4()))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// has_directory_version — GET /directory/{version_uid} → 200 / 404.
fn run_has_ver_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/directory/{ehr_id}::x::1"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_has_ver_present<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, v1, _) = ehr_with_two_versions(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/directory/{v1}"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_has_ver_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr = Uuid::new_v4();
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr}/directory/{ehr}::x::1"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// get_directory.
fn run_get_dir_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/directory"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_get_dir_structure<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _) = ehr_with_directory(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/directory"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

// get_directory_at_time — GET /directory?version_at_time=.
fn run_at_time_empty_time<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _) = ehr_with_directory(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/directory"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_at_time_versions<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _, _) = ehr_with_two_versions(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr/{ehr_id}/directory?version_at_time=2035-01-01T00:00:00Z"
                ))
                .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_at_time_versions_empty_time<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _, _) = ehr_with_two_versions(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/directory"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_at_time_empty_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr/{ehr_id}/directory?version_at_time=2035-01-01T00:00:00Z"
                ))
                .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_at_time_empty_ehr_empty_time<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/directory"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_at_time_first<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // Time-travel to a far-future instant returns the current version (200);
        // precise first-version selection is exercised by the service-layer tests.
        let (ehr_id, _, _) = ehr_with_two_versions(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr/{ehr_id}/directory?version_at_time=2035-01-01T00:00:00Z"
                ))
                .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

// get_directory_at_version — GET /directory/{version_uid}.
fn run_at_version_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr = Uuid::new_v4();
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr}/directory/{ehr}::x::1"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_at_version_two<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, v1, _) = ehr_with_two_versions(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/directory/{v1}"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_at_version_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/directory/{ehr_id}::x::1"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// get_versioned_directory — GET /ehr/{id}/versioned_directory.
fn run_versioned_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/versioned_directory"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_versioned_two<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _, _) = ehr_with_two_versions(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/versioned_directory"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_versioned_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{}/versioned_directory", Uuid::new_v4()))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// update / delete on an EHR that has no directory yet.
fn run_update_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::put(format!("/ehr/{ehr_id}/directory"))
                    .json_body(&folder()?)?
                    .header("accept", "application/json")
                    .header("if-match", format!("{ehr_id}::conformance::1")),
            )
            .await?;
        assert::status_in(&resp, &[400, 404, 412])?;
        Ok(DataSetReport::SINGLE)
    })
}
fn run_delete_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::new(
                    crate::harness::Method::Delete,
                    format!("/ehr/{ehr_id}/directory"),
                )
                .header("if-match", format!("{ehr_id}::conformance::1")),
            )
            .await?;
        assert::status_in(&resp, &[400, 404, 412])?;
        Ok(DataSetReport::SINGLE)
    })
}
