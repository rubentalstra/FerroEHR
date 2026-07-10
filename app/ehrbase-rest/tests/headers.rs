//! End-to-end HTTP tests for the W2-A response-header + `Prefer` handling:
//! `ETag`/`Location` on the EHR / `EHR_STATUS` / COMPOSITION writes and reads, and
//! the `return=minimal` (default, header-only) vs `return=representation`
//! (full body) `Prefer` policy — driven through the assembled router with a
//! canned [`EhrService`] backend.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use ehrbase_rest::RestConfig;
use ehrbase_rest::access::authn::config::AuthConfig;

mod common;
use common::{Hooks, Mock};

const BASE: &str = "/ehrbase/rest/openehr/v1";
const EHR_ID: &str = "7d44b88c-4199-4bad-97dc-d78268e01398";
const STATUS_OVID: &str = "6cb19121-4307-4648-9da0-d62e4d51f19b::openEHRSys::2";
const COMP_OVID: &str = "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys::3";
/// A valid `HIER_OBJECT_ID` for the bare `composition_get` route: the EHR
/// dispatcher decodes `uid_based_id` before the read, so it must be a real UUID.
const COMP_VO: &str = "8849182c-82ad-4088-a07f-48ead4180515";

/// A canned platform that echoes fixed resources + metadata so the header/
/// `Prefer` wiring in `dispatch::ehr` is exercised without a database. The SM
/// `create`/`update` calls return only the `version_uid`; the resource body a
/// `return=representation` response re-reads comes from the paired `get_*` hook.
fn hooks() -> Hooks {
    let ehr_uuid: Uuid = EHR_ID.parse().expect("valid ehr uuid");
    Hooks {
        // create_ehr returns the fixed id; ehr_object supplies the representation.
        create_ehr: Some(Arc::new(move |_status| Ok(ehr_uuid))),
        ehr_object: Some(Arc::new(|_id| {
            Ok(json!({ "_type": "EHR", "ehr_id": { "_type": "HIER_OBJECT_ID", "value": EHR_ID } }))
        })),
        // EHR_STATUS update returns the new version_uid; get_ehr_status the body.
        replace_ehr_status: Some(Arc::new(|_id, _uv| Ok(STATUS_OVID.to_owned()))),
        get_ehr_status: Some(Arc::new(|_id| {
            Ok(json!({
                "_type": "EHR_STATUS",
                "uid": { "_type": "OBJECT_VERSION_ID", "value": STATUS_OVID },
                "subject": { "_type": "PARTY_SELF" }
            }))
        })),
        // 200_VERSION_at_time: an ORIGINAL_VERSION carrying the version_uid.
        ehr_status_version_at_time: Some(Arc::new(|_id, _t| {
            Ok(json!({
                "_type": "ORIGINAL_VERSION",
                "uid": { "_type": "OBJECT_VERSION_ID", "value": STATUS_OVID }
            }))
        })),
        // The bare COMPOSITION read: the ETag/Location come from the body's uid.
        get_composition_latest: Some(Arc::new(|_e, _vo| {
            Ok(json!({
                "_type": "COMPOSITION",
                "uid": { "_type": "OBJECT_VERSION_ID", "value": COMP_OVID },
                "name": { "_type": "DV_TEXT", "value": "Encounter" }
            }))
        })),
        // 204_COMPOSITION_deleted: the deleted version_uid.
        delete_composition: Some(Arc::new(|_e, _ovid| Ok(COMP_OVID.to_owned()))),
        ..Default::default()
    }
}

fn config() -> RestConfig {
    RestConfig {
        bind: "127.0.0.1:0".to_owned(),
        base_path: BASE.to_owned(),
        swagger_ui: false,
        cors_permissive: false,
        auth: AuthConfig {
            enabled: false,
            basic: None,
            oidc: None,
            admin_scope: None,
        },
        admin: ehrbase_rest::AdminConfig::default(),
        terminology: ehrbase_rest::TerminologyConfig::default(),
    }
}

