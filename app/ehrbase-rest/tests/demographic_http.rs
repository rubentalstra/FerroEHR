//! End-to-end HTTP tests for the DEMOGRAPHIC group: the `demographic` routes
//! served through the real [`DemographicService`] over a real `PostgreSQL`, with
//! `ETag`/`Location`/`Prefer` and the deleted-read→`204` and precondition→`412`
//! behaviour mirroring the EHR group.
//!
//! The former `Mock` backend served fixed party bodies with a hard-coded
//! `PARTY_OVID`; the real service assigns each version its own
//! `OBJECT_VERSION_ID`, so the tests create real parties through the wire and
//! read the server-assigned `version_uid` back from the `ETag` — the invariant
//! assertions (weak-`ETag` present, `Location` = `{base}/demographic/{kind}/{uid}`
//! consistent with that `ETag`, body `_type`, status codes) are unchanged.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use ehrbase::config::auth::AuthConfig;
use ehrbase::config::server::ServerConfig;
use ehrbase_rest::config::AppConfig;

mod common;

const BASE: &str = "/ehrbase/rest/openehr/v1";

/// A spec-valid PERSON body (the shape `service_demographic.rs` commits).
fn person_body() -> Value {
    json!({
        "_type": "PERSON",
        "archetype_node_id": "openEHR-DEMOGRAPHIC-PERSON.person.v1",
        "name": { "_type": "DV_TEXT", "value": "Jane Doe" },
        "identities": [{
            "_type": "PARTY_IDENTITY",
            "archetype_node_id": "at0001",
            "name": { "_type": "DV_TEXT", "value": "legal name" },
            "details": {
                "_type": "ITEM_TREE",
                "archetype_node_id": "at0002",
                "name": { "_type": "DV_TEXT", "value": "structure" },
                "items": [{
                    "_type": "ELEMENT",
                    "archetype_node_id": "at0003",
                    "name": { "_type": "DV_TEXT", "value": "family" },
                    "value": { "_type": "DV_TEXT", "value": "Doe" }
                }]
            }
        }]
    })
}

/// A spec-valid ROLE body (needs a `performer` `PARTY_REF`; no `capabilities`).
fn role_body() -> Value {
    json!({
        "_type": "ROLE",
        "archetype_node_id": "openEHR-DEMOGRAPHIC-ROLE.role.v1",
        "name": { "_type": "DV_TEXT", "value": "clinician" },
        "identities": [{
            "_type": "PARTY_IDENTITY",
            "archetype_node_id": "at0001",
            "name": { "_type": "DV_TEXT", "value": "r" },
            "details": {
                "_type": "ITEM_TREE",
                "archetype_node_id": "at0002",
                "name": { "_type": "DV_TEXT", "value": "structure" },
                "items": []
            }
        }],
        "performer": {
            "_type": "PARTY_REF", "namespace": "demographic", "type": "PERSON",
            "id": { "_type": "HIER_OBJECT_ID", "value": "cccccccc-cccc-4ccc-8ccc-cccccccccccc" }
        }
    })
}

