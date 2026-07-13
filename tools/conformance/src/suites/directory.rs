//! DIRECTORY (FOLDER) cases (`I_EHR_DIRECTORY`), authored spine-first from the
//! CNF platform test schedule `master09-func_tc_ehr_directory.adoc` and the
//! vendored ITS-REST contract (`ehr-codegen.openapi.yaml` operations
//! `directory_create` / `directory_update` / `directory_delete` /
//! `directory_get_at_time` / `directory_get_by_version_id`).
//!
//! The abstract SM operations without a dedicated REST verb are realized per the
//! CNF guide §From Specifications to Runnable Tests (element 2): `has_*` is a
//! `GET` whose 200/404 is the boolean; `*_at_time` is `GET
//! /directory?version_at_time=`; `*_at_version` and `get_versioned_directory`
//! are `GET /directory/{version_uid}` (the tested OAS exposes no
//! `versioned_directory` resource — verified against `ehr-codegen.openapi.yaml`;
//! register 06 G-3). Wire ids come only from [`crate::wire::ids`]; there are no
//! silent id fallbacks (register 06 G-4). If-Match negatives derive a
//! syntactically valid but nonexistent `OBJECT_VERSION_ID` from an OBSERVED id
//! ([`crate::suites::support::nonexistent_version_like`]) — never a system-id
//! literal.
//!
//! `get_directory_at_time` selection (G.5/G.8) is exercised for real: v1 and v2
//! are created with a captured instant strictly between them, and the
//! between-instant query MUST return v1 (temporal selection is normative-
//! invariant; the timestamp format is RFC3339 UTC — register 06 G-1).

use std::time::Duration;

use serde_json::Value;
use uuid::Uuid;

use crate::edition::Edition;
use crate::engine::assert;
use crate::engine::harness::{
    CaseError, CaseFuture, CaseRun, DataSetReport, HttpRequest, HttpResponse, RunContext,
};
use crate::engine::registry::CaseEntry;
use crate::model::case::{Binding, Capability, CaseMeta, Compare, Format, ScheduleTrace};
use crate::model::catalog::Area;
use crate::suites::support;
use crate::testdata::fixtures;
use crate::wire::{ids, negotiate};

/// JSON-only formats (master09 §Test Data Sets; `directory.rs` entry format).
const JSON: &[Format] = &[Format::Json];

const B_DIR: &str = "GET /ehr/{ehr_id}/directory";
const B_CREATE: &str = "POST /ehr/{ehr_id}/directory";
const B_UPDATE: &str = "PUT /ehr/{ehr_id}/directory";
const B_DELETE: &str = "DELETE /ehr/{ehr_id}/directory";
const B_VER: &str = "GET /ehr/{ehr_id}/directory/{version_uid}";

/// The root-FOLDER name each directory version carries, so a version read can be
/// distinguished by name (the temporal + at-version selection assertions).
const V1_NAME: &str = "conformance-dir-v1";
const V2_NAME: &str = "conformance-dir-v2";

/// Absent-directory ladder for update/delete: non-existent directory → 404
/// (`directory_update`/`directory_delete` `404_unknown_ehr_id`); 412 the
/// precondition-failed form some editions return (register 06 G-7).
const ABSENT_RUNGS: &[(Edition, u16)] = &[(Edition::Development, 404), (Edition::Release103, 412)];