fn app() -> Router {
    ehrbase_rest::build_with(config(), Arc::new(Mock::with(hooks()))).expect("router builds")
}

async fn send(req: Request<Body>) -> (StatusCode, header::HeaderMap, String) {
    let resp = app().oneshot(req).await.expect("response");
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

#[tokio::test]
async fn ehr_create_default_is_minimal_with_headers() {
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(req).await;

    // 201_EHR default (return=minimal): headers only, no body.
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(etag(&h), Some(format!("\"{EHR_ID}\"").as_str()));
    assert_eq!(location(&h), Some(format!("{BASE}/ehr/{EHR_ID}").as_str()));
    assert!(body.is_empty(), "minimal create has no body, got {body:?}");
}

#[tokio::test]
async fn ehr_create_representation_returns_body() {
    let req = Request::builder()
        .method("POST")
        .uri(format!("{BASE}/ehr"))
        .header("Prefer", "return=representation")
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(req).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(etag(&h), Some(format!("\"{EHR_ID}\"").as_str()));
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "EHR");
}

#[tokio::test]
async fn ehr_status_update_default_is_204_with_headers() {
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{EHR_ID}/ehr_status"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, "\"prev::v::1\"")
        .body(Body::from(
            r#"{"_type":"EHR_STATUS","subject":{"_type":"PARTY_SELF"}}"#,
        ))
        .unwrap();
    let (status, h, body) = send(req).await;

    // 204_EHR_STATUS (default minimal): no body, ETag + Location.
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(etag(&h), Some(format!("\"{STATUS_OVID}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/ehr/{EHR_ID}/ehr_status/{STATUS_OVID}").as_str())
    );
    assert!(body.is_empty());
}

#[tokio::test]
async fn ehr_status_update_representation_is_200_with_body() {
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{EHR_ID}/ehr_status"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, "\"prev::v::1\"")
        .header("Prefer", "return=representation")
        .body(Body::from(
            r#"{"_type":"EHR_STATUS","subject":{"_type":"PARTY_SELF"}}"#,
        ))
        .unwrap();
    let (status, h, body) = send(req).await;

    // 200_EHR_STATUS_updated (representation): body present.
    assert_eq!(status, StatusCode::OK);
    assert_eq!(etag(&h), Some(format!("\"{STATUS_OVID}\"").as_str()));
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "EHR_STATUS");
}

#[tokio::test]
async fn composition_get_sets_etag_and_location() {
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{EHR_ID}/composition/{COMP_VO}"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(req).await;

    // 200_COMPOSITION_retrieved: ETag(version_uid) + Location.
    assert_eq!(status, StatusCode::OK);
    assert_eq!(etag(&h), Some(format!("\"{COMP_OVID}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/ehr/{EHR_ID}/composition/{COMP_OVID}").as_str())
    );
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "COMPOSITION");
}

#[tokio::test]
async fn versioned_ehr_status_version_at_time_sets_version_headers() {
    // F-01-05: 200_VERSION_at_time declares ETag (the version_uid) + Location
    // (the …/versioned_ehr_status/version/{version_uid} VERSION resource URL).
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{EHR_ID}/versioned_ehr_status/version"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(etag(&h), Some(format!("\"{STATUS_OVID}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/ehr/{EHR_ID}/versioned_ehr_status/version/{STATUS_OVID}").as_str())
    );
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["_type"], "ORIGINAL_VERSION");
}

#[tokio::test]
async fn composition_delete_is_204_with_headers() {
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("{BASE}/ehr/{EHR_ID}/composition/{COMP_OVID}"))
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(req).await;

    // 204_COMPOSITION_deleted: ETag + Location of the deleted version.
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(etag(&h), Some(format!("\"{COMP_OVID}\"").as_str()));
    assert_eq!(
        location(&h),
        Some(format!("{BASE}/ehr/{EHR_ID}/composition/{COMP_OVID}").as_str())
    );
    assert!(body.is_empty());
}
