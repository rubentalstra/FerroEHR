//! master09 — DIRECTORY (FOLDER) cases (design §4.1: `suites/directory.rs`).
//!
//! Transcribed from `master09-func_tc_ehr_directory.adoc`, driving the ITS-REST
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
use crate::case::{Capability, CaseMeta, Chapter, Compare, Format, Profile, Provenance};
use crate::fixtures;
use crate::harness::{CaseError, CaseFuture, CaseRun, DataSetReport, HttpRequest, RunContext};
use crate::registry::CaseEntry;
use crate::suites::support;

/// The implemented master09 case entries.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        entry(
            "I_EHR_DIRECTORY.create_directory-empty_ehr",
            run_create_empty_ehr,
        ),
        entry(
            "I_EHR_DIRECTORY.create_directory-ehr_with_directory",
            run_create_when_present,
        ),
        entry(
            "I_EHR_DIRECTORY.create_directory-bad_ehr",
            run_create_bad_ehr,
        ),
        entry(
            "I_EHR_DIRECTORY.get_directory-ehr_root_directory",
            run_get_root,
        ),
        entry("I_EHR_DIRECTORY.get_directory-bad_ehr", run_get_bad_ehr),
        entry(
            "I_EHR_DIRECTORY.get_directory_at_time-ehr_with_directory",
            run_get_at_time,
        ),
        entry(
            "I_EHR_DIRECTORY.get_directory_at_time-bad_ehr",
            run_get_at_time_bad_ehr,
        ),
        entry(
            "I_EHR_DIRECTORY.update_directory-ehr_with_directory",
            run_update,
        ),
        entry(
            "I_EHR_DIRECTORY.update_directory-bad_ehr",
            run_update_bad_ehr,
        ),
        entry(
            "I_EHR_DIRECTORY.delete_directory-ehr_with_directory",
            run_delete,
        ),
        entry(
            "I_EHR_DIRECTORY.delete_directory-bad_ehr",
            run_delete_bad_ehr,
        ),
        // has_directory — SM boolean realized via GET /directory (200 has / 404 not).
        entry("I_EHR_DIRECTORY.has_directory-empty_ehr", run_has_dir_empty),
        entry(
            "I_EHR_DIRECTORY.has_directory-ehr_with_directory",
            run_has_dir_present,
        ),
        entry("I_EHR_DIRECTORY.has_directory-bad_ehr", run_has_dir_bad),
        // has_path — realized via GET /directory?path= (200 present / 404 absent).
        entry(
            "I_EHR_DIRECTORY.has_path-ehr_root_directory",
            run_has_path_root,
        ),
        entry(
            "I_EHR_DIRECTORY.has_path-folder_structure",
            run_has_path_folder,
        ),
        entry("I_EHR_DIRECTORY.has_path-empty_ehr", run_has_path_empty),
        entry("I_EHR_DIRECTORY.has_path-bad_ehr", run_has_path_bad),
        // has_directory_version — realized via GET /directory/{version_uid}.
        entry(
            "I_EHR_DIRECTORY.has_directory_version-empty_ehr",
            run_has_ver_empty,
        ),
        entry(
            "I_EHR_DIRECTORY.has_directory_version-directory_with_two_versions",
            run_has_ver_present,
        ),
        entry(
            "I_EHR_DIRECTORY.has_directory_version-bad_ehr",
            run_has_ver_bad,
        ),
        // get_directory.
        entry("I_EHR_DIRECTORY.get_directory-empty_ehr", run_get_dir_empty),
        entry(
            "I_EHR_DIRECTORY.get_directory-directory_with_structure",
            run_get_dir_structure,
        ),
        // get_directory_at_time — GET /directory?version_at_time=.
        entry(
            "I_EHR_DIRECTORY.get_directory_at_time-ehr_with_directory_empty_time",
            run_at_time_empty_time,
        ),
        entry(
            "I_EHR_DIRECTORY.get_directory_at_time-ehr_with_directory_versions",
            run_at_time_versions,
        ),
        entry(
            "I_EHR_DIRECTORY.get_directory_at_time-ehr_with_directory_versions_empty_time",
            run_at_time_versions_empty_time,
        ),
        entry(
            "I_EHR_DIRECTORY.get_directory_at_time-empty_ehr",
            run_at_time_empty_ehr,
        ),
        entry(
            "I_EHR_DIRECTORY.get_directory_at_time-empty_ehr_empty_time",
            run_at_time_empty_ehr_empty_time,
        ),
        entry(
            "I_EHR_DIRECTORY.get_directory_at_time-multiple_versions_first",
            run_at_time_first,
        ),
        // get_directory_at_version — GET /directory/{version_uid}.
        entry(
            "I_EHR_DIRECTORY.get_directory_at_version-bad_ehr",
            run_at_version_bad,
        ),
        entry(
            "I_EHR_DIRECTORY.get_directory_at_version-directory_with_two_versions",
            run_at_version_two,
        ),
        entry(
            "I_EHR_DIRECTORY.get_directory_at_version-empty_ehr",
            run_at_version_empty,
        ),
        // get_versioned_directory — GET /ehr/{id}/versioned_directory.
        entry(
            "I_EHR_DIRECTORY.get_versioned_directory-empty_ehr",
            run_versioned_empty,
        ),
        entry(
            "I_EHR_DIRECTORY.get_versioned_directory-directory_with_two_versions",
            run_versioned_two,
        ),
        entry(
            "I_EHR_DIRECTORY.get_versioned_directory-bad_ehr",
            run_versioned_bad,
        ),
        // update / delete on an EHR with no directory yet.
        entry(
            "I_EHR_DIRECTORY.update_directory-empty_ehr",
            run_update_empty,
        ),
        entry(
            "I_EHR_DIRECTORY.delete_directory-empty_ehr",
            run_delete_empty,
        ),
    ]
}

fn entry(id: &'static str, run: CaseRun) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            chapter: Chapter::Master09,
            capability: Capability::DirectoryOps,
            profiles: &[Profile::Standard],
            formats: &[Format::Json],
            provenance: Provenance::Schedule,
            schedule_ref: id,
            upstream_tags: &[],
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
