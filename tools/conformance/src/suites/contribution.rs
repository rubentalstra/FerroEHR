//! CONTRIBUTION cases (`I_EHR_CONTRIBUTION`), authored spine-first from the CNF
//! platform test schedule `master08-func_tc_ehr_contribution.adoc` and the
//! vendored ITS-REST contract (`ehr-codegen.openapi.yaml` operations
//! `contribution_create` / `contribution_get`).
//!
//! Every expected status/header/body condition traces to master08 or the
//! contract, never to observed server behaviour. The `contribution_create`
//! operation declares exactly `{201, 400_CONTRIBUTION, 404_unknown_ehr_id,
//! 409}`; there is no `422` for CONTRIBUTION, so an invalid commit is the
//! contract's `400` (with `422` as the edition-ladder's lower rung — the
//! Unprocessable-Entity form prior ITS-REST editions / other CDRs emit for
//! schema-valid-but-RM-invalid content). CONTRIBUTION is JSON only on the wire
//! (a version-set + audit wrapper with no canonical-XML shape — master08 §Test
//! Data Sets; version-family XML is tracked separately).
//!
//! `list_contributions` (master08 §F) is skip-with-reason: the SM operation has
//! no ITS-REST binding (`/ehr/{ehr_id}/contribution` is POST-only, no GET
//! collection resource in the tested development@e8a093e OAS nor Release-1.0.3).
//! Wire ids come only from [`crate::wire::ids`]; the sole local body reader is
//! [`version_uid_in`] (a structured `versions[i].id.value` RM field, not an `ETag`
//! scrape), which errors rather than falling back (no silent id fallback).

use serde_json::{Value, json};
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

/// JSON-only formats (master08 §Test Data Sets: the CONTRIBUTION wire is
/// canonical JSON; no canonical-XML shape).
const JSON: &[Format] = &[Format::Json];

/// The ITS-REST binding every commit case drives.
const COMMIT_BINDING: &str = "POST /ehr/{ehr_id}/contribution";
/// The ITS-REST binding the get/has cases drive.
const GET_BINDING: &str = "GET /ehr/{ehr_id}/contribution/{contribution_uid}";

/// Invalidity ladder for semantically-invalid committed content: `422` —
/// "content type and syntax is correct … but there are semantic validation
/// errors, such as the underlying template is not known or is not validating
/// the supplied resource" (ITS-REST `Requests_and_responses` §422 + the OAS
/// `responses/422.yaml`; the same rule `composition_create` enumerates).
/// `400_CONTRIBUTION` is scoped to parse/syntax/modification-type errors
/// only, and the spec's prose reserves plain 400 for "when no other 4xx is
/// appropriate" — so the Release-1.0.3-era plain-400 form is the LOWER rung.
const INVALID_RUNGS: &[(Edition, u16)] = &[(Edition::Development, 422), (Edition::Release103, 400)];
/// Conflict ladder: `contribution_create` declares `409` for a change-control
/// conflict.
const CONFLICT_RUNGS: &[(Edition, u16)] = &[(Edition::Development, 409)];
/// The contract's declared client-error codes for a commit against an existing
/// EHR (`404_unknown_ehr_id` excluded — the EHR exists), used where master08
/// states only "negative" without pinning the cause to one code.
const NEGATIVE_SET: &[u16] = &[400, 409];

