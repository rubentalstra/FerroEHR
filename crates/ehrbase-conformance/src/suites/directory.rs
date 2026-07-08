//! master09 — DIRECTORY (FOLDER) cases (design §4.1: `suites/directory.rs`).
//!
//! Transcribed from `master09-func_tc_ehr_directory.adoc`, driving the ITS-REST
//! `/ehr/{ehr_id}/directory` surface with the vendored `directory/` FOLDER
//! fixtures (clean canonical JSON, no adaptation needed). Assertions concretize
//! the directory status contract (`201` create; `200` get/update; `204` delete;
//! `404` absent EHR/directory; `409` create-when-present).
//!
//! The `has_*`, `*_at_version` and `get_versioned_directory` schedule cases have
//! no dedicated ITS-REST endpoint on our surface and stay `NotYetTranscribed`.

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
