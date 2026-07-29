#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! End-to-end HTTP tests for the two ADMIN **extension** groups — the SM
//! activity report (`/admin/report/*`, `I_ADMIN_SERVICE` statistics) and the SM
//! archive pair (`/admin/archive/*`, `I_ADMIN_ARCHIVE`) — driven through the
//! assembled router over a real `EhrbaseService` on a real `PostgreSQL`.
//!
//! **No openEHR spec governs these routes** (the released Admin API is exactly
//! `admin_ehr_delete` + `admin_ehr_delete_all`); the operation SEMANTICS come
//! from `docs/specs/openehr/SM/docs/UML/classes/i_admin_service.adoc` and
//! `i_admin_archive.adoc`, and the wire shape is our own design/extension. What
//! is asserted here is exactly what the CNF extension bindings drive:
//! `admin-activity-report` and `admin-archive` in
//! `tools/cnf-runner/artifacts/vocab/wire_surface.yaml`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;

use ehrbase::config::auth::AuthConfig;
use ehrbase::config::server::{AdminConfig, ServerConfig};
use ehrbase_rest::config::AppConfig;

mod common;

const BASE: &str = "/ehrbase/rest/openehr/v1";
/// A syntactically valid id that is never created — the "unknown" probe.
const ABSENT: &str = "00000000-0000-0000-0000-000000000000";

fn config(admin_enabled: bool) -> AppConfig {
    AppConfig {
        server: ServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            base_path: BASE.to_owned(),
            max_in_flight: 1024,
            swagger_ui: false,
            cors_permissive: false,
            ..Default::default()
        },
        auth: AuthConfig {
            enabled: false,
            basic: None,
            oidc: None,
            admin_scope: None,
            ..AuthConfig::default()
        },
        admin: AdminConfig {
            enabled: admin_enabled,
        },
        ..Default::default()
    }
}