/// The registered master08 CONTRIBUTION cases.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "the registered ECC case table is inherently enumerative"
)]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // ── commit_contribution: COMPOSITION (master08 §Test Cases C) ─────────
        commit(
            "ctb/commit-contribution-valid-composition",
            "Commit contribution — valid composition",
            "I_EHR_CONTRIBUTION.commit_contribution-valid_composition (master08 §Test Cases)",
            "master08 §valid_composition; ITS-REST contribution_create 201_CONTRIBUTION; RM common master06 §Version",
            Compare::Superset,
            run_valid_composition,
        ),
        commit(
            "ctb/commit-contribution-invalid-composition",
            "Commit contribution — invalid composition",
            "I_EHR_CONTRIBUTION.commit_contribution-invalid_composition (master08 §Test Cases)",
            "master08 §invalid_composition; ITS-REST contribution_create 400_CONTRIBUTION; RM ehr master07 §COMPOSITION.composer mandatory",
            Compare::None,
            run_invalid_composition,
        ),
        commit(
            "ctb/commit-contribution-empty",
            "Commit contribution — empty",
            "I_EHR_CONTRIBUTION.commit_contribution-empty (master08 §Test Cases)",
            "master08 §empty (General B.4 empty CONTRIBUTION); ITS-REST contribution_create 400_CONTRIBUTION",
            Compare::None,
            run_empty,
        ),
        commit(
            "ctb/commit-contribution-valid-invalid-compositions",
            "Commit contribution — valid invalid compositions",
            "I_EHR_CONTRIBUTION.commit_contribution-valid_invalid_compositions (master08 §Test Cases D)",
            "master08 §multiple versions table D (if any COMPOSITION is invalid the whole commit fails); RM common master06 §Contributions (atomic)",
            Compare::None,
            run_valid_invalid_compositions,
        ),
        commit(
            "ctb/commit-contribution-non-exiting-opt",
            "Commit contribution — non exiting OPT",
            "I_EHR_CONTRIBUTION.commit_contribution-non_exiting_opt (master08 §Test Cases)",
            "master08 §non_exiting_opt (COMPOSITION B.2.b referenced OPT not loaded); ITS-REST contribution_create 400_CONTRIBUTION",
            Compare::None,
            run_non_existing_opt,
        ),
        commit(
            "ctb/commit-contribution-event-composition",
            "Commit contribution — event composition",
            "I_EHR_CONTRIBUTION.commit_contribution-event_composition (master08 §Test Cases)",
            "master08 §event_composition (creation then modification → version 2); RM common master06 §Version tree",
            Compare::None,
            run_event_composition,
        ),
        commit(
            "ctb/commit-contribution-persistent-composition",
            "Commit contribution — persistent composition",
            "I_EHR_CONTRIBUTION.commit_contribution-persistent_composition (master08 §Test Cases)",
            "master08 §persistent_composition (COMPOSITION B.1.c; create then modify → version 2); RM common master06 §Version tree",
            Compare::None,
            run_persistent_composition,
        ),
        commit(
            "ctb/commit-contribution-delete",
            "Commit contribution — delete",
            "I_EHR_CONTRIBUTION.commit_contribution-delete (master08 §Test Cases)",
            "master08 §delete (VERSIONED_OBJECT logically deleted); RM common master06 §Change control (deleted VERSION data Void, logical delete)",
            Compare::None,
            run_delete,
        ),
        commit(
            "ctb/commit-contribution-two-commits-second-invalid",
            "Commit contribution — two commits second invalid",
            "I_EHR_CONTRIBUTION.commit_contribution-two_commits_second_invalid (master08 §Test Cases)",
            "master08 §two_commits_second_invalid (only one VERSION remains); RM common master06 §Contributions (atomic)",
            Compare::None,
            run_two_commits_second_invalid,
        ),
        commit(
            "ctb/commit-contribution-two-commits-second-creation",
            "Commit contribution — two commits second creation",
            "I_EHR_CONTRIBUTION.commit_contribution-two_commits_second_creation (master08 §Test Cases)",
            "master08 §two_commits_second_creation (only one 'create' per object); RM common master06 §Change control",
            Compare::None,
            run_two_commits_second_creation,
        ),
        // ── commit_contribution: EHR_STATUS (master08 §EHR_STATUS Commit) ─────
        commit(
            "ctb/commit-contribution-minimal-ehr-status",
            "Commit contribution — minimal EHR status",
            "I_EHR_CONTRIBUTION.commit_contribution-minimal_ehr_status (master08 §EHR_STATUS CONTRIBUTION Commit)",
            "master08 §EHR_STATUS Accepted Cases 15-row matrix (is_modifiable × is_queryable × subject.external_ref), scenario 1 default EHR_STATUS",
            Compare::None,
            run_minimal_ehr_status,
        ),
        commit(
            "ctb/commit-contribution-full-ehr-status",
            "Commit contribution — full EHR status",
            "I_EHR_CONTRIBUTION.commit_contribution-full_ehr_status (master08 §EHR_STATUS CONTRIBUTION Commit)",
            "master08 §EHR_STATUS Accepted Cases scenario 2 (EHR created by providing an EHR_STATUS), 15-row matrix",
            Compare::None,
            run_full_ehr_status,
        ),
        commit(
            "ctb/commit-contribution-ehr-status-invalid-change-type",
            "Commit contribution — EHR status invalid change type",
            "I_EHR_CONTRIBUTION.commit_contribution-ehr_status_invalid_change_type (master08 §EHR_STATUS CONTRIBUTION Commit)",
            "master08 §EHR_STATUS Reject 1 (change_type ∈ {creation, deleted} rejected — STATUS already exists, cannot be deleted)",
            Compare::None,
            run_ehr_status_invalid_change_type,
        ),
        commit(
            "ctb/commit-contribution-invalid-ehr-status",
            "Commit contribution — invalid EHR status",
            "I_EHR_CONTRIBUTION.commit_contribution-invalid_ehr_status (master08 §EHR_STATUS CONTRIBUTION Commit)",
            "master08 §EHR_STATUS Reject 4 (invalid EHR_STATUS); ITS-REST contribution_create 400_CONTRIBUTION",
            Compare::None,
            run_invalid_ehr_status,
        ),
        // ── commit_contribution: FOLDER / directory (master08 §FOLDER Commit) ─
        commit(
            "ctb/commit-contribution-valid-directory",
            "Commit contribution — valid directory",
            "I_EHR_CONTRIBUTION.commit_contribution-valid_directory (master08 §FOLDER CONTRIBUTION Commit)",
            "master08 §valid_directory (creation to an EHR with no directory); RM ehr master04 §Folders",
            Compare::Superset,
            run_valid_directory,
        ),
        commit(
            "ctb/commit-contribution-fail-create-existing-directory",
            "Commit contribution — fail create existing directory",
            "I_EHR_CONTRIBUTION.commit_contribution-fail_create_existing_directory (master08 §FOLDER CONTRIBUTION Commit)",
            "master08 §fail_create_existing_directory (root FOLDER already present); ITS-REST contribution_create 409",
            Compare::None,
            run_fail_create_existing_directory,
        ),
        commit(
            "ctb/commit-contribution-fail-modify-non-existing-directory",
            "Commit contribution — fail modify non existing directory",
            "I_EHR_CONTRIBUTION.commit_contribution-fail_modify_non_existing_directory (master08 §FOLDER CONTRIBUTION Commit)",
            "master08 §fail_modify_non_existing_directory (modify a directory that does not exist)",
            Compare::None,
            run_fail_modify_non_existing_directory,
        ),
        commit(
            "ctb/commit-contribution-update-existing-directory",
            "Commit contribution — update existing directory",
            "I_EHR_CONTRIBUTION.commit_contribution-update_existing_directory (master08 §FOLDER CONTRIBUTION Commit)",
            "master08 §update_existing_directory (modify/amend an existing directory → new FOLDER version)",
            Compare::None,
            run_update_existing_directory,
        ),
        // ── get_contribution (master08 §H) ────────────────────────────────────
        get_case(
            "ctb/get-contribution-existing",
            "Get contribution — existing",
            "I_EHR_CONTRIBUTION.get_contribution-existing (master08 §get_contribution)",
            "master08 §get_contribution existing; ITS-REST contribution_get 200_CONTRIBUTION",
            Compare::Superset,
            run_get_existing,
        ),
        get_case(
            "ctb/get-contribution-empty-ehr",
            "Get contribution — empty EHR",
            "I_EHR_CONTRIBUTION.get_contribution-empty_ehr (master08 §get_contribution)",
            "master08 §get_contribution empty_ehr (error); ITS-REST contribution_get 404_CONTRIBUTION",
            Compare::None,
            run_get_empty_ehr,
        ),
        get_case(
            "ctb/get-contribution-bad-ehr",
            "Get contribution — bad EHR",
            "I_EHR_CONTRIBUTION.get_contribution-bad_ehr (master08 §get_contribution)",
            "master08 §get_contribution bad_ehr (error); ITS-REST contribution_get 404_CONTRIBUTION",
            Compare::None,
            run_get_bad_ehr,
        ),
        get_case(
            "ctb/get-contribution-bad-contribution",
            "Get contribution — bad contribution",
            "I_EHR_CONTRIBUTION.get_contribution-bad_contribution (master08 §get_contribution)",
            "master08 §get_contribution bad_contribution (error); ITS-REST contribution_get 404_CONTRIBUTION",
            Compare::None,
            run_get_bad_contribution,
        ),
        // ── has_contribution (master08 §G) — SM boolean realized as GET
        //    /contribution/{uid} (200 has / 404 not); the false/error trichotomy
        //    collapses to 200/404 by the CNF guide element-2 mapping.
        //    The native ehrbase-sm surface keeps the distinction; the wire
        //    runner must NOT "fix" a 404 into a 200-false.
        get_case(
            "ctb/has-contribution-existing",
            "Contribution existence check — existing",
            "I_EHR_CONTRIBUTION.has_contribution-existing (master08 §has_contribution)",
            "master08 §has_contribution existing (→true); realized 200 via ITS-REST contribution_get",
            Compare::Superset,
            run_get_existing,
        ),
        get_case(
            "ctb/has-contribution-bad-contribution",
            "Contribution existence check — bad contribution",
            "I_EHR_CONTRIBUTION.has_contribution-bad_contribution (master08 §has_contribution)",
            "master08 §has_contribution bad_contribution (→false); realized 404 via ITS-REST contribution_get",
            Compare::None,
            run_get_bad_contribution,
        ),
        get_case(
            "ctb/has-contribution-bad-ehr",
            "Contribution existence check — bad EHR",
            "I_EHR_CONTRIBUTION.has_contribution-bad_ehr (master08 §has_contribution)",
            "master08 §has_contribution bad_ehr (→error); realized 404 via ITS-REST contribution_get",
            Compare::None,
            run_get_bad_ehr,
        ),
        get_case(
            "ctb/has-contribution-empty-ehr",
            "Contribution existence check — empty EHR",
            "I_EHR_CONTRIBUTION.has_contribution-empty_ehr (master08 §has_contribution)",
            "master08 §has_contribution empty_ehr (→false); realized 404 via ITS-REST contribution_get (element-2 collapse)",
            Compare::None,
            run_get_empty_ehr,
        ),
        // ── list_contributions (master08 §F) — no ITS-REST binding → skip ─
        skip_case(
            "ctb/list-contributions-empty",
            "List contributions — empty",
            "I_EHR_CONTRIBUTION.list_contributions-empty (master08 §list_contributions)",
        ),
        skip_case(
            "ctb/list-contributions-non-existing-ehr",
            "List contributions — non existing EHR",
            "I_EHR_CONTRIBUTION.list_contributions-non_existing_ehr (master08 §list_contributions)",
        ),
        skip_case(
            "ctb/list-contributions-post-commit",
            "List contributions — post commit",
            "I_EHR_CONTRIBUTION.list_contributions-post_commit (master08 §list_contributions)",
        ),
        skip_case(
            "ctb/list-contributions-ehr-containing-directory",
            "List contributions — EHR containing directory",
            "I_EHR_CONTRIBUTION.list_contributions-ehr_containing_directory (master08 §list_contributions)",
        ),
        skip_case(
            "ctb/list-contributions-ehr-containing-ehr-status",
            "List contributions — EHR containing EHR status",
            "I_EHR_CONTRIBUTION.list_contributions-ehr_containing_ehr_status (master08 §list_contributions)",
        ),
    ]
}

