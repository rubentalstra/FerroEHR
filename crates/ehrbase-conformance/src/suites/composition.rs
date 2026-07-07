//! master07 — COMPOSITION cases (design §4.1: `suites/composition.rs`).
//!
//! Transcribed from `master07-func_tc_ehr_composition.adoc`, driving the ITS-REST
//! `/ehr/{ehr_id}/composition` + `/versioned_composition` surface. Positive cases
//! commit the vendored canonical compositions
//! (`compositions/CANONICAL_JSON` + `CANONICAL_XML`) after uploading the OPT they
//! reference (`nested.en.v1` → `valid_templates/nested/nested.opt`,
//! `persistent_minimal.en.v1` → `.../minimal_persistent/persistent_minimal.opt`);
//! negatives use the vendored `__invalid_wrong_structure` (malformed) and
//! `__invalid_opt_doesnt_exist` (references a missing OPT) fixtures. Assertions
//! concretize the ITS-REST composition contract from the operation specs
//! (`composition_create.yaml` 201/400/404/422; `composition_get.yaml`
//! 200/204/404; `composition_update.yaml` 200/400/404/412/422;
//! `composition_delete.yaml` 204/400/404/409; `versioned_composition_get.yaml`
//! 200/404).
//!
//! Event/persistent create + read round-trips run under **both** JSON and XML
//! (design: "run both `Format::Json` and `Format::Xml` where the case warrants");
//! the negatives, multi-version, update, and delete flows run JSON-only.
//!
//! The `has_composition{,-bad_composition,-bad_ehr}` schedule cases have no
//! dedicated ITS-REST endpoint on our surface (the API exposes GET
//! `composition/versioned_composition`, not a boolean `has`), so they stay
//! `NotYetTranscribed`.

use jiff::Timestamp;
use serde_json::Value;
use uuid::Uuid;

use openehr_rm::prelude::Composition;

use crate::assert;
use crate::case::{Capability, CaseMeta, Chapter, Compare, Format, Profile, Provenance};
use crate::fixtures;
use crate::harness::{
    CaseError, CaseFuture, CaseRun, DataSetReport, HttpRequest, Method, RunContext,
};
use crate::registry::CaseEntry;
use crate::suites::support;

