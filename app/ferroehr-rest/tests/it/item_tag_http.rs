//! End-to-end HTTP tests for the `ITEM_TAG` surface, driven through the
//! assembled router over a **real** `FerroEhrService` on a real `PostgreSQL` 18
//! (auth disabled), via `tower::ServiceExt::oneshot` in-process requests.
//!
//! Three behaviours live here, each with its refusal AND its accepting twin:
//!
//! * **Write-schema strictness** — `schemas/common/UpdateItemTag.yaml` is
//!   `additionalProperties: false` over exactly `key`/`value`/`target_path`
//!   with `key` required. An undeclared member, a non-string `value` and a
//!   non-string `target_path` are each refused `400`; the status is register
//!   AMB-205 (the release declares the constraint and assigns no status to
//!   violating it).
//! * **FOLDER wrapper tags** — ITS-REST overview `Requests_and_responses.md`
//!   §openehr-item-tag and openehr-version-item-tag names FOLDER among the
//!   change-controlled resources the headers associate tags with, and the
//!   release defines no dedicated `/directory/…/tags` operations, so a FOLDER
//!   tag is set through the wrapper and read through the EHR-wide listing.
//! * **Validate-first** — a wrapper-header tag that breaks an RM `ITEM_TAG`
//!   invariant refuses the write with NOTHING committed (register AMB-204).
//!
//! Every assertion is strict-spec: where the implementation disagrees the test
//! is meant to FAIL (the code is fixed, never the test).
#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this integration \
              module's helpers and async bodies; panicking assertions and direct \
              fixture indexing are the intended shape here (the Rust Book ch11)"
)]

use axum::Router;
use axum::body::Body;
use http::{HeaderMap, Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use crate::common;

const BASE: &str = "/ferroehr/rest/openehr/v1";
const FOLDER_ARCHETYPE: &str = "openEHR-EHR-FOLDER.directory.v1";

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, HeaderMap, String) {
    let resp = app.clone().oneshot(req).await.expect("response");
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

/// The version uid carried by a weak `ETag`.
fn etag_uid(h: &HeaderMap) -> String {
    h.get(header::ETAG)
        .and_then(|v| v.to_str().ok())
        .expect("ETag")
        .trim_start_matches("W/")
        .trim_matches('"')
        .to_owned()
}

async fn create_ehr(app: &Router) -> String {
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(app, req).await;
    assert_eq!(status, StatusCode::CREATED, "create ehr: {body}");
    etag_uid(&h)
}

fn folder(name: &str) -> Value {
    json!({
        "_type": "FOLDER",
        "name": { "_type": "DV_TEXT", "value": name },
        "archetype_node_id": FOLDER_ARCHETYPE,
    })
}

/// `POST /ehr/{id}/directory`, optionally carrying an `openehr-item-tag`
/// wrapper header.
async fn create_directory(
    app: &Router,
    ehr: &str,
    item_tag: Option<&str>,
) -> (StatusCode, HeaderMap, String) {
    let mut b = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr/{ehr}/directory"))
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(tag) = item_tag {
        b = b.header("openehr-item-tag", tag);
    }
    send(app, b.body(Body::from(folder("root").to_string())).unwrap()).await
}

/// The EHR-wide `ITEM_TAG` listing (`GET /ehr/{ehr_id}/tags`).
async fn ehr_tags(app: &Router, ehr: &str) -> Vec<Value> {
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{ehr}/tags"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, body) = send(app, req).await;
    assert_eq!(status, StatusCode::OK, "ehr tags: {body}");
    serde_json::from_str::<Value>(&body)
        .expect("json ITEM_TAG list")
        .as_array()
        .expect("ITEM_TAG list")
        .clone()
}

/// `GET /ehr/{ehr_id}/directory` — used to prove a refused create committed
/// nothing.
async fn directory_status(app: &Router, ehr: &str) -> StatusCode {
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{ehr}/directory"))
        .body(Body::empty())
        .unwrap();
    send(app, req).await.0
}

/// `PUT` an `ITEM_TAG` list onto the `EHR_STATUS` container (the tag surface
/// reachable without a template upload).
async fn put_status_tags(
    app: &Router,
    ehr: &str,
    status_vo: &str,
    body: &Value,
) -> (StatusCode, String) {
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{ehr}/ehr_status/{status_vo}/tags"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let (status, _h, text) = send(app, req).await;
    (status, text)
}

/// The `EHR_STATUS` `VERSIONED_OBJECT` id of a freshly created EHR.
async fn status_vo(app: &Router, ehr: &str) -> String {
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{ehr}/ehr_status"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(app, req).await;
    assert_eq!(status, StatusCode::OK, "ehr_status read: {body}");
    let uid = etag_uid(&h);
    uid.split("::").next().expect("object id").to_owned()
}

// ── UpdateItemTag write-schema strictness ────────────────────────────────────

/// The released write schema declares exactly three members and
/// `additionalProperties: false`, so an undeclared member is REFUSED — never
/// silently dropped. Register AMB-205 assigns the `400`.
#[tokio::test]
async fn a_tag_put_with_an_undeclared_member_is_refused() {
    let (_pg, app) = common::test_router().await;
    let ehr = create_ehr(&app).await;
    let vo = status_vo(&app, &ehr).await;

    for undeclared in ["target", "owner_id", "_type", "colour"] {
        let body = json!([{ "key": "reviewed", undeclared: "anything" }]);
        let (status, text) = put_status_tags(&app, &ehr, &vo, &body).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "UpdateItemTag.yaml is additionalProperties:false, so {undeclared:?} must be \
             refused, not ignored: {text}"
        );
        assert!(
            text.contains(undeclared),
            "the refusal must name the offending member {undeclared:?}: {text}"
        );
    }
    // …and nothing was stored by any of the refused requests.
    assert!(
        ehr_tags(&app, &ehr).await.is_empty(),
        "a refused tag PUT stores nothing"
    );
}