// ── entry builders ────────────────────────────────────────────────────────────

fn meta(
    id: &'static str,
    title: &'static str,
    schedule: &'static str,
    binding: Binding,
    citation: &'static str,
    compare: Compare,
) -> CaseMeta {
    CaseMeta {
        id,
        title,
        area: Area::Ctb,
        capability: Capability::ChangeSets,
        formats: JSON,
        citation,
        schedule: ScheduleTrace::Schedule(schedule),
        binding,
        compare,
    }
}

fn commit(
    id: &'static str,
    title: &'static str,
    schedule: &'static str,
    citation: &'static str,
    compare: Compare,
    run: CaseRun,
) -> CaseEntry {
    CaseEntry {
        meta: meta(
            id,
            title,
            schedule,
            Binding::Rest(COMMIT_BINDING),
            citation,
            compare,
        ),
        run,
    }
}

fn get_case(
    id: &'static str,
    title: &'static str,
    schedule: &'static str,
    citation: &'static str,
    compare: Compare,
    run: CaseRun,
) -> CaseEntry {
    CaseEntry {
        meta: meta(
            id,
            title,
            schedule,
            Binding::Rest(GET_BINDING),
            citation,
            compare,
        ),
        run,
    }
}

/// A `list_contributions` case: the SM operation has no ITS-REST binding, so the
/// case skips-with-reason rather than fabricating a URL.
fn skip_case(id: &'static str, title: &'static str, schedule: &'static str) -> CaseEntry {
    CaseEntry {
        meta: meta(
            id,
            title,
            schedule,
            Binding::NoRestBinding(
                "I_EHR_CONTRIBUTION.list_contributions (master08 §list_contributions)",
            ),
            "master08 §list_contributions — SM operation with no ITS-REST binding (/ehr/{ehr_id}/contribution is POST-only; no GET collection resource in development@e8a093e nor Release-1.0.3)",
            Compare::None,
        ),
        run: run_skip_list,
    }
}