/// The implemented master07 case entries.
#[must_use]
pub fn entries() -> Vec<CaseEntry> {
    vec![
        // ── create ───────────────────────────────────────────────────────────
        entry_fmt(
            "I_EHR_COMPOSITION.create_composition-event",
            BOTH,
            run_create_event,
        ),
        entry_fmt(
            "I_EHR_COMPOSITION.create_composition-persistent",
            BOTH,
            run_create_persistent,
        ),
        entry(
            "I_EHR_COMPOSITION.create_composition-same_opt_twice",
            run_create_same_opt_twice,
        ),
        entry(
            "I_EHR_COMPOSITION.create_composition-invalid_event",
            run_create_invalid_event,
        ),
        entry(
            "I_EHR_COMPOSITION.create_composition-invalid_persistent",
            run_create_invalid_persistent,
        ),
        entry(
            "I_EHR_COMPOSITION.create_composition-event_bad_opt",
            run_create_event_bad_opt,
        ),
        entry(
            "I_EHR_COMPOSITION.create_composition-event_bad_ehr",
            run_create_event_bad_ehr,
        ),
        // ── get latest ───────────────────────────────────────────────────────
        entry_fmt(
            "I_EHR_COMPOSITION.get_composition_latest",
            BOTH,
            run_get_latest,
        ),
        entry(
            "I_EHR_COMPOSITION.get_composition_latest-bad_composition",
            run_get_latest_bad_composition,
        ),
        entry(
            "I_EHR_COMPOSITION.get_composition_latest-bad_ehr",
            run_get_latest_bad_ehr,
        ),
        // ── get at time ──────────────────────────────────────────────────────
        entry_fmt(
            "I_EHR_COMPOSITION.get_composition_at_time-no_time_arg",
            BOTH,
            run_get_at_time_no_arg,
        ),
        entry(
            "I_EHR_COMPOSITION.get_composition_at_time-bad_composition",
            run_get_at_time_bad_composition,
        ),
        entry(
            "I_EHR_COMPOSITION.get_composition_at_time-bad_ehr",
            run_get_at_time_bad_ehr,
        ),
        entry(
            "I_EHR_COMPOSITION.get_composition_at_times",
            run_get_at_times,
        ),
        // ── get version ──────────────────────────────────────────────────────
        entry_fmt(
            "I_EHR_COMPOSITION.get_composition_version",
            BOTH,
            run_get_version,
        ),
        entry(
            "I_EHR_COMPOSITION.get_composition_version-bad_version",
            run_get_version_bad_version,
        ),
        entry(
            "I_EHR_COMPOSITION.get_composition_version-bad_ehr",
            run_get_version_bad_ehr,
        ),
        entry(
            "I_EHR_COMPOSITION.get_composition_versions",
            run_get_versions,
        ),
        // ── versioned composition ────────────────────────────────────────────
        entry_fmt(
            "I_EHR_COMPOSITION.get_versioned_composition",
            BOTH,
            run_get_versioned,
        ),
        entry(
            "I_EHR_COMPOSITION.get_versioned_composition-non_existent",
            run_get_versioned_non_existent,
        ),
        entry(
            "I_EHR_COMPOSITION.get_versioned_composition-bad_ehr",
            run_get_versioned_bad_ehr,
        ),
        // ── update ───────────────────────────────────────────────────────────
        entry(
            "I_EHR_COMPOSITION.update_composition-event",
            run_update_event,
        ),
        entry(
            "I_EHR_COMPOSITION.update_composition-persistent",
            run_update_persistent,
        ),
        entry(
            "I_EHR_COMPOSITION.update_composition-non_existent",
            run_update_non_existent,
        ),
        entry(
            "I_EHR_COMPOSITION.update_composition-wrong_template",
            run_update_wrong_template,
        ),
        // ── delete ───────────────────────────────────────────────────────────
        entry(
            "I_EHR_COMPOSITION.delete_composition-event",
            run_delete_event,
        ),
        entry(
            "I_EHR_COMPOSITION.delete_composition-persistent",
            run_delete_persistent,
        ),
        entry(
            "I_EHR_COMPOSITION.delete_composition-non_existent",
            run_delete_non_existent,
        ),
    ]
}

/// JSON-only formats.
const JSON: &[Format] = &[Format::Json];
/// Both canonical formats.
const BOTH: &[Format] = &[Format::Json, Format::Xml];

/// A JSON-only schedule-provenance master07 case.
fn entry(id: &'static str, run: CaseRun) -> CaseEntry {
    entry_fmt(id, JSON, run)
}

/// A schedule-provenance master07 case with the given formats.
fn entry_fmt(id: &'static str, formats: &'static [Format], run: CaseRun) -> CaseEntry {
    CaseEntry {
        meta: CaseMeta {
            id,
            chapter: Chapter::Master07,
            capability: Capability::CompositionOps,
            profiles: &[Profile::Core, Profile::Standard],
            formats,
            provenance: Provenance::Schedule,
            schedule_ref: id,
            upstream_tags: &[],
            compare: Compare::Superset,
        },
        run,
    }
}

// ── fixtures + helpers ───────────────────────────────────────────────────────

/// A COMPOSITION category, selecting the vendored fixture set + its OPT.
#[derive(Clone, Copy)]
enum Kind {
    /// An event `COMPOSITION` (`nested.en.v1`).
    Event,
    /// A persistent `COMPOSITION` (`persistent_minimal.en.v1`).
    Persistent,
}

impl Kind {
    /// The OPT to provision (relative to `valid_templates/`).
    fn opt(self) -> &'static str {
        match self {
            Kind::Event => "nested/nested.opt",
            Kind::Persistent => "minimal_persistent/persistent_minimal.opt",
        }
    }

    /// The canonical-JSON `__full` composition fixture.
    fn json_fixture(self) -> &'static str {
        match self {
            Kind::Event => "compositions/CANONICAL_JSON/nested.en.v1__full.json",
            Kind::Persistent => "compositions/CANONICAL_JSON/persistent_minimal.en.v1__full.json",
        }
    }

    /// The canonical-XML `__full` composition fixture.
    fn xml_fixture(self) -> &'static str {
        match self {
            Kind::Event => "compositions/CANONICAL_XML/nested.en.v1__full.xml",
            Kind::Persistent => "compositions/CANONICAL_XML/persistent_minimal.en.v1__full.xml",
        }
    }

    /// The `__invalid_wrong_structure` (malformed) fixture text for this kind.
    fn invalid_structure_fixture(self) -> &'static str {
        match self {
            Kind::Event => "compositions/CANONICAL_JSON/nested.en.v1__invalid_wrong_structure.json",
            Kind::Persistent => {
                "compositions/CANONICAL_JSON/persistent_minimal.en.v1__invalid_wrong_structure.json"
            }
        }
    }
}