fn relationship_body() -> Value {
    json!({
        "_type": "PARTY_RELATIONSHIP",
        "archetype_node_id": "openEHR-DEMOGRAPHIC-PARTY_RELATIONSHIP.relationship.v1",
        "name": { "_type": "DV_TEXT", "value": "parent-of" },
        "source": { "_type": "PARTY_REF", "namespace": "demographic", "type": "PERSON",
                    "id": { "_type": "HIER_OBJECT_ID", "value": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa" } },
        "target": { "_type": "PARTY_REF", "namespace": "demographic", "type": "PERSON",
                    "id": { "_type": "HIER_OBJECT_ID", "value": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb" } }
    })
}

fn config() -> AppConfig {
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
        ..Default::default()
    }
}

async fn app(db: &str) -> (common::Pg, Router) {
    let (pg, service) = common::test_service(db).await;
    (pg, common::router_with(config(), service))
}

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, header::HeaderMap, String) {
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

fn etag(h: &header::HeaderMap) -> Option<&str> {
    h.get(header::ETAG).and_then(|v| v.to_str().ok())
}

fn location(h: &header::HeaderMap) -> Option<&str> {
    h.get(header::LOCATION).and_then(|v| v.to_str().ok())
}

/// The bare uid inside a weak `ETag` (`W/"{uid}"`).
fn etag_uid(h: &header::HeaderMap) -> String {
    etag(h)
        .expect("ETag present")
        .trim_start_matches("W/")
        .trim_matches('"')
        .to_owned()
}

/// Create a party of `kind` (segment) and return its `version_uid` (`OVID`).
async fn create(app: &Router, seg: &str, body: &Value) -> String {
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/demographic/{seg}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let (status, h, _b) = send(app, req).await;
    assert_eq!(status, StatusCode::CREATED, "create {seg}");
    etag_uid(&h)
}

/// The bare versioned-object uuid from an `OBJECT_VERSION_ID`.
fn vo_of(ovid: &str) -> &str {
    ovid.split("::").next().expect("vo uuid")
}

#[tokio::test]
async fn person_create_default_is_minimal_with_headers() {
    let (_pg, app) = app("dem_person_create_min").await;
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/demographic/person"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(person_body().to_string()))
        .unwrap();
    let (status, h, body) = send(&app, req).await;

    // 201 default (return=minimal): headers only, no body.
    assert_eq!(status, StatusCode::CREATED);
    let uid = etag_uid(&h);
    assert!(uid.ends_with("::1"), "first version_uid, got {uid}");
    assert_eq!(etag(&h), Some(format!("W/\"{uid}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/demographic/person/{uid}").as_str())
    );
    assert!(body.is_empty(), "minimal create has no body, got {body:?}");
}

#[tokio::test]
async fn person_create_representation_returns_body() {
    let (_pg, app) = app("dem_person_create_repr").await;
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/demographic/person"))
        .header(header::CONTENT_TYPE, "application/json")
        .header("Prefer", "return=representation")
        .body(Body::from(person_body().to_string()))
        .unwrap();
    let (status, h, body) = send(&app, req).await;

    assert_eq!(status, StatusCode::CREATED);
    let uid = etag_uid(&h);
    assert_eq!(etag(&h), Some(format!("W/\"{uid}\"").as_str()));
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "PERSON");
}

#[tokio::test]
async fn person_get_sets_etag_and_location() {
    let (_pg, app) = app("dem_person_get").await;
    let ovid = create(&app, "person", &person_body()).await;
    let vo = vo_of(&ovid);

    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/demographic/person/{vo}"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(etag(&h), Some(format!("W/\"{ovid}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/demographic/person/{ovid}").as_str())
    );
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "PERSON");
}

#[tokio::test]
async fn deleted_person_read_is_204() {
    let (_pg, app) = app("dem_person_deleted_204").await;
    let ovid = create(&app, "person", &person_body()).await;
    let vo = vo_of(&ovid);

    // Delete the party (preceding version in the path).
    let del = Request::builder()
        .method("DELETE")
        .uri(format!("{BASE}/demographic/person/{ovid}"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, _b) = send(&app, del).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Reading the (now deleted) current version → Null body → 204.
    let get = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/demographic/person/{vo}"))
        .body(Body::empty())
        .unwrap();
    let (status, _h, body) = send(&app, get).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_empty());
}

#[tokio::test]
async fn person_delete_is_204_with_headers() {
    let (_pg, app) = app("dem_person_delete").await;
    let ovid = create(&app, "person", &person_body()).await;

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("{BASE}/demographic/person/{ovid}"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    // ETag + Location of the deleted version (a new version_uid).
    let deleted = etag_uid(&h);
    assert_eq!(etag(&h), Some(format!("W/\"{deleted}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/demographic/person/{deleted}").as_str())
    );
    assert!(body.is_empty());
}

/// The versioned-object-uid delete shape (ECC-DEM-005 family): the path is the
/// bare `HIER_OBJECT_ID` and the preceding version is carried by `If-Match`.
#[tokio::test]
async fn person_delete_by_versioned_uid_with_if_match_is_204() {
    let (_pg, app) = app("dem_person_delete_ifmatch").await;
    let ovid = create(&app, "person", &person_body()).await;
    let vo = vo_of(&ovid);

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("{BASE}/demographic/person/{vo}"))
        .header(header::IF_MATCH, format!("\"{ovid}\""))
        .body(Body::empty())
        .unwrap();
    let (status, _h, body) = send(&app, req).await;

    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_empty());
}

#[tokio::test]
async fn stale_update_is_412_with_latest_headers() {
    let (_pg, app) = app("dem_person_stale_412").await;
    let ovid = create(&app, "person", &person_body()).await;
    let vo = vo_of(&ovid);
    // A syntactically valid but stale OBJECT_VERSION_ID (same VO, wrong version).
    let stale = format!("{vo}::ehrbase-rs.local::9");

    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/demographic/person/{vo}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, format!("\"{stale}\""))
        .body(Body::from(person_body().to_string()))
        .unwrap();
    let (status, h, _body) = send(&app, req).await;

    // Precondition failure → 412, decorated with the latest version headers.
    assert_eq!(status, StatusCode::PRECONDITION_FAILED);
    assert_eq!(etag(&h), Some(format!("W/\"{ovid}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/demographic/person/{ovid}").as_str())
    );
}