// ── shared fixtures + helpers ───────────────────────────────────────────────────

fn codec(e: &fixtures::FixtureError) -> CaseError {
    CaseError::Codec(e.to_string())
}

/// Load a contribution fixture (a named file under a `corpus-dir:` manifest key)
/// as canonical JSON.
fn load(dir_key: &str, file: &str) -> Result<Value, CaseError> {
    let text = fixtures::read_from(dir_key, file).map_err(|e| codec(&e))?;
    serde_json::from_str(&text).map_err(|e| CaseError::Codec(e.to_string()))
}

/// Commit a CONTRIBUTION against `ehr_id` (JSON, `return=representation`).
async fn commit_req(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    body: &Value,
) -> Result<HttpResponse, CaseError> {
    ctx.send(negotiate::representation(
        HttpRequest::post(format!("/ehr/{ehr_id}/contribution")).json_body(body)?,
        Format::Json,
    ))
    .await
}

/// The `OBJECT_VERSION_ID` of the version at index `i` in a committed
/// CONTRIBUTION representation (`versions[i].id.value`) — a structured RM field,
/// not a wire-header scrape. Errors (never falls back) if absent, so a missing
/// version id is a case failure. The commit's `ETag` is the
/// *CONTRIBUTION* uid, not the version uid, so it cannot serve here.
fn version_uid_in(body: &Value, i: usize) -> Result<String, CaseError> {
    body["versions"][i]["id"]["value"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| {
            CaseError::Assertion(format!(
                "committed CONTRIBUTION has no versions[{i}].id.value (body: {})",
                truncate(&body.to_string())
            ))
        })
}

/// Assert the version-tree id of an `OBJECT_VERSION_ID` equals `expected`
/// (`::1` = a creation, `::2` = the first modification; RM common master06
/// §Version tree).
fn assert_version_number(uid: &str, expected: &str, what: &str) -> Result<(), CaseError> {
    let ovid = ids::parse_object_version_id(uid)?;
    if ovid.version_tree_id == expected {
        Ok(())
    } else {
        Err(CaseError::Assertion(format!(
            "{what}: expected version_tree_id {expected}, got {} ({uid})",
            ovid.version_tree_id
        )))
    }
}

/// Set `versions[i].preceding_version_uid` (fixture adaptation: the vendored
/// modification/delete fixtures carry a placeholder preceding uid that must be
/// the predecessor returned by the first commit — additive, never a defect fix).
fn set_preceding(body: &mut Value, i: usize, uid: &str) {
    body["versions"][i]["preceding_version_uid"] =
        json!({ "_type": "OBJECT_VERSION_ID", "value": uid });
}

/// A `DV_CODED_TEXT` for an openEHR `audit change type` group code + rubric.
///
// NOTE: the codes 249=creation / 251=modification / 253=deleted are the
// openEHR Terminology `audit change type` group (master08 §Data Set
// Considerations) — edition-invariant. Ideally they would be sourced from
// `openehr-term` rather than as literals; `tools/conformance` does not depend on
// `openehr-term` (it would need a Cargo.toml dependency this file cannot add), so
// they stay literals here — a fix-pass item, not a spec-facing risk.
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
// NOTE (fixture adaptation): the vendored `*.contribution.modification`
// fixtures are labelled `value:"modification"` but carry `code_string:"249"`
// (creation) — an internally-inconsistent RM-1.0.x-era fixture. The server
// treats `defining_code` as authoritative, so this corrects the code to the
// fixture's stated intent; it never fixes an *intended* defect (standing rule 3).
fn normalize_modification(body: &mut Value, i: usize) {
    body["versions"][i]["commit_audit"]["change_type"] = change_type("251", "modification");
}

/// The current `EHR_STATUS` version uid (`GET /ehr/{id}/ehr_status`).
async fn ehr_status_uid(ctx: &RunContext<'_>, ehr_id: &str) -> Result<String, CaseError> {
    let resp = ctx
        .send(negotiate::accept(
            HttpRequest::get(format!("/ehr/{ehr_id}/ehr_status")),
            Format::Json,
        ))
        .await?;
    assert::status(&resp, 200)?;
    ids::body_uid(&resp.json()?)
}

/// Create an EHR by providing a full `EHR_STATUS` (`POST /ehr` with a body) —
/// master08 §`EHR_STATUS` Combinations scenario 2. Returns the `ehr_id`.
async fn create_ehr_with_status(ctx: &RunContext<'_>, status: &Value) -> Result<String, CaseError> {
    let resp = ctx
        .send(negotiate::representation(
            HttpRequest::post("/ehr").json_body(status)?,
            Format::Json,
        ))
        .await?;
    assert::status(&resp, 201)?;
    ids::ehr_id(&resp.json()?)
}

/// Assert an invalid-payload rejection: the contract's `400_CONTRIBUTION`
/// (development), `422` the prior-edition Unprocessable-Entity form.
fn assert_invalid(ctx: &RunContext<'_>, resp: &HttpResponse, what: &str) -> Result<(), CaseError> {
    assert::status_ladder(ctx, resp, INVALID_RUNGS, what).map(|_| ())
}

/// Assert a change-control conflict: `contribution_create` declares `409`.
fn assert_conflict(ctx: &RunContext<'_>, resp: &HttpResponse, what: &str) -> Result<(), CaseError> {
    assert::status_ladder(ctx, resp, CONFLICT_RUNGS, what).map(|_| ())
}

fn truncate(s: &str) -> String {
    s.chars().take(200).collect()
}

macro_rules! case {
    ($body:block) => {
        Box::pin(async move { $body })
    };
}

// ── commit_contribution: COMPOSITION ─────────────────────────────────────────