fn codec(e: fixtures::FixtureError) -> CaseError {
    CaseError::Codec(e.to_string())
}

/// Commit a canonical composition for `kind` in the run's format (uploading the
/// OPT first), returning the raw response for the case to assert.
async fn commit(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    kind: Kind,
) -> Result<crate::harness::HttpResponse, CaseError> {
    support::ensure_opt(ctx, kind.opt()).await?;
    let req = match ctx.format {
        Format::Json => {
            let body = fixtures::read_json(kind.json_fixture()).map_err(codec)?;
            HttpRequest::post(format!("/ehr/{ehr_id}/composition"))
                .json_body(&body)?
                .header("accept", "application/json")
                .header("prefer", "return=representation")
        }
        Format::Xml => {
            let xml = fixtures::read(kind.xml_fixture()).map_err(codec)?;
            HttpRequest::post(format!("/ehr/{ehr_id}/composition"))
                .text_body(xml, "application/xml")
                .header("accept", "application/xml")
                .header("prefer", "return=representation")
        }
    };
    ctx.send(req).await
}

/// Create an EHR, provision the OPT, commit a `kind` composition, assert 201,
/// and return `(ehr_id, version_uid)`.
async fn setup(ctx: &RunContext<'_>, kind: Kind) -> Result<(String, String), CaseError> {
    let ehr_id = support::create_ehr(ctx).await?;
    let resp = commit(ctx, &ehr_id, kind).await?;
    assert::status(&resp, 201)?;
    let uid = support::version_uid(&resp)?;
    Ok((ehr_id, uid))
}

/// A canonical-JSON composition body for `kind` (for the update/put flows, which
/// are JSON-only).
fn json_body(kind: Kind) -> Result<Value, CaseError> {
    fixtures::read_json(kind.json_fixture()).map_err(codec)
}

/// PUT an updated `kind` composition against `object_uid`, precondition
/// `If-Match: precede`, returning the raw response.
async fn update(
    ctx: &RunContext<'_>,
    ehr_id: &str,
    object_uid: &str,
    precede: &str,
    kind: Kind,
) -> Result<crate::harness::HttpResponse, CaseError> {
    let body = json_body(kind)?;
    ctx.send(
        HttpRequest::put(format!("/ehr/{ehr_id}/composition/{object_uid}"))
            .json_body(&body)?
            .header("accept", "application/json")
            .header("prefer", "return=representation")
            .header("if-match", precede.to_owned()),
    )
    .await
}

/// Assert a retrieved composition body is present and, for JSON, is valid RM.
fn check_composition(
    ctx: &RunContext<'_>,
    resp: &crate::harness::HttpResponse,
) -> Result<(), CaseError> {
    if resp.body.is_empty() {
        return Err(CaseError::Assertion(
            "retrieved composition body is empty".to_owned(),
        ));
    }
    if ctx.format == Format::Json {
        openehr_its::json::from_canonical_json::<Composition>(&resp.text()).map_err(|e| {
            CaseError::Assertion(format!("retrieved composition is not valid RM: {e}"))
        })?;
    }
    Ok(())
}

/// Assert a version uid's version number is `n` (the `::<n>` suffix).
fn assert_version_number(uid: &str, n: u32) -> Result<(), CaseError> {
    let got = uid.rsplit("::").next().unwrap_or_default();
    if got == n.to_string() {
        Ok(())
    } else {
        Err(CaseError::Assertion(format!(
            "expected version number {n} in {uid:?}, got {got:?}"
        )))
    }
}

macro_rules! case {
    ($body:block) => {
        Box::pin(async move { $body })
    };
}

// ── create ───────────────────────────────────────────────────────────────────