/// The registered master09 DIRECTORY cases.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "the registered ECC case table is inherently enumerative"
)]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // ── has_directory (master09 §C) — 200 has / 404 not; the false/error
        //    trichotomy collapses to 200/404 by element-2 mapping (register 06
        //    G-5); the native ehrbase-sm surface keeps the distinction.
        entry(
            "dir/has-directory-empty-ehr",
            "Directory existence check — empty EHR",
            "I_EHR_DIRECTORY.has_directory-empty_ehr (master09 §has_directory)",
            B_DIR,
            "master09 §has_directory empty_ehr (→false); realized 404 via directory_get_at_time",
            Compare::None,
            run_has_dir_empty,
        ),
        entry(
            "dir/has-directory-ehr-with-directory",
            "Directory existence check — EHR with directory",
            "I_EHR_DIRECTORY.has_directory-ehr_with_directory (master09 §has_directory)",
            B_DIR,
            "master09 §has_directory ehr_with_directory (→true); realized 200 via directory_get_at_time",
            Compare::None,
            run_has_dir_present,
        ),
        entry(
            "dir/has-directory-bad-ehr",
            "Directory existence check — bad EHR",
            "I_EHR_DIRECTORY.has_directory-bad_ehr (master09 §has_directory)",
            B_DIR,
            "master09 §has_directory bad_ehr (→error); realized 404",
            Compare::None,
            run_has_dir_bad,
        ),
        // ── has_path (master09 §D) — assert BOTH branches (register 06 G-2) ────
        entry(
            "dir/has-path-ehr-root-directory",
            "Directory path existence check — EHR root directory",
            "I_EHR_DIRECTORY.has_path-ehr_root_directory (master09 §has_path)",
            B_DIR,
            "master09 §has_path ehr_root_directory data set {'/'→true, random→false}",
            Compare::None,
            run_has_path_root,
        ),
        entry(
            "dir/has-path-folder-structure",
            "Directory path existence check — folder structure",
            "I_EHR_DIRECTORY.has_path-folder_structure (master09 §has_path)",
            B_DIR,
            "master09 §has_path folder_structure 12-row path table over the reference tree (true + →false rows)",
            Compare::None,
            run_has_path_folder,
        ),
        entry(
            "dir/has-path-empty-ehr",
            "Directory path existence check — empty EHR",
            "I_EHR_DIRECTORY.has_path-empty_ehr (master09 §has_path)",
            B_DIR,
            "master09 §has_path empty_ehr (→false); realized 404",
            Compare::None,
            run_has_path_empty,
        ),
        entry(
            "dir/has-path-bad-ehr",
            "Directory path existence check — bad EHR",
            "I_EHR_DIRECTORY.has_path-bad_ehr (master09 §has_path)",
            B_DIR,
            "master09 §has_path bad_ehr (→error); realized 404",
            Compare::None,
            run_has_path_bad,
        ),
        // ── create_directory (master09 §E) ────────────────────────────────────
        entry(
            "dir/create-directory-empty-ehr",
            "Create directory — empty EHR",
            "I_EHR_DIRECTORY.create_directory-empty_ehr (master09 §create_directory)",
            B_CREATE,
            "master09 §create_directory empty_ehr; ITS-REST directory_create 201_directory + ETag + Location",
            Compare::None,
            run_create_empty,
        ),
        entry(
            "dir/create-directory-ehr-with-directory",
            "Create directory — EHR with directory",
            "I_EHR_DIRECTORY.create_directory-ehr_with_directory (master09 §create_directory)",
            B_CREATE,
            "master09 §create_directory ehr_with_directory (already exists → conflict); ITS-REST 409",
            Compare::None,
            run_create_when_present,
        ),
        entry(
            "dir/create-directory-bad-ehr",
            "Create directory — bad EHR",
            "I_EHR_DIRECTORY.create_directory-bad_ehr (master09 §create_directory)",
            B_CREATE,
            "master09 §create_directory bad_ehr; ITS-REST directory_create 404_unknown_ehr_id",
            Compare::None,
            run_create_bad_ehr,
        ),
        // ── get_directory (master09 §F) ───────────────────────────────────────
        entry(
            "dir/get-directory-empty-ehr",
            "Get directory — empty EHR",
            "I_EHR_DIRECTORY.get_directory-empty_ehr (master09 §get_directory)",
            B_DIR,
            "master09 §get_directory empty_ehr (NOTE: REST 4xx); ITS-REST directory_get_at_time 404",
            Compare::None,
            run_get_dir_empty,
        ),
        entry(
            "dir/get-directory-ehr-root-directory",
            "Get directory — EHR root directory",
            "I_EHR_DIRECTORY.get_directory-ehr_root_directory (master09 §get_directory)",
            B_DIR,
            "master09 §get_directory ehr_root_directory; ITS-REST directory_get_at_time 200_FOLDER_retrieved",
            Compare::None,
            run_get_root,
        ),
        entry(
            "dir/get-directory-directory-with-structure",
            "Get directory — directory with structure",
            "I_EHR_DIRECTORY.get_directory-directory_with_structure (master09 §get_directory)",
            B_DIR,
            "master09 §get_directory directory_with_structure (return the full structure); RM ehr master04 §Folders — body fidelity (register 06 G-6)",
            Compare::Superset,
            run_get_dir_structure,
        ),
        entry(
            "dir/get-directory-bad-ehr",
            "Get directory — bad EHR",
            "I_EHR_DIRECTORY.get_directory-bad_ehr (master09 §get_directory)",
            B_DIR,
            "master09 §get_directory bad_ehr (→error); realized 404",
            Compare::None,
            run_get_bad_ehr,
        ),
        // ── get_directory_at_time (master09 §G) ───────────────────────────────
        entry(
            "dir/get-directory-at-time-ehr-with-directory",
            "Get directory at time — EHR with directory",
            "I_EHR_DIRECTORY.get_directory_at_time-ehr_with_directory (master09 §get_directory_at_time)",
            B_DIR,
            "master09 §get_directory_at_time ehr_with_directory (current time → current)",
            Compare::None,
            run_at_time_current,
        ),
        entry(
            "dir/get-directory-at-time-bad-ehr",
            "Get directory at time — bad EHR",
            "I_EHR_DIRECTORY.get_directory_at_time-bad_ehr (master09 §get_directory_at_time)",
            B_DIR,
            "master09 §get_directory_at_time bad_ehr (→error); realized 404",
            Compare::None,
            run_at_time_bad_ehr,
        ),
        entry(
            "dir/update-directory-ehr-with-directory",
            "Update directory — EHR with directory",
            "I_EHR_DIRECTORY.update_directory-ehr_with_directory (master09 §update_directory)",
            B_UPDATE,
            "master09 §update_directory ehr_with_directory; ITS-REST directory_update 200/204 (If-Match = OBJECT_VERSION_ID)",
            Compare::None,
            run_update,
        ),
        entry(
            "dir/update-directory-bad-ehr",
            "Update directory — bad EHR",
            "I_EHR_DIRECTORY.update_directory-bad_ehr (master09 §update_directory)",
            B_UPDATE,
            "master09 §update_directory bad_ehr (→error); ITS-REST directory_update 404_unknown_ehr_id",
            Compare::None,
            run_update_bad_ehr,
        ),
        entry(
            "dir/delete-directory-ehr-with-directory",
            "Delete directory — EHR with directory",
            "I_EHR_DIRECTORY.delete_directory-ehr_with_directory (master09 §delete_directory)",
            B_DELETE,
            "master09 §delete_directory ehr_with_directory (new deleted version); ITS-REST directory_delete 204_deleted; RM common master06 §Change control (logical delete)",
            Compare::None,
            run_delete,
        ),
        entry(
            "dir/delete-directory-bad-ehr",
            "Delete directory — bad EHR",
            "I_EHR_DIRECTORY.delete_directory-bad_ehr (master09 §delete_directory)",
            B_DELETE,
            "master09 §delete_directory bad_ehr (→error); ITS-REST directory_delete 404_unknown_ehr_id",
            Compare::None,
            run_delete_bad_ehr,
        ),
        // ── has_directory (rest of §C data sets) via has_path/get combined above.
        // ── has_directory_version (master09 §J) ───────────────────────────────
        entry(
            "dir/has-directory-version-empty-ehr",
            "Directory version existence check — empty EHR",
            "I_EHR_DIRECTORY.has_directory_version-empty_ehr (master09 §has_directory_version)",
            B_VER,
            "master09 §has_directory_version empty_ehr (→false); realized 404",
            Compare::None,
            run_has_ver_empty,
        ),
        entry(
            "dir/has-directory-version-directory-with-two-versions",
            "Directory version existence check — directory with two versions",
            "I_EHR_DIRECTORY.has_directory_version-directory_with_two_versions (master09 §has_directory_version)",
            B_VER,
            "master09 §has_directory_version directory_with_two_versions (BOTH versions →true; register 06 G-2)",
            Compare::None,
            run_has_ver_present,
        ),
        entry(
            "dir/has-directory-version-bad-ehr",
            "Directory version existence check — bad EHR",
            "I_EHR_DIRECTORY.has_directory_version-bad_ehr (master09 §has_directory_version)",
            B_VER,
            "master09 §has_directory_version bad_ehr (→error); realized 404",
            Compare::None,
            run_has_ver_bad,
        ),
        // ── get_directory_at_time (rest of §G) ────────────────────────────────
        entry(
            "dir/get-directory-at-time-ehr-with-directory-empty-time",
            "Get directory at time — EHR with directory empty time",
            "I_EHR_DIRECTORY.get_directory_at_time-ehr_with_directory_empty_time (master09 §get_directory_at_time)",
            B_DIR,
            "master09 §get_directory_at_time ehr_with_directory_empty_time (→current)",
            Compare::None,
            run_at_time_empty_time,
        ),
        entry(
            "dir/get-directory-at-time-ehr-with-directory-versions",
            "Get directory at time — EHR with directory versions",
            "I_EHR_DIRECTORY.get_directory_at_time-ehr_with_directory_versions (master09 §get_directory_at_time)",
            B_DIR,
            "master09 §get_directory_at_time ehr_with_directory_versions (before→empty; between v1/v2→v1; current→v2 — register 06 G-1)",
            Compare::None,
            run_at_time_versions,
        ),
        entry(
            "dir/get-directory-at-time-ehr-with-directory-versions-empty-time",
            "Get directory at time — EHR with directory versions empty time",
            "I_EHR_DIRECTORY.get_directory_at_time-ehr_with_directory_versions_empty_time (master09 §get_directory_at_time)",
            B_DIR,
            "master09 §get_directory_at_time ehr_with_directory_versions_empty_time (→current latest v2)",
            Compare::None,
            run_at_time_versions_empty_time,
        ),
        entry(
            "dir/get-directory-at-time-empty-ehr",
            "Get directory at time — empty EHR",
            "I_EHR_DIRECTORY.get_directory_at_time-empty_ehr (master09 §get_directory_at_time)",
            B_DIR,
            "master09 §get_directory_at_time empty_ehr (→empty); realized 404",
            Compare::None,
            run_at_time_empty_ehr,
        ),
        entry(
            "dir/get-directory-at-time-empty-ehr-empty-time",
            "Get directory at time — empty EHR empty time",
            "I_EHR_DIRECTORY.get_directory_at_time-empty_ehr_empty_time (master09 §get_directory_at_time)",
            B_DIR,
            "master09 §get_directory_at_time empty_ehr_empty_time (→empty); realized 404",
            Compare::None,
            run_at_time_empty_ehr_empty_time,
        ),
        entry(
            "dir/get-directory-at-time-multiple-versions-first",
            "Get directory at time — multiple versions first",
            "I_EHR_DIRECTORY.get_directory_at_time-multiple_versions_first (master09 §get_directory_at_time)",
            B_DIR,
            "master09 §get_directory_at_time multiple_versions_first (time AFTER v1 but BEFORE v2 must return v1 — register 06 G-1, highest priority)",
            Compare::None,
            run_at_time_first,
        ),
        // ── get_directory_at_version (master09 §K) ────────────────────────────
        entry(
            "dir/get-directory-at-version-bad-ehr",
            "Get directory at version — bad EHR",
            "I_EHR_DIRECTORY.get_directory_at_version-bad_ehr (master09 §get_directory_at_version)",
            B_VER,
            "master09 §get_directory_at_version bad_ehr (→error); realized 404",
            Compare::None,
            run_at_version_bad,
        ),
        entry(
            "dir/get-directory-at-version-directory-with-two-versions",
            "Get directory at version — directory with two versions",
            "I_EHR_DIRECTORY.get_directory_at_version-directory_with_two_versions (master09 §get_directory_at_version)",
            B_VER,
            "master09 §get_directory_at_version directory_with_two_versions (v1 uid→v1, v2 uid→v2; body fidelity, register 06 G-2/G-6)",
            Compare::Superset,
            run_at_version_two,
        ),
        entry(
            "dir/get-directory-at-version-empty-ehr",
            "Get directory at version — empty EHR",
            "I_EHR_DIRECTORY.get_directory_at_version-empty_ehr (master09 §get_directory_at_version)",
            B_VER,
            "master09 §get_directory_at_version empty_ehr (→error); realized 404",
            Compare::None,
            run_at_version_empty,
        ),
        // ── get_versioned_directory (master09 §L) — Versioning (register 06 G-3):
        //    the tested OAS exposes no versioned_directory resource, so this is
        //    rebound to GET /directory/{version_uid}; L.2 approximates the
        //    "references the two versions" container semantics by asserting BOTH
        //    versions are reachable.
        entry_ver(
            "dir/get-versioned-directory-empty-ehr",
            "Get versioned directory — empty EHR",
            "I_EHR_DIRECTORY.get_versioned_directory-empty_ehr (master09 §get_versioned_directory)",
            B_VER,
            "master09 §get_versioned_directory empty_ehr; rebound to directory_get_by_version_id (no versioned_directory resource in the tested OAS); RM common master06 §VERSIONED_OBJECT",
            Compare::None,
            run_versioned_empty,
        ),
        entry_ver(
            "dir/get-versioned-directory-directory-with-two-versions",
            "Get versioned directory — directory with two versions",
            "I_EHR_DIRECTORY.get_versioned_directory-directory_with_two_versions (master09 §get_versioned_directory)",
            B_VER,
            "master09 §get_versioned_directory directory_with_two_versions (references the two versions → both reachable; register 06 G-3); RM common master06 §VERSIONED_OBJECT",
            Compare::Superset,
            run_versioned_two,
        ),
        entry_ver(
            "dir/get-versioned-directory-bad-ehr",
            "Get versioned directory — bad EHR",
            "I_EHR_DIRECTORY.get_versioned_directory-bad_ehr (master09 §get_versioned_directory)",
            B_VER,
            "master09 §get_versioned_directory bad_ehr (→error); realized 404",
            Compare::None,
            run_versioned_bad,
        ),
        // ── update / delete on an EHR with no directory yet ───────────────────
        entry(
            "dir/update-directory-empty-ehr",
            "Update directory — empty EHR",
            "I_EHR_DIRECTORY.update_directory-empty_ehr (master09 §update_directory)",
            B_UPDATE,
            "master09 §update_directory empty_ehr (no directory → error); ITS-REST directory_update 404 (412 If-Match form; register 06 G-7)",
            Compare::None,
            run_update_empty,
        ),
        entry(
            "dir/delete-directory-empty-ehr",
            "Delete directory — empty EHR",
            "I_EHR_DIRECTORY.delete_directory-empty_ehr (master09 §delete_directory)",
            B_DELETE,
            "master09 §delete_directory empty_ehr (no directory → error); ITS-REST directory_delete 404 (412 If-Match form; register 06 G-7)",
            Compare::None,
            run_delete_empty,
        ),
    ]
}