/// C.1 — commit a valid COMPOSITION → 201 + `ETag`; the created VERSION is `::1`;
/// the CONTRIBUTION is then retrievable with a non-empty version list.
fn run_valid_composition<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        support::ensure_opt(ctx, "template.valid", "minimal/minimal_evaluation.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let body = load(
            "contribution.valid",
            "minimal/minimal_evaluation.contribution.json",
        )?;
        let resp = commit_req(ctx, &ehr_id, &body).await?;
        assert::status(&resp, 201)?;
        assert::header_present(&resp, "etag")?;
        let repr = resp.json()?;
        assert_version_number(&version_uid_in(&repr, 0)?, "1", "C.1 creation is version 1")?;
        // Post-condition (master08 §get_contribution flow): the committed
        // CONTRIBUTION is retrievable and reports its version(s).
        let ctb_uid = ids::contribution_uid(ctx, &resp)?;
        let got = get_contribution(ctx, &ehr_id, &ctb_uid).await?;
        assert::status(&got, 200)?;
        let cbody = got.json()?;
        if cbody["_type"] != "CONTRIBUTION" {
            return Err(CaseError::Assertion(format!(
                "post-condition: expected CONTRIBUTION, got {}",
                cbody["_type"]
            )));
        }
        if cbody["versions"]
            .as_array()
            .is_none_or(std::vec::Vec::is_empty)
        {
            return Err(CaseError::Assertion(
                "post-condition: retrieved CONTRIBUTION has an empty version list".to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE)
    })
}

/// C.2 — an invalid COMPOSITION (mandatory `composer` removed) is rejected.
fn run_invalid_composition<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        support::ensure_opt(ctx, "template.valid", "minimal/minimal_evaluation.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let mut body = load(
            "contribution.valid",
            "minimal/minimal_evaluation.contribution.json",
        )?;
        if let Some(data) = body["versions"][0]["data"].as_object_mut() {
            data.remove("composer"); // COMPOSITION.composer is mandatory (RM ehr master07)
        }
        let resp = commit_req(ctx, &ehr_id, &body).await?;
        assert_invalid(ctx, &resp, "C.2 invalid COMPOSITION (composer removed)")?;
        Ok(DataSetReport::SINGLE)
    })
}

/// C.3 — an empty CONTRIBUTION (no VERSIONs) is rejected.
fn run_empty<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let body = load("contribution.invalid", "no_versions.json")?;
        let resp = commit_req(ctx, &ehr_id, &body).await?;
        assert_invalid(ctx, &resp, "C.3 empty CONTRIBUTION (no VERSIONs)")?;
        Ok(DataSetReport::SINGLE)
    })
}

/// C.4 — a mix of valid + invalid VERSIONs is rejected transactionally; the
/// wire-observable rollback signal is that no CONTRIBUTION resource is created.
fn run_valid_invalid_compositions<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        support::ensure_opt(ctx, "template.valid", "minimal/minimal_evaluation.opt").await?;
        support::ensure_opt(ctx, "template.valid", "minimal/minimal_admin.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let body = load(
            "contribution.invalid",
            "multiple_valid_and_invalid_compos.json",
        )?;
        let resp = commit_req(ctx, &ehr_id, &body).await?;
        assert_invalid(ctx, &resp, "C.4 mixed valid+invalid commit rejected")?;
        // Atomic-rollback post-condition (RM common master06 §Contributions):
        // no CONTRIBUTION/VERSION persists. Wire-observable signal — the reject
        // emits no created-resource header (Location/ETag). Full contribution-
        // count verification needs list_contributions, which has no ITS-REST
        // binding (master08 §F), so the created-resource-absence
        // check is the spec-determined part asserted here.
        if resp.header("location").is_some() || resp.header("etag").is_some() {
            return Err(CaseError::Assertion(
                "atomic rollback: a rejected mixed commit created a resource (Location/ETag present)".to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE.of_schedule_rows(4))
    })
}

/// C.10 — a COMPOSITION referencing an OPT never loaded is rejected.
fn run_non_existing_opt<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let body = load("contribution.invalid", "ref_to_non_existent_OPT.json")?;
        let resp = commit_req(ctx, &ehr_id, &body).await?;
        assert_invalid(ctx, &resp, "C.10 COMPOSITION references a non-existent OPT")?;
        Ok(DataSetReport::SINGLE)
    })
}

/// C.5 — an event COMPOSITION, then modified in a second commit → version 2.
fn run_event_composition<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        support::ensure_opt(ctx, "template.valid", "minimal/minimal_admin.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let created = commit_req(
            ctx,
            &ehr_id,
            &load(
                "contribution.valid",
                "minimal/minimal_admin.contribution.json",
            )?,
        )
        .await?;
        assert::status(&created, 201)?;
        let v1 = version_uid_in(&created.json()?, 0)?;

        let mut modify = load(
            "contribution.valid",
            "minimal/minimal_admin.contribution.modification.complete.json",
        )?;
        set_preceding(&mut modify, 0, &v1);
        let updated = commit_req(ctx, &ehr_id, &modify).await?;
        assert::status(&updated, 201)?;
        assert_version_number(
            &version_uid_in(&updated.json()?, 0)?,
            "2",
            "C.5 modification is version 2",
        )?;
        Ok(DataSetReport::SINGLE)
    })
}

/// C.6 — a persistent COMPOSITION, then modified → version 2.
fn run_persistent_composition<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        support::ensure_opt(
            ctx,
            "template.valid",
            "minimal_persistent/persistent_minimal.opt",
        )
        .await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let created = commit_req(
            ctx,
            &ehr_id,
            &load(
                "contribution.valid",
                "minimal_persistent/minimal_persistent.contribution.json",
            )?,
        )
        .await?;
        assert::status(&created, 201)?;
        let v1 = version_uid_in(&created.json()?, 0)?;

        let mut modify = load(
            "contribution.valid",
            "minimal_persistent/minimal_persistent.contribution.modification.json",
        )?;
        set_preceding(&mut modify, 0, &v1);
        let updated = commit_req(ctx, &ehr_id, &modify).await?;
        assert::status(&updated, 201)?;
        assert_version_number(
            &version_uid_in(&updated.json()?, 0)?,
            "2",
            "C.6 modification is version 2",
        )?;
        Ok(DataSetReport::SINGLE)
    })
}