/// A NON-STRING `value` used to be read as ABSENT — a silent loss of clinical
/// annotation. It is now the same refusal, with the accepting twin beside it.
#[tokio::test]
async fn a_tag_put_with_a_non_string_value_is_refused_and_the_string_twin_is_stored() {
    let (_pg, app) = common::test_router().await;
    let ehr = create_ehr(&app).await;
    let vo = status_vo(&app, &ehr).await;

    for wrong in [json!(42), json!(true), json!(["a"]), json!({ "v": 1 })] {
        let body = json!([{ "key": "reviewed", "value": wrong }]);
        let (status, text) = put_status_tags(&app, &ehr, &vo, &body).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "`value` is declared `type: string`; {wrong} must be refused, never read as \
             absent: {text}"
        );
    }
    assert!(
        ehr_tags(&app, &ehr).await.is_empty(),
        "a refused tag PUT stores nothing"
    );

    // The accepting twin: the same tag with a string value is stored, value intact.
    let (status, text) = put_status_tags(
        &app,
        &ehr,
        &vo,
        &json!([{ "key": "reviewed", "value": "42" }]),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "the valid twin: {text}");
    let tags = ehr_tags(&app, &ehr).await;
    assert_eq!(tags.len(), 1, "{tags:?}");
    assert_eq!(tags[0]["key"], "reviewed");
    assert_eq!(tags[0]["value"], "42");
}

/// A non-string `target_path` is worse than a dropped value: `target_path` is
/// half the `ITEM_TAG` IDENTITY (the (`key`, `target_path`) pair), so reading
/// it as absent silently stores the tag under a different identity than the
/// client asked for — and the client's later delete then addresses nothing.
#[tokio::test]
async fn a_tag_put_with_a_non_string_target_path_is_refused_and_the_string_twin_is_stored() {
    let (_pg, app) = common::test_router().await;
    let ehr = create_ehr(&app).await;
    let vo = status_vo(&app, &ehr).await;

    let body = json!([{ "key": "flag", "target_path": 7 }]);
    let (status, text) = put_status_tags(&app, &ehr, &vo, &body).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "`target_path` is declared `type: string` and is half the ITEM_TAG identity: {text}"
    );
    assert!(
        ehr_tags(&app, &ehr).await.is_empty(),
        "a refused tag PUT stores nothing"
    );

    // The accepting twin keeps the path verbatim (stored opaque — RM admits
    // both an AQL and an RM path there, with no discriminator).
    let (status, text) = put_status_tags(
        &app,
        &ehr,
        &vo,
        &json!([{ "key": "flag", "target_path": "/subject/external_ref/id/value" }]),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "the valid twin: {text}");
    let tags = ehr_tags(&app, &ehr).await;
    assert_eq!(tags.len(), 1, "{tags:?}");
    assert_eq!(tags[0]["target_path"], "/subject/external_ref/id/value");
}