// ── entry builders ────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn build(
    id: &'static str,
    title: &'static str,
    schedule: &'static str,
    binding: &'static str,
    citation: &'static str,
    capability: Capability,
    compare: Compare,
    run: CaseRun,
) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            title,
            area: Area::Dir,
            capability,
            formats: JSON,
            citation,
            schedule: ScheduleTrace::Schedule(schedule),
            binding: Binding::Rest(binding),
            compare,
        },
        run,
    }
}

fn entry(
    id: &'static str,
    title: &'static str,
    schedule: &'static str,
    binding: &'static str,
    citation: &'static str,
    compare: Compare,
    run: CaseRun,
) -> CaseEntry {
    build(
        id,
        title,
        schedule,
        binding,
        citation,
        Capability::DirectoryOps,
        compare,
        run,
    )
}

/// A version-read case tagged [`Capability::Versioning`] (CORE) so the
/// versioned-directory reads evidence CORE claimability (register 06 §L / D5).
fn entry_ver(
    id: &'static str,
    title: &'static str,
    schedule: &'static str,
    binding: &'static str,
    citation: &'static str,
    compare: Compare,
    run: CaseRun,
) -> CaseEntry {
    build(
        id,
        title,
        schedule,
        binding,
        citation,
        Capability::Versioning,
        compare,
        run,
    )
}