fn run_create_event<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_create(ctx, Kind::Event).await })
}

fn run_create_persistent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_create(ctx, Kind::Persistent).await })
}

/// Create a new `kind` composition: 201 + ETag/Location; version number 1.
async fn run_create(ctx: &RunContext<'_>, kind: Kind) -> Result<DataSetReport, CaseError> {
    let ehr_id = support::create_ehr(ctx).await?;
    let resp = commit(ctx, &ehr_id, kind).await?;
    assert::status(&resp, 201)?;
    assert::header_present(&resp, "etag")?;
    assert::header_present(&resp, "location")?;
    let uid = support::version_uid(&resp)?;
    assert_version_number(&uid, 1)?;
    Ok(DataSetReport::SINGLE)
}

fn run_create_same_opt_twice<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // Only one 'create' is allowed for a persistent COMPOSITION; the second
        // create for the same persistent OPT must be a negative response
        // (schedule §create_composition-same_opt_twice).
        let ehr_id = support::create_ehr(ctx).await?;
        let first = commit(ctx, &ehr_id, Kind::Persistent).await?;
        assert::status(&first, 201)?;
        let second = commit(ctx, &ehr_id, Kind::Persistent).await?;
        support::assert_negative(&second)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_create_invalid_event<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_create_invalid(ctx, Kind::Event).await })
}

fn run_create_invalid_persistent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_create_invalid(ctx, Kind::Persistent).await })
}

/// Commit the vendored `__invalid_wrong_structure` fixture (malformed content);
/// the server must reject it (`composition_create.yaml` 400/422).
async fn run_create_invalid(ctx: &RunContext<'_>, kind: Kind) -> Result<DataSetReport, CaseError> {
    let ehr_id = support::create_ehr(ctx).await?;
    support::ensure_opt(ctx, kind.opt()).await?;
    let malformed = fixtures::read(kind.invalid_structure_fixture()).map_err(codec)?;
    let resp = ctx
        .send(
            HttpRequest::post(format!("/ehr/{ehr_id}/composition"))
                .text_body(malformed, "application/json")
                .header("accept", "application/json"),
        )
        .await?;
    support::assert_negative(&resp)?;
    Ok(DataSetReport::SINGLE)
}

fn run_create_event_bad_opt<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // The composition references a template ("not_exist") never uploaded;
        // the server must reject it (`composition_create.yaml` 404/422 — our
        // validation returns 422, see service_validation.rs).
        let ehr_id = support::create_ehr(ctx).await?;
        let body = fixtures::read_json(
            "compositions/CANONICAL_JSON/nested.en.v1__invalid_opt_doesnt_exist.json",
        )
        .map_err(codec)?;
        let resp = ctx
            .send(
                HttpRequest::post(format!("/ehr/{ehr_id}/composition"))
                    .json_body(&body)?
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status_in(&resp, &[404, 422])?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_create_event_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        support::ensure_opt(ctx, Kind::Event.opt()).await?;
        let body = json_body(Kind::Event)?;
        let resp = ctx
            .send(
                HttpRequest::post(format!("/ehr/{}/composition", Uuid::new_v4()))
                    .json_body(&body)?
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── get latest ───────────────────────────────────────────────────────────────

fn run_get_latest<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, uid) = setup(ctx, Kind::Event).await?;
        let object = support::object_uid(&uid).to_owned();
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/composition/{object}"))
                    .header("accept", ctx.format.media_type()),
            )
            .await?;
        assert::status(&resp, 200)?;
        check_composition(ctx, &resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_latest_bad_composition<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/composition/{}", Uuid::new_v4()))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_latest_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr/{}/composition/{}",
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

// ── get at time ──────────────────────────────────────────────────────────────

fn run_get_at_time_no_arg<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // No time argument → the latest version (identical to get latest).
        let (ehr_id, uid) = setup(ctx, Kind::Event).await?;
        let object = support::object_uid(&uid).to_owned();
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/composition/{object}"))
                    .header("accept", ctx.format.media_type()),
            )
            .await?;
        assert::status(&resp, 200)?;
        check_composition(ctx, &resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_at_time_bad_composition<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr/{ehr_id}/composition/{}?version_at_time=2030-01-01T00:00:00Z",
                    Uuid::new_v4()
                ))
                .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_at_time_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr/{}/composition/{}?version_at_time=2030-01-01T00:00:00Z",
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

