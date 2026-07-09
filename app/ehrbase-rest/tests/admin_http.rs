//! End-to-end HTTP tests for the ADMIN API group (physical EHR delete): the
//! config gate (`RestConfig::admin.enabled`), the `204`/`404`/`400`/`200`
//! wire outcomes, and the `{"deleted": n}` bulk body — driven through the
//! assembled router with a canned [`AdminService`] backend that records whether
//! it was consulted.
//!
//! Spec grounding: SM `I_ADMIN_SERVICE.physical_ehr_delete` and the CNF Robot
//! prior art (`CNF/tests/platform/robot/I_ADMIN_SERVICE/001-EHR.robot`:
//! `DELETE /admin/ehr/{ehr_id}` → `204`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use ehrbase_rest::auth::config::AuthConfig;
use ehrbase_rest::{
    AdminConfig, AdminService, DemographicService, EhrService, QueryService, RestConfig,
    WebTemplateService,
};
use openehr_its::rest::generated::definition::DefinitionApi;
use openehr_its::rest::runtime::ApiError;

const BASE: &str = "/ehrbase/rest/openehr/v1";
/// A known EHR the mock "deletes" successfully; anything else is `NotFound`.
const KNOWN: &str = "7d44b88c-4199-4bad-97dc-d78268e01398";
const OTHER: &str = "11111111-2222-3333-4444-555555555555";

/// A canned backend: `admin_ehr_delete` succeeds for [`KNOWN`] and 404s
/// otherwise; `admin_ehr_delete_all` reports the count of ids it was handed.
/// The `calls` counter proves the backend is (not) consulted.
#[derive(Debug, Default)]
struct MockBackend {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl AdminService for MockBackend {
    async fn admin_ehr_delete(&self, ehr_id: String) -> Result<(), ApiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if ehr_id == KNOWN {
            Ok(())
        } else {
            Err(ApiError::NotFound(format!("EHR {ehr_id}")))
        }
    }

    async fn admin_ehr_delete_all(&self, ehr_ids: Vec<String>) -> Result<u64, ApiError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ehr_ids.len() as u64)
    }
}

impl EhrService for MockBackend {}
impl ehrbase_rest::EhrStatusService for MockBackend {}
impl ehrbase_rest::EhrCompositionService for MockBackend {}
impl ehrbase_rest::EhrDirectoryService for MockBackend {}
impl ehrbase_rest::EhrContributionService for MockBackend {}
impl DefinitionApi for MockBackend {}
impl ehrbase_rest::DefinitionAdl14Service for MockBackend {}
impl ehrbase_rest::DefinitionAdl2Service for MockBackend {}
impl ehrbase_rest::DefinitionQueryService for MockBackend {}
impl ehrbase_rest::PartyRelationshipService for MockBackend {}
impl ehrbase_rest::EhrIndexService for MockBackend {}
impl WebTemplateService for MockBackend {}
impl QueryService for MockBackend {}
impl DemographicService for MockBackend {}

fn config(admin_enabled: bool) -> RestConfig {
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
        admin: AdminConfig {
            enabled: admin_enabled,
        },
    }
}

fn app(admin_enabled: bool) -> (Router, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let backend = Arc::new(MockBackend {
        calls: calls.clone(),
    });
    let router = ehrbase_rest::build_with(config(admin_enabled), backend).expect("router builds");
    (router, calls)
}

async fn send(app: Router, req: Request<Body>) -> (StatusCode, String) {
    let resp = app.oneshot(req).await.expect("response");
    let status = resp.status();
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn delete(uri: String) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn disabled_admin_is_404_and_never_touches_backend() {
    let (app, calls) = app(false);
    let (status, _) = send(app, delete(format!("{BASE}/admin/ehr/{KNOWN}"))).await;
    // The gate answers 404 as if the route were unmounted.
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "backend must not be called"
    );
}

#[tokio::test]
async fn enabled_delete_is_204() {
    let (app, calls) = app(true);
    let (status, body) = send(app, delete(format!("{BASE}/admin/ehr/{KNOWN}"))).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_empty(), "204 carries no body, got {body:?}");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn enabled_delete_unknown_maps_to_404() {
    let (app, _calls) = app(true);
    // The backend's NotFound (ehr_id_does_not_exist) surfaces as HTTP 404.
    let (status, _) = send(app, delete(format!("{BASE}/admin/ehr/{OTHER}"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn enabled_delete_all_returns_count() {
    let (app, _calls) = app(true);
    let (status, body) = send(
        app,
        delete(format!("{BASE}/admin/ehr/all?ehr_id={KNOWN},{OTHER}")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["deleted"], 2, "the comma-separated list has two ids");
}

#[tokio::test]
async fn enabled_delete_all_repeated_param_returns_count() {
    let (app, _calls) = app(true);
    let (status, body) = send(
        app,
        delete(format!(
            "{BASE}/admin/ehr/all?ehr_id={KNOWN}&ehr_id={OTHER}"
        )),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).expect("json body");
    // The repeated `?ehr_id=` form is surfaced from the raw query (the generated
    // Option<String> param would otherwise keep only the first).
    assert_eq!(v["deleted"], 2);
}

#[tokio::test]
async fn enabled_delete_all_missing_list_is_400_without_backend() {
    let (app, calls) = app(true);
    let (status, _) = send(app, delete(format!("{BASE}/admin/ehr/all"))).await;
    // Refusing an implicit delete-everything: absent list → 400, backend untouched.
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "no backend call on empty list"
    );
}