// ── shared fixtures + helpers ───────────────────────────────────────────────────

fn codec(e: &fixtures::FixtureError) -> CaseError {
    CaseError::Codec(e.to_string())
}

/// The reference FOLDER tree (`/emergency/{episode_x,episode_y}`,
/// `/hospitalization`, …) from the vendored corpus.
fn folder() -> Result<Value, CaseError> {
    let text = fixtures::read_from("directory.folder", "subfolders_in_directory.json")
        .map_err(|e| codec(&e))?;
    serde_json::from_str(&text).map_err(|e| CaseError::Codec(e.to_string()))
}

/// The reference tree with the root FOLDER renamed (so version reads can be told
/// apart by name).
fn folder_named(name: &str) -> Result<Value, CaseError> {
    let mut f = folder()?;
    f["name"]["value"] = Value::String(name.to_owned());
    Ok(f)
}

/// `POST /ehr/{id}/directory` (JSON, `return=representation`).
async fn create_directory(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    body: &Value,
) -> Result<HttpResponse, CaseError> {
    ctx.send(negotiate::representation(
        HttpRequest::post(format!("/ehr/{ehr_id}/directory")).json_body(body)?,
        Format::Json,
    ))
    .await
}

/// `PUT /ehr/{id}/directory` with `If-Match` (JSON, `return=representation`).
async fn update_directory(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    body: &Value,
    if_match: &str,
) -> Result<HttpResponse, CaseError> {
    ctx.send(negotiate::if_match(
        negotiate::representation(
            HttpRequest::put(format!("/ehr/{ehr_id}/directory")).json_body(body)?,
            Format::Json,
        ),
        if_match,
    ))
    .await
}