fn run_get_at_times<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // Two versions committed at t0 < t1; probe three time points (schedule
        // §get_composition_at_times): before t0 → negative; between → v1; after
        // t1 → v2.
        let ehr_id = support::create_ehr(ctx).await?;
        support::ensure_opt(ctx, Kind::Event.opt()).await?;
        let first = commit(ctx, &ehr_id, Kind::Event).await?;
        assert::status(&first, 201)?;
        let uid1 = support::version_uid(&first)?;
        let object = support::object_uid(&uid1).to_owned();

        // A window strictly between the two commits.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let between = Timestamp::now();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let second = update(ctx, &ehr_id, &object, &uid1, Kind::Event).await?;
        assert::status_in(&second, &[200, 204])?;
        let uid2 = support::version_uid(&second)?;

        let mut passed = 0u32;

        // Before any version.
        let before = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr/{ehr_id}/composition/{object}?version_at_time=1900-01-01T00:00:00Z"
                ))
                .header("accept", "application/json"),
            )
            .await?;
        if [204, 400, 404].contains(&before.status) {
            passed += 1;
        }

        // Between t0 and t1 → v1.
        let mid = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr/{ehr_id}/composition/{object}?version_at_time={between}"
                ))
                .header("accept", "application/json"),
            )
            .await?;
        if mid.status == 200 && support::version_uid(&mid).is_ok_and(|u| u == uid1) {
            passed += 1;
        }

        // After t1 → v2 (latest).
        let after = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr/{ehr_id}/composition/{object}?version_at_time=2030-01-01T00:00:00Z"
                ))
                .header("accept", "application/json"),
            )
            .await?;
        if after.status == 200 && support::version_uid(&after).is_ok_and(|u| u == uid2) {
            passed += 1;
        }

        if passed == 3 {
            Ok(DataSetReport { passed, total: 3 })
        } else {
            Err(CaseError::Assertion(format!(
                "get_composition_at_times: only {passed}/3 time points resolved correctly"
            )))
        }
    })
}

// ── get version ──────────────────────────────────────────────────────────────

fn run_get_version<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, uid) = setup(ctx, Kind::Event).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/composition/{uid}"))
                    .header("accept", ctx.format.media_type()),
            )
            .await?;
        assert::status(&resp, 200)?;
        check_composition(ctx, &resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_version_bad_version<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let bogus = format!("{}::conformance::1", Uuid::new_v4());
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/composition/{bogus}"))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_version_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let bogus = format!("{}::conformance::1", Uuid::new_v4());
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{}/composition/{bogus}", Uuid::new_v4()))
                    .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_versions<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // Two VERSIONs v1, v2; each version id retrieves its own version.
        let ehr_id = support::create_ehr(ctx).await?;
        support::ensure_opt(ctx, Kind::Event.opt()).await?;
        let first = commit(ctx, &ehr_id, Kind::Event).await?;
        assert::status(&first, 201)?;
        let uid1 = support::version_uid(&first)?;
        let object = support::object_uid(&uid1).to_owned();
        let second = update(ctx, &ehr_id, &object, &uid1, Kind::Event).await?;
        assert::status_in(&second, &[200, 204])?;
        let uid2 = support::version_uid(&second)?;

        let mut passed = 0u32;
        for uid in [&uid1, &uid2] {
            let resp = ctx
                .send(
                    HttpRequest::get(format!("/ehr/{ehr_id}/composition/{uid}"))
                        .header("accept", "application/json"),
                )
                .await?;
            if resp.status == 200 && support::version_uid(&resp).is_ok_and(|u| &u == uid) {
                passed += 1;
            }
        }
        if passed == 2 {
            Ok(DataSetReport { passed, total: 2 })
        } else {
            Err(CaseError::Assertion(format!(
                "get_composition_versions: only {passed}/2 version ids resolved correctly"
            )))
        }
    })
}

// ── versioned composition ────────────────────────────────────────────────────