/// C.7 — create then delete a COMPOSITION; the deleted VERSION is version 2 and
/// the object is logically deleted (its current retrieval reports deleted/gone).
fn run_delete<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        support::ensure_opt(ctx, "template.valid", "minimal/minimal_admin.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let created = commit_req(
            ctx,
            &ehr_id,
            &load(
                "contribution.valid",
                "minimal/minimal_admin.contribution.json",
            )?,
        )
        .await?;
        assert::status(&created, 201)?;
        let v1 = version_uid_in(&created.json()?, 0)?;

        // The deleted VERSION's data is Void (RM common master06 §Change control)
        // — the vendored fixture carries data, nulled here as an additive RM-shape
        // adaptation, not a defect fix.
        let mut delete = load(
            "contribution.valid",
            "minimal/minimal_admin.contribution.deleted.deleted.json",
        )?;
        delete["versions"][0]["data"] = Value::Null;
        set_preceding(&mut delete, 0, &v1);
        let deleted = commit_req(ctx, &ehr_id, &delete).await?;
        assert::status(&deleted, 201)?;
        assert_version_number(
            &version_uid_in(&deleted.json()?, 0)?,
            "2",
            "C.7 delete is version 2",
        )?;

        // Logical-delete post-condition (master08 §delete): the VERSIONED_OBJECT
        // is logically deleted. Retrieving the current COMPOSITION reports it gone
        // — `composition_get` declares 204_deleted_at_time / 404 for a deleted or
        // absent current version.
        let obj = ids::object_uid(&v1);
        let after = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!("/ehr/{ehr_id}/composition/{obj}")),
                Format::Json,
            ))
            .await?;
        assert::status_in(&after, &[204, 404])?;
        Ok(DataSetReport::SINGLE)
    })
}

/// C.8 — a second commit with invalid content is rejected; exactly one VERSION
/// remains (the current COMPOSITION is still version 1).
fn run_two_commits_second_invalid<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        support::ensure_opt(ctx, "template.valid", "minimal/minimal_admin.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let created = commit_req(
            ctx,
            &ehr_id,
            &load(
                "contribution.valid",
                "minimal/minimal_admin.contribution.json",
            )?,
        )
        .await?;
        assert::status(&created, 201)?;
        let v1 = version_uid_in(&created.json()?, 0)?;

        let mut modify = load(
            "contribution.valid",
            "minimal/minimal_admin.contribution.modification.complete.json",
        )?;
        set_preceding(&mut modify, 0, &v1);
        if let Some(data) = modify["versions"][0]["data"].as_object_mut() {
            data.remove("composer"); // mandatory → invalid content
        }
        let resp = commit_req(ctx, &ehr_id, &modify).await?;
        assert_invalid(ctx, &resp, "C.8 second commit invalid content")?;

        // Post-condition (master08 §two_commits_second_invalid): only one VERSION
        // remains — the current COMPOSITION is still version 1.
        let obj = ids::object_uid(&v1);
        let after = ctx
            .send(negotiate::accept(
                HttpRequest::get(format!("/ehr/{ehr_id}/composition/{obj}")),
                Format::Json,
            ))
            .await?;
        assert::status(&after, 200)?;
        assert_version_number(
            &ids::body_uid(&after.json()?)?,
            "1",
            "C.8 only one COMPOSITION version remains",
        )?;
        Ok(DataSetReport::SINGLE)
    })
}

/// C.9 — a second commit with `change_type=creation` on the existing object is
/// rejected (only one 'create' per object).
fn run_two_commits_second_creation<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        support::ensure_opt(ctx, "template.valid", "minimal/minimal_admin.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let created = commit_req(
            ctx,
            &ehr_id,
            &load(
                "contribution.valid",
                "minimal/minimal_admin.contribution.json",
            )?,
        )
        .await?;
        assert::status(&created, 201)?;
        let v1 = version_uid_in(&created.json()?, 0)?;

        let mut second = load(
            "contribution.valid",
            "minimal/minimal_admin.contribution.modification.complete.json",
        )?;
        set_preceding(&mut second, 0, &v1);
        second["versions"][0]["commit_audit"]["change_type"] = change_type("249", "creation");
        let resp = commit_req(ctx, &ehr_id, &second).await?;
        // master08 states only "negative"; the cause (a duplicate 'create') is not
        // pinned to one contract code, so assert membership in contribution_create's
        // declared negative set {400, 409} (ruling: assert only the spec-determined
        // part when the schedule underdetermines).
        assert::status_in(&resp, NEGATIVE_SET)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── commit_contribution: EHR_STATUS ──────────────────────────────────────────

/// Which `subject.external_ref` shape a matrix row uses (master08 §`EHR_STATUS`
/// Accepted Cases).
#[derive(Debug, Clone, Copy)]
enum ExtRef {
    /// `PARTY_REF.id` typed `HIER_OBJECT_ID`.
    Hier,
    /// `PARTY_REF.id` typed `GENERIC_ID`.
    Generic,
    /// No external ref (anonymous subject).
    Null,
}

/// The 15-row `EHR_STATUS` accepted matrix, verbatim from master08 §`EHR_STATUS`
/// Accepted Cases (lines listing `is_modifiable | is_queryable |
/// subject.external_ref` — including the schedule's own duplicated `false|true`
/// rows). `is_modifiable × is_queryable × {HIER_OBJECT_ID, GENERIC_ID, NULL}`.
const STATUS_MATRIX: [(bool, bool, ExtRef); 15] = [
    (true, true, ExtRef::Hier),
    (true, true, ExtRef::Generic),
    (true, true, ExtRef::Null),
    (true, false, ExtRef::Hier),
    (true, false, ExtRef::Generic),
    (true, false, ExtRef::Null),
    (false, true, ExtRef::Hier),
    (false, true, ExtRef::Generic),
    (false, true, ExtRef::Null),
    (false, true, ExtRef::Hier),
    (false, true, ExtRef::Generic),
    (false, true, ExtRef::Null),
    (false, false, ExtRef::Hier),
    (false, false, ExtRef::Generic),
    (false, false, ExtRef::Null),
];