/// `DELETE /ehr/{id}/directory` with `If-Match`.
async fn delete_directory(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    if_match: &str,
) -> Result<HttpResponse, CaseError> {
    ctx.send(negotiate::if_match(
        HttpRequest::delete(format!("/ehr/{ehr_id}/directory")),
        if_match,
    ))
    .await
}

/// `GET /ehr/{id}/directory` optionally at `version_at_time` (RFC3339 UTC).
async fn get_dir_at(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    at: Option<&str>,
) -> Result<HttpResponse, CaseError> {
    let path = match at {
        Some(t) => format!("/ehr/{ehr_id}/directory?version_at_time={t}"),
        None => format!("/ehr/{ehr_id}/directory"),
    };
    ctx.send(negotiate::accept(HttpRequest::get(path), Format::Json))
        .await
}

/// `GET /ehr/{id}/directory?path=`.
async fn get_dir_path(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    path: &str,
) -> Result<HttpResponse, CaseError> {
    ctx.send(negotiate::accept(
        HttpRequest::get(format!("/ehr/{ehr_id}/directory?path={path}")),
        Format::Json,
    ))
    .await
}

/// `GET /ehr/{id}/directory/{version_uid}`.
async fn get_dir_version(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    version_uid: &str,
) -> Result<HttpResponse, CaseError> {
    ctx.send(negotiate::accept(
        HttpRequest::get(format!("/ehr/{ehr_id}/directory/{version_uid}")),
        Format::Json,
    ))
    .await
}

/// Create an EHR and its root directory (named [`V1_NAME`]); return
/// `(ehr_id, v1_version_uid)`. The version uid comes from [`ids::version_uid`]
/// (`ETag` preferred) with no silent fallback (register 06 G-4).
async fn ehr_with_directory(ctx: &RunContext<'_>) -> Result<(String, String), CaseError> {
    let ehr_id = support::create_ehr(ctx).await?;
    let resp = create_directory(ctx, &ehr_id, &folder_named(V1_NAME)?).await?;
    assert::status(&resp, 201)?;
    let v1 = ids::version_uid(ctx, &resp)?;
    Ok((ehr_id, v1))
}

/// An EHR with two directory versions (v1 → v2 renamed); `(ehr_id, v1, v2)`.
async fn two_versions(ctx: &RunContext<'_>) -> Result<(String, String, String), CaseError> {
    let (ehr_id, v1) = ehr_with_directory(ctx).await?;
    let resp = update_directory(ctx, &ehr_id, &folder_named(V2_NAME)?, &v1).await?;
    assert::status_in(&resp, &[200, 204])?;
    let v2 = ids::version_uid(ctx, &resp)?;
    Ok((ehr_id, v1, v2))
}