/// The demographic family runs the SAME schema and must refuse identically —
/// the drift this pins is a family-specific reader, which is exactly what the
/// two seams used to have.
#[tokio::test]
async fn the_demographic_tag_put_is_strict_in_the_same_way() {
    let (_pg, app) = common::test_router().await;

    let person = json!({
        "_type": "PERSON",
        "name": { "_type": "DV_TEXT", "value": "PERSON" },
        "archetype_node_id": "openEHR-DEMOGRAPHIC-PERSON.person.v1",
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": { "_type": "ARCHETYPE_ID",
                              "value": "openEHR-DEMOGRAPHIC-PERSON.person.v1" },
            "rm_version": "1.1.0"
        },
        "identities": [{
            "_type": "PARTY_IDENTITY",
            "name": { "_type": "DV_TEXT", "value": "legal identity" },
            "archetype_node_id": "at0002",
            "details": {
                "_type": "ITEM_TREE",
                "name": { "_type": "DV_TEXT", "value": "tree" },
                "archetype_node_id": "at0003",
                "items": [{
                    "_type": "ELEMENT",
                    "name": { "_type": "DV_TEXT", "value": "name" },
                    "archetype_node_id": "at0004",
                    "value": { "_type": "DV_TEXT", "value": "Jane Doe" }
                }]
            }
        }]
    });
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/demographic/person"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(person.to_string()))
        .unwrap();
    let (status, h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::CREATED, "create person: {body}");
    let vo = etag_uid(&h)
        .split("::")
        .next()
        .expect("object id")
        .to_owned();

    for bad in [
        json!([{ "key": "reviewed", "owner_id": "x" }]),
        json!([{ "key": "reviewed", "value": 42 }]),
        json!([{ "key": "reviewed", "target_path": 7 }]),
    ] {
        let req = Request::builder()
            .method("PUT")
            .uri(format!("{BASE}/demographic/person/{vo}/tags"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(bad.to_string()))
            .unwrap();
        let (status, _h, text) = send(&app, req).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "the demographic seam runs the SAME UpdateItemTag schema: {bad} -> {text}"
        );
    }

    // The accepting twin, including the `target_path: ""` normalization the
    // register fixes identically on both families (AMB-96: "" folds to absent).
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/demographic/person/{vo}/tags"))
        .header(header::CONTENT_TYPE, "application/json")
        .header("Prefer", "return=representation")
        .body(Body::from(
            json!([{ "key": "reviewed", "value": "true", "target_path": "" }]).to_string(),
        ))
        .unwrap();
    let (status, _h, text) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK, "the valid twin: {text}");
    let tags: Value = serde_json::from_str(&text).expect("json ITEM_TAG list");
    assert_eq!(tags[0]["key"], "reviewed");
    assert!(
        tags[0].get("target_path").is_none(),
        "an empty target_path normalizes to ABSENT on the demographic family too \
         (register AMB-96): {text}"
    );
}

// ── FOLDER wrapper tags ──────────────────────────────────────────────────────