async fn app(admin_enabled: bool) -> (testkit::TestDb, Router) {
    let (pg, service) = common::test_service().await;
    (pg, common::router_with(config(admin_enabled), service))
}

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, String) {
    let resp = app.clone().oneshot(req).await.expect("response");
    let status = resp.status();
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn get(path: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(format!("{BASE}{path}"))
        .body(Body::empty())
        .expect("request")
}

fn post_json(path: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!("{BASE}{path}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .expect("request")
}

/// Create an EHR through the released wire and return its id.
async fn create_ehr(app: &Router) -> String {
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr"))
        .header("Prefer", "return=representation")
        .body(Body::empty())
        .expect("request");
    let (status, body) = send(app, req).await;
    assert_eq!(status, StatusCode::CREATED, "EHR create: {body}");
    let value: serde_json::Value = serde_json::from_str(&body).expect("EHR json");
    value["ehr_id"]["value"]
        .as_str()
        .expect("ehr_id")
        .to_owned()
}

// ── the activity report (SM I_ADMIN_SERVICE statistics) ──────────────────────

/// The four reporting routes answer `200` with the SM return shape: a list of
/// CONTRIBUTION ids, and three bare `Integer` counts. An EHR created through
/// the released wire commits one CONTRIBUTION, so the counts are observable
/// rather than trivially zero.
#[tokio::test]
async fn the_activity_report_counts_what_the_released_wire_committed() {
    let (_pg, app) = app(true).await;

    let (status, body) = send(&app, get("/admin/report/contribution?a_service=Ehr")).await;
    assert_eq!(status, StatusCode::OK, "empty-server list: {body}");
    assert_eq!(body, "[]", "an empty server has committed nothing");

    let _ehr_id = create_ehr(&app).await;

    let (status, body) = send(&app, get("/admin/report/contribution?a_service=Ehr")).await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<String> = serde_json::from_str(&body).expect("id list");
    assert_eq!(ids.len(), 1, "the EHR create committed one CONTRIBUTION");

    let (status, body) = send(&app, get("/admin/report/contribution/count?a_service=Ehr")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "1", "the count is the bare SM Integer");

    // COMPOSITIONs are EHR-scoped and none was committed, so both COMPOSITION
    // counters are zero — and a service that is not a versioned-content
    // service reports zero for everything.
    for path in [
        "/admin/report/versioned_composition/count?a_service=Ehr",
        "/admin/report/composition_version/count?a_service=Ehr",
        "/admin/report/contribution/count?a_service=Query",
    ] {
        let (status, body) = send(&app, get(path)).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(body, "0", "{path}");
    }
}

/// The SM `Interval<Iso8601_date_time>` filter reaches the query: a range that
/// cannot contain the commit answers zero, an open-ended one containing it
/// answers one.
#[tokio::test]
async fn the_report_time_interval_filters_the_commit_window() {
    let (_pg, app) = app(true).await;
    let _ehr_id = create_ehr(&app).await;

    let (status, body) = send(
        &app,
        get("/admin/report/contribution/count?a_service=Ehr&time_interval=2000-01-01T00:00:00Z/2000-12-31T00:00:00Z"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "0", "the commit is outside the range");

    let (status, body) = send(
        &app,
        get("/admin/report/contribution/count?a_service=Ehr&time_interval=2000-01-01T00:00:00Z/"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "1", "an open upper bound contains the commit");
}

/// The two parameter refusals: `a_service` outside the SM `PLATFORM_SERVICE`
/// enumeration, and a `time_interval` that is not `<lower>/<upper>`.
#[tokio::test]
async fn the_report_refuses_an_unknown_service_and_a_malformed_interval() {
    let (_pg, app) = app(true).await;

    for path in [
        "/admin/report/contribution",
        "/admin/report/contribution?a_service=Terminology",
        "/admin/report/contribution?a_service=Ehr&time_interval=2020-01-01T00:00:00Z",
        "/admin/report/contribution?a_service=Ehr&time_interval=not-a-date/",
    ] {
        let (status, body) = send(&app, get(path)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}: {body}");
    }
}

/// A `time_interval` bounded on both sides with `lower > upper` is no
/// `Interval` at all — BASE
/// `docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.foundation_types.interval.adoc`
/// §Invariants, `Limits_consistent`: `(not upper_unbounded and not
/// lower_unbounded) implies lower <= upper`. The SM parameter IS an
/// `Interval<Iso8601_date_time>` (`i_admin_service.adoc`), so the value
/// violates its own type and the call is refused rather than answered with the
/// empty result an inverted range would select. All four reporting routes share
/// the one boundary parser, so all four refuse.
#[tokio::test]
async fn the_report_refuses_an_interval_whose_lower_bound_is_after_its_upper() {
    let (_pg, app) = app(true).await;

    let inverted = "time_interval=2026-12-31T00:00:00Z/2020-01-01T00:00:00Z";
    for route in [
        "/admin/report/contribution",
        "/admin/report/contribution/count",
        "/admin/report/versioned_composition/count",
        "/admin/report/composition_version/count",
    ] {
        let path = format!("{route}?a_service=Ehr&{inverted}");
        let (status, body) = send(&app, get(&path)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}: {body}");
    }

    // The equal-bounds pair satisfies `lower <= upper` and is a legitimate
    // (closed, single-instant) interval — the refusal must not swallow it.
    let (status, body) = send(
        &app,
        get("/admin/report/contribution/count\
             ?a_service=Ehr&time_interval=2026-01-01T00:00:00Z/2026-01-01T00:00:00Z"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "equal bounds: {body}");
}

// ── the archive pair (SM I_ADMIN_ARCHIVE) ────────────────────────────────────

/// `archive_ehrs` marks the named EHRs archived and answers `204`; archival is
/// read-neutral, so the EHR stays retrievable through the released wire.
/// Re-archiving is idempotent.
#[tokio::test]
async fn archiving_an_ehr_is_read_neutral_and_idempotent() {
    let (_pg, app) = app(true).await;
    let ehr_id = create_ehr(&app).await;

    let body = format!(r#"{{"ehr_ids":["{ehr_id}"]}}"#);
    for _ in 0..2 {
        let (status, reply) = send(&app, post_json("/admin/archive/ehrs", &body)).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "archive: {reply}");
    }

    let (status, _) = send(&app, get(&format!("/ehr/{ehr_id}"))).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "archival is a marker, not a deletion — the EHR must stay retrievable"
    );
}

/// The all-or-nothing refusals of both halves: an unknown id is `404` (SM
/// `ehr_id_does_not_exist` / `party_id_does_not_exist`) and a malformed one is
/// `400`, with nothing archived either way. A body that is not
/// `{ "<field>": [ … ] }` is `400` too.
#[tokio::test]
async fn the_archive_routes_refuse_unknown_malformed_and_shapeless_requests() {
    let (_pg, app) = app(true).await;

    for (path, field) in [
        ("/admin/archive/ehrs", "ehr_ids"),
        ("/admin/archive/parties", "party_ids"),
    ] {
        let (status, body) = send(
            &app,
            post_json(path, &format!(r#"{{"{field}":["{ABSENT}"]}}"#)),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path} unknown id: {body}");

        let (status, body) = send(
            &app,
            post_json(path, &format!(r#"{{"{field}":["not-a-uuid"]}}"#)),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "{path} malformed id: {body}"
        );

        let (status, body) = send(&app, post_json(path, r#"{"wrong":[]}"#)).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "{path} wrong shape: {body}"
        );

        // An EMPTY list archives nothing and succeeds — the SM parameter is a
        // list, and an empty one names no target to fail on.
        let (status, body) = send(&app, post_json(path, &format!(r#"{{"{field}":[]}}"#))).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "{path} empty list: {body}");
    }
}

// ── the dump/load pair (SM I_ADMIN_DUMP_LOAD) ────────────────────────────────

/// A unique archive location for one test (the SM `file_sys_loc` parameter is
/// a location on the SERVER's file system, which in-process is this one).
fn archive_dir() -> String {
    std::env::temp_dir()
        .join(format!("ehrbase-cnf-dump-{}", uuid::Uuid::now_v7()))
        .to_string_lossy()
        .into_owned()
}

/// `export_ehrs` writes the archive and answers `200` with the SM
/// `List<DUMP_LOAD_FAIL_REPORT>` — EMPTY when every EHR was dumped. Requesting
/// the `zip` `COMPRESSION_FORMAT` member packs the same archive into one
/// container, and `load_ehrs` — which takes only `file_sys_loc` — reads either
/// container back.
#[tokio::test]
async fn dump_and_load_round_trip_in_both_containers() {
    for compression in [None, Some("zip")] {
        let (_pg, app) = app(true).await;
        let _ehr = create_ehr(&app).await;
        let dir = archive_dir();

        let mut spec = serde_json::json!({
            "file_sys_loc": dir,
            "logical_format": "openehr_canonical_json",
            "segment_split_size": 1024
        });
        if let Some(member) = compression {
            spec["compression_format"] = serde_json::Value::String(member.to_owned());
        }
        let (status, body) = send(&app, post_json("/admin/dump", &spec.to_string())).await;
        assert_eq!(status, StatusCode::OK, "dump {compression:?}: {body}");
        assert_eq!(body, "[]", "a clean dump reports no failures");

        // Loading the archive back into the SAME repository is the SM's own
        // documented duplicate case ("import EHRs with duplicate EHR ids will
        // fail"): the call succeeds and reports the entity, never a fatal.
        let (status, body) = send(
            &app,
            post_json("/admin/load", &format!(r#"{{"file_sys_loc":{dir:?}}}"#)),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "load {compression:?}: {body}");
        let reports: serde_json::Value = serde_json::from_str(&body).expect("report json");
        let list = reports.as_array().expect("a JSON array");
        assert_eq!(list.len(), 1, "the one duplicate EHR is reported: {body}");
        assert_eq!(list[0]["entity_type"], "EHR");
        assert_eq!(list[0]["dump_status"], false);
        assert!(list[0]["error"].is_string(), "with an explanatory error");

        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// The request-shape refusals, each SM `precondition_violation` → `400`:
/// an absent/blank `file_sys_loc`, a value naming no enumeration member, a
/// non-integer `segment_split_size`, and `encoding` — whose `ENCODING_FORMAT`
/// enumeration (`encoding_format.adoc`) declares NO members, so no value is
/// representable.
#[tokio::test]
async fn dump_refuses_every_unrepresentable_request_shape() {
    let (_pg, app) = app(true).await;
    let dir = archive_dir();

    for (label, body) in [
        (
            "no location",
            serde_json::json!({ "segment_split_size": 1024 }),
        ),
        (
            "blank location",
            serde_json::json!({ "file_sys_loc": "   " }),
        ),
        (
            "unknown logical format",
            serde_json::json!({ "file_sys_loc": dir, "logical_format": "canonical_json" }),
        ),
        (
            "unknown compression format",
            serde_json::json!({ "file_sys_loc": dir, "compression_format": "gzip" }),
        ),
        (
            "encoding (empty enumeration)",
            serde_json::json!({ "file_sys_loc": dir, "encoding": "utf_8" }),
        ),
        (
            "non-integer split size",
            serde_json::json!({ "file_sys_loc": dir, "segment_split_size": "big" }),
        ),
        (
            "non-positive split size",
            serde_json::json!({ "file_sys_loc": dir, "segment_split_size": 0 }),
        ),
    ] {
        let (status, response) = send(&app, post_json("/admin/dump", &body.to_string())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{label}: {response}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A well-formed request for an enumeration member this service does not
/// realize is `501 Not Implemented` (RFC 9110 §15.6.2) — never a `400`, which
/// would call a valid SM value malformed.
#[tokio::test]
async fn dump_answers_unrealized_enumeration_members_with_not_implemented() {
    let (_pg, app) = app(true).await;
    let dir = archive_dir();

    // `7z` left this list 2026-07-29 (owner-approved realization; the
    // service round-trip test covers it) — the XML archive form is the one
    // remaining unrealized member (#670).
    for member in
        [serde_json::json!({ "file_sys_loc": dir, "logical_format": "openehr_canonical_xml" })]
    {
        let (status, body) = send(&app, post_json("/admin/dump", &member.to_string())).await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{member}: {body}");
    }
    // ... and the realized `7z` member succeeds on the same wire.
    let seven = serde_json::json!({ "file_sys_loc": dir, "compression_format": "7z" });
    let (status, body) = send(&app, post_json("/admin/dump", &seven.to_string())).await;
    assert_eq!(status, StatusCode::OK, "7z dump: {body}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Every route of all three extension groups inherits the ADMIN group's config
/// gate: with `admin.enabled` off they answer `405` with an empty `Allow`,
/// before the backend is touched.
#[tokio::test]
async fn the_admin_gate_covers_the_extension_groups() {
    let (_pg, app) = app(false).await;

    let (status, _) = send(&app, get("/admin/report/contribution?a_service=Ehr")).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);

    let (status, _) = send(&app, post_json("/admin/archive/ehrs", r#"{"ehr_ids":[]}"#)).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);

    let (status, _) = send(
        &app,
        post_json("/admin/dump", r#"{"file_sys_loc":"/tmp/never"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);

    let (status, _) = send(
        &app,
        post_json("/admin/load", r#"{"file_sys_loc":"/tmp/never"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}