/// The two-version directory plus a `t_before` (pre-creation) and a `t_between`
/// (strictly after v1, strictly before v2) RFC3339-UTC instant — the temporal-
/// selection fixture (register 06 G-1). The ~1.5s gaps make the instants
/// unambiguously ordered against the server clock (the SUT is host-colocated).
struct Timed {
    ehr_id: String,
    t_before: String,
    t_between: String,
}

async fn two_versions_timed(ctx: &RunContext<'_>) -> Result<Timed, CaseError> {
    let ehr_id = support::create_ehr(ctx).await?;
    // "Before creation" uses a far-past instant, not a client-clock `now()`:
    // the selection compares against SERVER commit times, and a zero-margin
    // client timestamp flips to 200 under any client/server clock skew (the
    // composition at-time cases use the same fixed-past pattern).
    let t_before = "1900-01-01T00:00:00Z".to_owned();
    let v1resp = create_directory(ctx, &ehr_id, &folder_named(V1_NAME)?).await?;
    assert::status(&v1resp, 201)?;
    let v1 = ids::version_uid(ctx, &v1resp)?;
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let t_between = jiff::Timestamp::now().to_string();
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let v2resp = update_directory(ctx, &ehr_id, &folder_named(V2_NAME)?, &v1).await?;
    assert::status_in(&v2resp, &[200, 204])?;
    Ok(Timed {
        ehr_id,
        t_before,
        t_between,
    })
}

/// A syntactically valid but nonexistent `OBJECT_VERSION_ID`, derived from an
/// observed id (a throwaway directory create) so the SUT's own system id + tree
/// id are reused — no literals (register 06 G-4).
async fn fake_version(ctx: &RunContext<'_>) -> Result<String, CaseError> {
    let ehr_id = support::create_ehr(ctx).await?;
    let resp = create_directory(ctx, &ehr_id, &folder_named(V1_NAME)?).await?;
    assert::status(&resp, 201)?;
    let observed = ids::version_id(ctx, &resp)?;
    Ok(support::nonexistent_version_like(&observed))
}

/// Assert a returned FOLDER's root `name.value` equals `expected`.
fn assert_folder_name(body: &Value, expected: &str, what: &str) -> Result<(), CaseError> {
    match body["name"]["value"].as_str() {
        Some(v) if v == expected => Ok(()),
        other => Err(CaseError::Assertion(format!(
            "{what}: expected root FOLDER name {expected:?}, got {other:?}"
        ))),
    }
}

/// The set of top-level sub-FOLDER names of a FOLDER body.
fn subfolder_names(body: &Value) -> Vec<String> {
    body["folders"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|f| f["name"]["value"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

macro_rules! case {
    ($body:block) => {
        Box::pin(async move { $body })
    };
}

// ── has_directory (master09 §C) ─────────────────────────────────────────────

fn run_has_dir_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = get_dir_at(ctx, &ehr_id, None).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_has_dir_present<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _) = ehr_with_directory(ctx).await?;
        let resp = get_dir_at(ctx, &ehr_id, None).await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_has_dir_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = get_dir_at(ctx, &Uuid::new_v4().to_string(), None).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── has_path (master09 §D) — both branches (register 06 G-2) ─────────────────

fn run_has_path_root<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _) = ehr_with_directory(ctx).await?;
        // '/' → present
        let present = get_dir_path(ctx, &ehr_id, "/").await?;
        assert::status_in(&present, &[200, 204])?;
        // random → absent (the →false row master09 D.2 tabulates)
        let absent = get_dir_path(ctx, &ehr_id, &format!("/{}", Uuid::new_v4())).await?;
        assert::status(&absent, 404)?;
        Ok(DataSetReport {
            passed: 2,
            total: 2,
            schedule_rows: Some(2),
        })
    })
}

fn run_has_path_folder<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _) = ehr_with_directory(ctx).await?;
        // True rows over the reference tree (master09 D.3).
        for p in ["/emergency", "/hospitalization", "/emergency/episode_x"] {
            let r = get_dir_path(ctx, &ehr_id, p).await?;
            assert::status_in(&r, &[200, 204]).map_err(|e| {
                CaseError::Assertion(format!("has_path {p:?} (expected present): {e}"))
            })?;
        }
        // →false rows (the branch the legacy suite never asserted).
        let random_path = format!("/{}", Uuid::new_v4());
        for p in ["/emergency/episode_z", random_path.as_str()] {
            let r = get_dir_path(ctx, &ehr_id, p).await?;
            assert::status(&r, 404).map_err(|e| {
                CaseError::Assertion(format!("has_path {p:?} (expected absent): {e}"))
            })?;
        }
        // 5 of the schedule's 12-row path table driven (register 06 G-2 bound).
        Ok(DataSetReport {
            passed: 5,
            total: 5,
            schedule_rows: Some(12),
        })
    })
}

fn run_has_path_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = get_dir_path(ctx, &ehr_id, "/").await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_has_path_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = get_dir_path(ctx, &Uuid::new_v4().to_string(), "/").await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── create_directory (master09 §E) ──────────────────────────────────────────