/// FOLDER is one of the change-controlled resource types the wrapper-header
/// section names, so a DIRECTORY commit may carry `openehr-item-tag`. The
/// release defines no `/directory/…/tags` operations, so the stored tag is read
/// back through the EHR-wide listing — the aggregate defined over "any target
/// VERSION or `VERSIONED_OBJECT` within the EHR".
#[tokio::test]
async fn a_folder_may_be_tagged_through_the_wrapper_header() {
    let (_pg, app) = common::test_router().await;
    let ehr = create_ehr(&app).await;

    let (status, h, body) =
        create_directory(&app, &ehr, Some(r#"key="folder-scope",value="clinical""#)).await;
    assert_eq!(status, StatusCode::CREATED, "create directory: {body}");
    let version_uid = etag_uid(&h);
    let container = version_uid.split("::").next().expect("object id");

    // The echo confirms the stored list (a MAY, honoured here).
    let echo = h
        .get("openehr-item-tag")
        .and_then(|v| v.to_str().ok())
        .expect("the wrapper echo");
    assert!(
        echo.contains(r#"key="folder-scope""#) && echo.contains(r#"value="clinical""#),
        "the echo confirms the stored list: {echo}"
    );

    // …and the EHR-wide listing is the truth: one tag, on the VERSIONED_FOLDER.
    let tags = ehr_tags(&app, &ehr).await;
    assert_eq!(tags.len(), 1, "{tags:?}");
    assert_eq!(tags[0]["key"], "folder-scope");
    assert_eq!(tags[0]["value"], "clinical");
    assert_eq!(
        tags[0]["target"]["value"], container,
        "the tag targets the VERSIONED_FOLDER container: {tags:?}"
    );

    // The UPDATE leg carries the header too — §Usage in Requests binds it to
    // "creation or update operations (`PUT`, `POST`)" — and, being a whole-list
    // replace, it REPLACES the container's collection.
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{ehr}/directory"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, format!("\"{version_uid}\""))
        .header("openehr-item-tag", r#"key="folder-scope",value="research""#)
        .body(Body::from(folder("root").to_string()))
        .unwrap();
    let (status, h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "update directory: {body}");
    let echo = h
        .get("openehr-item-tag")
        .and_then(|v| v.to_str().ok())
        .expect("the wrapper echo");
    assert!(echo.contains(r#"value="research""#), "the echo: {echo}");
    let tags = ehr_tags(&app, &ehr).await;
    assert_eq!(
        tags.len(),
        1,
        "a whole-list replace, not an append: {tags:?}"
    );
    assert_eq!(tags[0]["value"], "research");
}

/// The DIRECTORY family serves no dedicated tag routes, and must not grow any:
/// the release defines none — the twenty-three tag operations cover COMPOSITION,
/// `EHR_STATUS` and the five party kinds only — and a server may not close that
/// gap by inventing them. (`/directory/tags` is refused because `tags` is read
/// as the `version_uid` segment of `directory_get_by_version_id`, which is the
/// point: there is no tag route there to match.)
#[tokio::test]
async fn no_dedicated_directory_tag_routes_exist() {
    let (_pg, app) = common::test_router().await;
    let ehr = create_ehr(&app).await;
    let (status, _h, body) = create_directory(&app, &ehr, None).await;
    assert_eq!(status, StatusCode::CREATED, "create directory: {body}");

    for (method, path) in [
        ("GET", format!("{BASE}/ehr/{ehr}/directory/tags")),
        ("PUT", format!("{BASE}/ehr/{ehr}/directory/tags")),
        (
            "DELETE",
            format!("{BASE}/ehr/{ehr}/directory/tags/reviewed"),
        ),
    ] {
        let req = Request::builder()
            .method(method)
            .uri(&path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("[]"))
            .unwrap();
        let (status, _h, body) = send(&app, req).await;
        assert!(
            status.is_client_error(),
            "the release defines no {method} {path}; got {status}: {body}"
        );
        assert!(
            serde_json::from_str::<Value>(&body)
                .ok()
                .is_none_or(|v| !v.is_array()),
            "no tag COLLECTION may be served at {path}; got {body}"
        );
    }
}

// ── validate-first: a refused tag list commits no content ────────────────────

/// A wrapper-header tag that breaks `ITEM_TAG.Inv_key_valid` refuses the write
/// BEFORE the content commits, so no version exists afterwards (register
/// AMB-204). The alternative — refusing after the commit — answers `4xx` for a
/// request whose VERSION is durable and whose response carries no `ETag` and no
/// `Location`, so the client's only recovery is to re-POST and duplicate
/// clinical content.
#[tokio::test]
async fn an_invalid_wrapper_tag_refuses_before_the_content_commits() {
    let (_pg, app) = common::test_router().await;
    let ehr = create_ehr(&app).await;

    let (status, h, body) = create_directory(&app, &ehr, Some(r#"key=" padded ""#)).await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "ITEM_TAG.Inv_key_valid: a key may not carry surrounding whitespace: {body}"
    );
    assert!(
        h.get(header::ETAG).is_none() && h.get(header::LOCATION).is_none(),
        "a refused write creates nothing, so it identifies nothing"
    );
    assert_eq!(
        directory_status(&app, &ehr).await,
        StatusCode::NOT_FOUND,
        "NO version was created: the tag list is judged before the content commit"
    );

    // The accepting twin: the same commit with a well-formed tag succeeds.
    let (status, _h, body) = create_directory(&app, &ehr, Some(r#"key="padded""#)).await;
    assert_eq!(status, StatusCode::CREATED, "the valid twin: {body}");
    assert_eq!(directory_status(&app, &ehr).await, StatusCode::OK);
}

/// A wrapper header whose entry carries no `key` is refused too — `key` is the
/// one REQUIRED member of the operation the header wraps, so the wrapper cannot
/// admit what the operation refuses (register AMB-203).
#[tokio::test]
async fn a_keyless_wrapper_entry_refuses_the_write() {
    let (_pg, app) = common::test_router().await;
    let ehr = create_ehr(&app).await;

    let (status, _h, body) = create_directory(&app, &ehr, Some(r#"value="orphan""#)).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "a keyless wrapper entry must be refused, never skipped: {body}"
    );
    assert_eq!(
        directory_status(&app, &ehr).await,
        StatusCode::NOT_FOUND,
        "NO version was created"
    );
}