/// Build a `PARTY_REF` with a unique id of the given RM type.
fn party_ref(id_type: &str, unique: &str) -> Value {
    let mut id = json!({ "_type": id_type, "value": unique });
    if id_type == "GENERIC_ID" {
        id["scheme"] = Value::String("id_scheme".to_owned());
    }
    json!({
        "_type": "PARTY_REF",
        "namespace": "conformance",
        "type": "PERSON",
        "id": id
    })
}

/// Apply a matrix row to an `EHR_STATUS` node (`versions[0].data`): set
/// `is_modifiable`/`is_queryable` and the `subject.external_ref` shape.
fn apply_status_row(data: &mut Value, row: (bool, bool, ExtRef), unique: &str) {
    data["is_modifiable"] = json!(row.0);
    data["is_queryable"] = json!(row.1);
    let mut subject = json!({ "_type": "PARTY_SELF" });
    match row.2 {
        ExtRef::Hier => subject["external_ref"] = party_ref("HIER_OBJECT_ID", unique),
        ExtRef::Generic => subject["external_ref"] = party_ref("GENERIC_ID", unique),
        ExtRef::Null => {}
    }
    data["subject"] = subject;
}

/// Build a full `EHR_STATUS` (`subject.external_ref` populated) for the scenario-2
/// precondition, adapted to the RM 1.2.0 wire from a vendored valid fixture.
fn full_status() -> Result<Value, CaseError> {
    let base = fixtures::ehr_valid()
        .map_err(|e| codec(&e))?
        .into_iter()
        .next()
        .ok_or_else(|| CaseError::Skipped("no ehr-status.valid fixture available".to_owned()))?
        .json()
        .map_err(|e| codec(&e))?;
    Ok(fixtures::adapt_ehr_status(
        base,
        "conformance-ctb",
        &Uuid::new_v4().to_string(),
    ))
}

/// Drive the 15-row `EHR_STATUS` accepted matrix, each row a valid modification →
/// 201. `full` selects the precondition scenario: scenario 2 (EHR created by
/// providing a full `EHR_STATUS`) vs scenario 1 (default `EHR_STATUS`).
async fn run_status_matrix(ctx: &RunContext<'_>, full: bool) -> Result<DataSetReport, CaseError> {
    let mut passed: u32 = 0;
    for (i, row) in STATUS_MATRIX.iter().enumerate() {
        let ehr_id = if full {
            create_ehr_with_status(ctx, &full_status()?).await?
        } else {
            support::create_ehr(ctx).await?
        };
        let current = ehr_status_uid(ctx, &ehr_id).await?;
        let mut body = load(
            "contribution.valid",
            "minimal/status.contribution.modification.json",
        )?;
        normalize_modification(&mut body, 0);
        set_preceding(&mut body, 0, &current);
        apply_status_row(
            &mut body["versions"][0]["data"],
            *row,
            &Uuid::new_v4().to_string(),
        );
        let resp = commit_req(ctx, &ehr_id, &body).await?;
        assert::status(&resp, 201).map_err(|e| {
            CaseError::Assertion(format!(
                "EHR_STATUS matrix row {i} (modifiable={}, queryable={}, ref={:?}): {e}",
                row.0, row.1, row.2
            ))
        })?;
        passed += 1;
    }
    Ok(DataSetReport {
        passed,
        total: u32::try_from(STATUS_MATRIX.len()).unwrap_or(u32::MAX),
        schedule_rows: Some(u32::try_from(STATUS_MATRIX.len()).unwrap_or(u32::MAX)),
    })
}

/// D.1 — the accepted `EHR_STATUS` matrix over the default (scenario-1) status.
fn run_minimal_ehr_status<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_status_matrix(ctx, false).await })
}

/// D.2 — the accepted `EHR_STATUS` matrix over a provided full status (scenario 2):
/// the EHR is created WITH a full `EHR_STATUS`, the distinct precondition master08
/// draws (scenario D.2 — the legacy runner reused D.1 verbatim).
fn run_full_ehr_status<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_status_matrix(ctx, true).await })
}

/// D.3 — `change_type=creation` on an EHR that already has an `EHR_STATUS` is
/// rejected (STATUS cannot be created again, nor deleted).
fn run_ehr_status_invalid_change_type<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let mut body = load(
            "contribution.valid",
            "minimal/status.contribution.modification.json",
        )?;
        if let Some(v) = body["versions"][0].as_object_mut() {
            v.remove("preceding_version_uid");
        }
        body["versions"][0]["commit_audit"]["change_type"] = change_type("249", "creation");
        let resp = commit_req(ctx, &ehr_id, &body).await?;
        // master08 §EHR_STATUS Reject 1 states only "rejected"; the cause
        // (change_type on an existing STATUS) is not pinned to one contract code,
        // so assert contribution_create's declared negative set {400, 409}. The
        // `deleted` half of the reject rule is a boundary the schedule tabulates
        // but which cannot be expressed on the default STATUS over the wire.
        assert::status_in(&resp, NEGATIVE_SET)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// D.4 — a modification with invalid `EHR_STATUS` content is rejected.
fn run_invalid_ehr_status<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let current = ehr_status_uid(ctx, &ehr_id).await?;
        let mut body = load(
            "contribution.valid",
            "minimal/status.contribution.modification.json",
        )?;
        normalize_modification(&mut body, 0);
        set_preceding(&mut body, 0, &current);
        if let Some(data) = body["versions"][0]["data"].as_object_mut() {
            data.remove("is_queryable"); // mandatory EHR_STATUS attribute (RM ehr master04)
        }
        let resp = commit_req(ctx, &ehr_id, &body).await?;
        assert_invalid(ctx, &resp, "D.4 invalid EHR_STATUS (is_queryable removed)")?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── commit_contribution: FOLDER (directory) ──────────────────────────────────