fn run_create_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = create_directory(ctx, &ehr_id, &folder()?).await?;
        assert::status(&resp, 201)?;
        assert::header_present(&resp, "etag")?;
        assert::header_present(&resp, "location")?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_create_when_present<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _) = ehr_with_directory(ctx).await?;
        let resp = create_directory(ctx, &ehr_id, &folder()?).await?;
        assert::status(&resp, 409)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_create_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = create_directory(ctx, &Uuid::new_v4().to_string(), &folder()?).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── get_directory (master09 §F) ─────────────────────────────────────────────

fn run_get_dir_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = get_dir_at(ctx, &ehr_id, None).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_root<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _) = ehr_with_directory(ctx).await?;
        let resp = get_dir_at(ctx, &ehr_id, None).await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// F.3 — the returned tree must carry the committed structure (register 06 G-6):
/// every committed top-level sub-FOLDER name is present in the response.
fn run_get_dir_structure<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _) = ehr_with_directory(ctx).await?;
        let resp = get_dir_at(ctx, &ehr_id, None).await?;
        assert::status(&resp, 200)?;
        let returned = subfolder_names(&resp.json()?);
        for name in subfolder_names(&folder()?) {
            if !returned.contains(&name) {
                return Err(CaseError::Assertion(format!(
                    "returned directory omits committed sub-FOLDER {name:?} (got {returned:?})"
                )));
            }
        }
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = get_dir_at(ctx, &Uuid::new_v4().to_string(), None).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── get_directory_at_time (master09 §G) ─────────────────────────────────────

fn run_at_time_current<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _) = ehr_with_directory(ctx).await?;
        let now = jiff::Timestamp::now().to_string();
        let resp = get_dir_at(ctx, &ehr_id, Some(&now)).await?;
        assert::status(&resp, 200)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_at_time_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let now = jiff::Timestamp::now().to_string();
        let resp = get_dir_at(ctx, &Uuid::new_v4().to_string(), Some(&now)).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// G.5 — the three-point temporal selection: before creation → empty; between v1
/// and v2 → v1; current → v2 (register 06 G-1).
fn run_at_time_versions<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let d = two_versions_timed(ctx).await?;
        let before = get_dir_at(ctx, &d.ehr_id, Some(&d.t_before)).await?;
        assert::status(&before, 404)
            .map_err(|e| CaseError::Assertion(format!("before creation must be empty: {e}")))?;
        let between = get_dir_at(ctx, &d.ehr_id, Some(&d.t_between)).await?;
        assert::status(&between, 200)
            .map_err(|e| CaseError::Assertion(format!("between v1/v2 must resolve: {e}")))?;
        assert_folder_name(
            &between.json()?,
            V1_NAME,
            "G.5 between v1 and v2 selects v1",
        )?;
        let current = get_dir_at(ctx, &d.ehr_id, None).await?;
        assert::status(&current, 200)?;
        assert_folder_name(&current.json()?, V2_NAME, "G.5 current selects v2")?;
        Ok(DataSetReport {
            passed: 3,
            total: 3,
            schedule_rows: Some(3),
        })
    })
}

fn run_at_time_versions_empty_time<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, _, _) = two_versions(ctx).await?;
        let resp = get_dir_at(ctx, &ehr_id, None).await?;
        assert::status(&resp, 200)?;
        assert_folder_name(
            &resp.json()?,
            V2_NAME,
            "G.6 empty time selects the latest (v2)",
        )?;
        Ok(DataSetReport::SINGLE)
    })
}

/// G.4 — an EHR **with** a directory, empty time parameter: the current
/// directory version is returned (master09 §`get_directory_at_time`
/// `ehr_with_directory_empty_time`).
fn run_at_time_empty_time<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let f = folder_named(V1_NAME)?;
        let created = create_directory(ctx, &ehr_id, &f).await?;
        assert::status(&created, 201)?;
        let resp = get_dir_at(ctx, &ehr_id, None).await?;
        assert::status(&resp, 200)?;
        assert_folder_name(
            &resp.json()?,
            V1_NAME,
            "G.4 empty time selects the current directory",
        )?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_at_time_empty_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let now = jiff::Timestamp::now().to_string();
        let resp = get_dir_at(ctx, &ehr_id, Some(&now)).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_at_time_empty_ehr_empty_time<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = get_dir_at(ctx, &ehr_id, None).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// G.8 — highest-priority temporal fix: a time AFTER v1 but BEFORE v2 must
/// return v1 (register 06 G-1).
fn run_at_time_first<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let d = two_versions_timed(ctx).await?;
        let between = get_dir_at(ctx, &d.ehr_id, Some(&d.t_between)).await?;
        assert::status(&between, 200)?;
        assert_folder_name(
            &between.json()?,
            V1_NAME,
            "G.8 time between v1 and v2 must select v1",
        )?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── has_directory_version (master09 §J) ─────────────────────────────────────

fn run_has_ver_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let fake = fake_version(ctx).await?;
        let resp = get_dir_version(ctx, &ehr_id, &fake).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// J.2 — BOTH versions exist → 200 (register 06 G-2).