#[tokio::test]
async fn role_create_uses_role_segment() {
    // The 5× kind fan-out routes each kind to its own segment.
    let (_pg, app) = app("dem_role_create").await;
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/demographic/role"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(role_body().to_string()))
        .unwrap();
    let (status, h, _body) = send(&app, req).await;
    assert_eq!(status, StatusCode::CREATED);
    let uid = etag_uid(&h);
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/demographic/role/{uid}").as_str())
    );
}

#[tokio::test]
async fn party_relationship_create_is_mounted_with_headers() {
    // The our-own-design PARTY_RELATIONSHIP extension route is mounted and
    // reaches the seam (a create returns 201 + ETag/Location on the
    // /demographic/party_relationship segment; an unmounted route would 404).
    let (_pg, app) = app("dem_rel_create").await;
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/demographic/party_relationship"))
        .header(header::CONTENT_TYPE, "application/json")
        .header("Prefer", "return=representation")
        .body(Body::from(relationship_body().to_string()))
        .unwrap();
    let (status, h, body) = send(&app, req).await;

    assert_eq!(status, StatusCode::CREATED);
    let uid = etag_uid(&h);
    assert_eq!(etag(&h), Some(format!("W/\"{uid}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/demographic/party_relationship/{uid}").as_str())
    );
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "PARTY_RELATIONSHIP");
}

#[tokio::test]
async fn party_relationship_get_is_mounted() {
    let (_pg, app) = app("dem_rel_get").await;
    let ovid = create(&app, "party_relationship", &relationship_body()).await;
    let vo = vo_of(&ovid);

    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/demographic/party_relationship/{vo}"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(&app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(etag(&h), Some(format!("W/\"{ovid}\"").as_str()));
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "PARTY_RELATIONSHIP");
}

#[tokio::test]
async fn versioned_party_relationship_is_mounted() {
    // RE-TARGET: the old Mock returned a blanket `501` (route mounted, unbuilt).
    // The versioned-party-relationship read is a real implementation now, so a
    // created relationship reads back its VERSIONED_PARTY_RELATIONSHIP → `200`
    // (an unmounted path would 404), which still proves the route is mounted.
    let (_pg, app) = app("dem_versioned_rel").await;
    let ovid = create(&app, "party_relationship", &relationship_body()).await;
    let vo = vo_of(&ovid);

    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "{BASE}/demographic/versioned_party_relationship/{vo}"
        ))
        .body(Body::empty())
        .unwrap();
    let (status, _h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("json body");
    // The ehr-less relationship version spine serves the base RM `VERSIONED_OBJECT`
    // (owner_id references the relationship's own versioned object) — the
    // extension route's own design (relationship.rs PORT NOTE G-6).
    assert_eq!(v["_type"], "VERSIONED_OBJECT");
    assert_eq!(v["owner_id"]["type"], "PARTY_RELATIONSHIP");
}

/// The ITS-REST overview committal-header merge requirement holds on the
/// demographic wire exactly as on the EHR APIs: attributes supplied via
/// `openEHR-AUDIT_DETAILS.*` request headers "MUST be merged with the default
/// `VERSION` and `VERSION.audit_details` attributes on commit runtime". The
/// persisted `ORIGINAL_VERSION`'s `commit_audit` must reflect the caller's
/// description and committer (`change_type` stays operation-owned).
#[tokio::test]
async fn demographic_committal_headers_merge_into_the_commit() {
    let (_pg, app) = app("dem_committal_merge").await;

    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/demographic/person"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            "openEHR-AUDIT_DETAILS.description",
            "value=\"Registered at intake\"",
        )
        .header(
            "openEHR-AUDIT_DETAILS.committer",
            "name=\"John Doe\", external_ref.id=\"BC8132EA-8F4A-11E7-BB31-BE2E44B06B34\", \
             external_ref.namespace=\"demographic\", external_ref.type=\"PERSON\"",
        )
        .body(Body::from(person_body().to_string()))
        .unwrap();
    let (status, h, _b) = send(&app, req).await;
    assert_eq!(status, StatusCode::CREATED);
    let v1 = etag_uid(&h);
    let vo = vo_of(&v1).to_owned();

    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "{BASE}/demographic/versioned_party/{vo}/version/{v1}"
        ))
        .header(header::ACCEPT, "application/json")
        .body(Body::empty())
        .unwrap();
    let (status, _h, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK, "version read: {body}");
    let ver: Value = serde_json::from_str(&body).expect("original_version json");
    assert_eq!(ver["_type"], "ORIGINAL_VERSION");
    let audit = &ver["commit_audit"];
    assert_eq!(
        audit["description"]["value"], "Registered at intake",
        "openEHR-AUDIT_DETAILS.description merged: {ver}"
    );
    assert_eq!(
        audit["committer"]["name"], "John Doe",
        "openEHR-AUDIT_DETAILS.committer merged"
    );
    assert_eq!(
        audit["change_type"]["defining_code"]["code_string"], "249",
        "change_type stays operation-owned (creation)"
    );
}