/// E.1 — commit a valid FOLDER (creation) to an EHR with no directory → 201;
/// the CONTRIBUTION is retrievable.
fn run_valid_directory<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let body = load(
            "contribution.valid",
            "minimal/folder.contribution.creation.json",
        )?;
        let resp = commit_req(ctx, &ehr_id, &body).await?;
        assert::status(&resp, 201)?;
        let ctb_uid = ids::contribution_uid(ctx, &resp)?;
        let got = get_contribution(ctx, &ehr_id, &ctb_uid).await?;
        assert::status(&got, 200)?;
        if got.json()?["_type"] != "CONTRIBUTION" {
            return Err(CaseError::Assertion(
                "post-condition: expected CONTRIBUTION".to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE)
    })
}

/// E.2 — creating a directory when one already exists is a conflict.
fn run_fail_create_existing_directory<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let create = load(
            "contribution.valid",
            "minimal/folder.contribution.creation.json",
        )?;
        let first = commit_req(ctx, &ehr_id, &create).await?;
        assert::status(&first, 201)?;
        let second = commit_req(ctx, &ehr_id, &create).await?;
        assert_conflict(ctx, &second, "E.2 create directory when one already exists")?;
        Ok(DataSetReport::SINGLE)
    })
}

/// E.3 — modifying a directory that does not exist is rejected.
fn run_fail_modify_non_existing_directory<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let mut body = load(
            "contribution.valid",
            "minimal/folder.contribution.modification.json",
        )?;
        normalize_modification(&mut body, 0);
        // A syntactically valid but nonexistent preceding OBJECT_VERSION_ID,
        // derived from an observed id (the EHR's own default EHR_STATUS version)
        // so the SUT's real system id is reused — no literal.
        let observed = ids::parse_object_version_id(&ehr_status_uid(ctx, &ehr_id).await?)?;
        set_preceding(&mut body, 0, &support::nonexistent_version_like(&observed));
        let resp = commit_req(ctx, &ehr_id, &body).await?;
        // Cause (modify a non-existent target) not pinned to one contract code by
        // master08 → the declared negative set {400, 409}.
        assert::status_in(&resp, NEGATIVE_SET)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// E.4 — modify/amend an existing directory → new FOLDER version (version 2).
fn run_update_existing_directory<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let create = load(
            "contribution.valid",
            "minimal/folder.contribution.creation.json",
        )?;
        let created = commit_req(ctx, &ehr_id, &create).await?;
        assert::status(&created, 201)?;
        let v1 = version_uid_in(&created.json()?, 0)?;

        let mut modify = load(
            "contribution.valid",
            "minimal/folder.contribution.modification.json",
        )?;
        normalize_modification(&mut modify, 0);
        set_preceding(&mut modify, 0, &v1);
        let updated = commit_req(ctx, &ehr_id, &modify).await?;
        assert::status(&updated, 201)?;
        assert_version_number(
            &version_uid_in(&updated.json()?, 0)?,
            "2",
            "E.4 directory update is version 2",
        )?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── get_contribution / has_contribution ──────────────────────────────────────

/// `GET /ehr/{id}/contribution/{uid}` (JSON).
async fn get_contribution(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    uid: &str,
) -> Result<HttpResponse, CaseError> {
    ctx.send(negotiate::accept(
        HttpRequest::get(format!("/ehr/{ehr_id}/contribution/{uid}")),
        Format::Json,
    ))
    .await
}

/// H.1 / G.1 — an existing CONTRIBUTION retrieves 200 with `_type` CONTRIBUTION.
fn run_get_existing<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        support::ensure_opt(ctx, "template.valid", "minimal/minimal_admin.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let created = commit_req(
            ctx,
            &ehr_id,
            &load(
                "contribution.valid",
                "minimal/minimal_admin.contribution.json",
            )?,
        )
        .await?;
        assert::status(&created, 201)?;
        let uid = ids::contribution_uid(ctx, &created)?;
        let resp = get_contribution(ctx, &ehr_id, &uid).await?;
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

/// H.2 / G.2 — a random uid on a fresh EHR is not found.
fn run_get_empty_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = get_contribution(ctx, &ehr_id, &Uuid::new_v4().to_string()).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// H.3 / G.3 — a random EHR + uid is not found.
fn run_get_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = get_contribution(
            ctx,
            &Uuid::new_v4().to_string(),
            &Uuid::new_v4().to_string(),
        )
        .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

/// H.4 / G.4 — a random uid on an EHR that holds a real CONTRIBUTION → 404.
fn run_get_bad_contribution<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        support::ensure_opt(ctx, "template.valid", "minimal/minimal_admin.opt").await?;
        let ehr_id = support::create_ehr(ctx).await?;
        let created = commit_req(
            ctx,
            &ehr_id,
            &load(
                "contribution.valid",
                "minimal/minimal_admin.contribution.json",
            )?,
        )
        .await?;
        assert::status(&created, 201)?;
        let resp = get_contribution(ctx, &ehr_id, &Uuid::new_v4().to_string()).await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── list_contributions — skip-with-reason (no ITS-REST binding) ──────────────

const LIST_SKIP: &str = "master08 §list_contributions: the SM operation I_EHR_CONTRIBUTION.list_contributions() \
    has no ITS-REST binding — /ehr/{ehr_id}/contribution is POST-only (no GET collection resource) in the \
    tested development@e8a093e OAS and in Release-1.0.3; the list is a native-API concern, not wire-exercisable";

fn run_skip_list<'a>(_ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    Box::pin(async move { Err::<DataSetReport, _>(CaseError::Skipped(LIST_SKIP.to_owned())) })
}