fn run_has_ver_present<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, v1, v2) = two_versions(ctx).await?;
        for (v, label) in [(&v1, "v1"), (&v2, "v2")] {
            let resp = get_dir_version(ctx, &ehr_id, v).await?;
            assert::status(&resp, 200)
                .map_err(|e| CaseError::Assertion(format!("has_directory_version {label}: {e}")))?;
        }
        Ok(DataSetReport {
            passed: 2,
            total: 2,
            schedule_rows: Some(2),
        })
    })
}

fn run_has_ver_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let fake = fake_version(ctx).await?;
        let resp = get_dir_version(ctx, &Uuid::new_v4().to_string(), &fake).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── get_directory_at_version (master09 §K) ──────────────────────────────────

fn run_at_version_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let fake = fake_version(ctx).await?;
        let resp = get_dir_version(ctx, &Uuid::new_v4().to_string(), &fake).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// K.2 — v1 uid returns v1 content, v2 uid returns v2 content (register 06
/// G-2/G-6 body fidelity).
fn run_at_version_two<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, v1, v2) = two_versions(ctx).await?;
        let r1 = get_dir_version(ctx, &ehr_id, &v1).await?;
        assert::status(&r1, 200)?;
        assert_folder_name(&r1.json()?, V1_NAME, "K.2 v1 uid returns v1")?;
        let r2 = get_dir_version(ctx, &ehr_id, &v2).await?;
        assert::status(&r2, 200)?;
        assert_folder_name(&r2.json()?, V2_NAME, "K.2 v2 uid returns v2")?;
        Ok(DataSetReport {
            passed: 2,
            total: 2,
            schedule_rows: Some(2),
        })
    })
}

fn run_at_version_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let fake = fake_version(ctx).await?;
        let resp = get_dir_version(ctx, &ehr_id, &fake).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── get_versioned_directory (master09 §L) — rebound (register 06 G-3) ────────

fn run_versioned_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let fake = fake_version(ctx).await?;
        let resp = get_dir_version(ctx, &ehr_id, &fake).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// L.2 — approximate the `VERSIONED_OBJECT` "references the two versions" semantics
/// by asserting BOTH versions are reachable and return their own content
/// (register 06 G-3 — the OAS has no `versioned_directory` resource to drive).
fn run_versioned_two<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, v1, v2) = two_versions(ctx).await?;
        let r1 = get_dir_version(ctx, &ehr_id, &v1).await?;
        assert::status(&r1, 200)?;
        assert_folder_name(&r1.json()?, V1_NAME, "L.2 v1 reachable")?;
        let r2 = get_dir_version(ctx, &ehr_id, &v2).await?;
        assert::status(&r2, 200)?;
        assert_folder_name(&r2.json()?, V2_NAME, "L.2 v2 reachable")?;
        Ok(DataSetReport {
            passed: 2,
            total: 2,
            schedule_rows: Some(2),
        })
    })
}

fn run_versioned_bad<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let fake = fake_version(ctx).await?;
        let resp = get_dir_version(ctx, &Uuid::new_v4().to_string(), &fake).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── update_directory (master09 §H) ──────────────────────────────────────────

fn run_update<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, v1) = ehr_with_directory(ctx).await?;
        let resp = update_directory(ctx, &ehr_id, &folder_named(V2_NAME)?, &v1).await?;
        assert::status_in(&resp, &[200, 204])?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_update_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let fake = fake_version(ctx).await?;
        let resp = update_directory(ctx, &Uuid::new_v4().to_string(), &folder()?, &fake).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_update_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // EHR exists but has no directory; the If-Match names a nonexistent
        // version → non-existent directory (register 06 G-7).
        let ehr_id = support::create_ehr(ctx).await?;
        let fake = fake_version(ctx).await?;
        let resp = update_directory(ctx, &ehr_id, &folder()?, &fake).await?;
        assert::status_ladder(
            ctx,
            &resp,
            ABSENT_RUNGS,
            "H.2 update a non-existent directory",
        )?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── delete_directory (master09 §I) ──────────────────────────────────────────

fn run_delete<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, v1) = ehr_with_directory(ctx).await?;
        let resp = delete_directory(ctx, &ehr_id, &v1).await?;
        // ITS-REST directory_delete declares 204_deleted for success.
        assert::status(&resp, 204)?;
        // Logical-delete post-condition (master09 NOTE; RM common master06
        // §Change control): the directory becomes a deleted version — the current
        // retrieval reports it gone (204_deleted_at_time / 404 no version).
        let after = get_dir_at(ctx, &ehr_id, None).await?;
        assert::status_in(&after, &[204, 404])?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_delete_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let fake = fake_version(ctx).await?;
        let resp = delete_directory(ctx, &Uuid::new_v4().to_string(), &fake).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_delete_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let fake = fake_version(ctx).await?;
        let resp = delete_directory(ctx, &ehr_id, &fake).await?;
        assert::status_ladder(
            ctx,
            &resp,
            ABSENT_RUNGS,
            "I.1 delete a non-existent directory",
        )?;
        Ok(DataSetReport::SINGLE)
    })
}