fn run_get_versioned<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let (ehr_id, uid) = setup(ctx, Kind::Event).await?;
        let object = support::object_uid(&uid).to_owned();
        let resp = ctx
            .send(
                HttpRequest::get(format!("/ehr/{ehr_id}/versioned_composition/{object}"))
                    .header("accept", ctx.format.media_type()),
            )
            .await?;
        assert::status(&resp, 200)?;
        if resp.body.is_empty() {
            return Err(CaseError::Assertion(
                "VERSIONED_COMPOSITION body is empty".to_owned(),
            ));
        }
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_versioned_non_existent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        let resp = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr/{ehr_id}/versioned_composition/{}",
                    Uuid::new_v4()
                ))
                .header("accept", "application/json"),
            )
            .await?;
        assert::status(&resp, 404)?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_get_versioned_bad_ehr<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let resp = ctx
            .send(
                HttpRequest::get(format!(
                    "/ehr/{}/versioned_composition/{}",
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

// ── update ───────────────────────────────────────────────────────────────────

fn run_update_event<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_update_ok(ctx, Kind::Event).await })
}

fn run_update_persistent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_update_ok(ctx, Kind::Persistent).await })
}

/// Create then update a `kind` composition → 200, version number 2.
async fn run_update_ok(ctx: &RunContext<'_>, kind: Kind) -> Result<DataSetReport, CaseError> {
    let (ehr_id, uid1) = setup(ctx, kind).await?;
    let object = support::object_uid(&uid1).to_owned();
    let resp = update(ctx, &ehr_id, &object, &uid1, kind).await?;
    assert::status_in(&resp, &[200, 204])?;
    let uid2 = support::version_uid(&resp)?;
    assert_version_number(&uid2, 2)?;
    Ok(DataSetReport::SINGLE)
}

fn run_update_non_existent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        support::ensure_opt(ctx, Kind::Event.opt()).await?;
        let object = Uuid::new_v4();
        let precede = format!("{}::conformance::1", Uuid::new_v4());
        let resp = update(ctx, &ehr_id, &object.to_string(), &precede, Kind::Event).await?;
        assert::status_in(&resp, &[404, 412])?;
        Ok(DataSetReport::SINGLE)
    })
}

fn run_update_wrong_template<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        // Update an event composition with a body referencing a *different*
        // template (persistent_minimal) — must be rejected (template_id mismatch).
        let (ehr_id, uid1) = setup(ctx, Kind::Event).await?;
        let object = support::object_uid(&uid1).to_owned();
        support::ensure_opt(ctx, Kind::Persistent.opt()).await?;
        let resp = update(ctx, &ehr_id, &object, &uid1, Kind::Persistent).await?;
        support::assert_negative(&resp)?;
        Ok(DataSetReport::SINGLE)
    })
}

// ── delete ───────────────────────────────────────────────────────────────────

fn run_delete_event<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_delete_ok(ctx, Kind::Event).await })
}

fn run_delete_persistent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({ run_delete_ok(ctx, Kind::Persistent).await })
}

/// Create then delete a `kind` composition → 204 (logical delete). The delete
/// path segment is the version uid to delete (`composition_delete.yaml`: the
/// `uid_based_id` MUST be the `OBJECT_VERSION_ID` of the most recent version).
async fn run_delete_ok(ctx: &RunContext<'_>, kind: Kind) -> Result<DataSetReport, CaseError> {
    let (ehr_id, uid1) = setup(ctx, kind).await?;
    let resp = ctx
        .send(HttpRequest::new(
            Method::Delete,
            format!("/ehr/{ehr_id}/composition/{uid1}"),
        ))
        .await?;
    assert::status_in(&resp, &[200, 204])?;
    Ok(DataSetReport::SINGLE)
}

fn run_delete_non_existent<'a>(ctx: &'a RunContext<'a>) -> CaseFuture<'a> {
    case!({
        let ehr_id = support::create_ehr(ctx).await?;
        // A well-formed but non-existent OBJECT_VERSION_ID → 404 (or 409 stale).
        let version = format!("{}::conformance::1", Uuid::new_v4());
        let resp = ctx
            .send(HttpRequest::new(
                Method::Delete,
                format!("/ehr/{ehr_id}/composition/{version}"),
            ))
            .await?;
        assert::status_in(&resp, &[404, 409, 412])?;
        Ok(DataSetReport::SINGLE)
    })
}
