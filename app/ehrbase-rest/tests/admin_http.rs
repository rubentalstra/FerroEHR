//! End-to-end HTTP tests for the ADMIN API group (physical EHR delete): the
//! config gate (`RestConfig::admin.enabled`) and the `204`/`404` wire outcomes
//! — driven through the assembled router with a canned [`AdminService`]
//! backend that records whether it was consulted. Per the vendored Admin OAS
//! (`operations/admin_ehr_delete_all.yaml`) the bulk delete declares only
//! bodyless successes (`202`/`204`), and an absent `ehr_id` list means
//! "delete ALL EHRs" (`parameters/query/ehr_id_Admin.yaml`).
//!
//! Spec grounding: SM `I_ADMIN_SERVICE.physical_ehr_delete` and the CNF Robot
//! prior art (`CNF/tests/platform/robot/I_ADMIN_SERVICE/001-EHR.robot`:
//! `DELETE /admin/ehr/{ehr_id}` → `204`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use ehrbase_rest::access::authn::config::AuthConfig;
use ehrbase_rest::{AdminConfig, AppConfig, ServerConfig};
use ehrbase::service::SmError;

mod common;
use common::{Hooks, Mock};

const BASE: &str = "/ehrbase/rest/openehr/v1";
/// A known EHR the mock "deletes" successfully; anything else is `NotFound`.
const KNOWN: &str = "7d44b88c-4199-4bad-97dc-d78268e01398";
const OTHER: &str = "11111111-2222-3333-4444-555555555555";

/// The admin hooks: `admin_ehr_delete` succeeds for [`KNOWN`] and
/// `ehr_id_does_not_exist` (→ `404`) otherwise; `admin_ehr_delete_all` reports
/// the count of ids it was handed. The `calls` counter proves the backend is
/// (not) consulted.
fn hooks(calls: Arc<AtomicUsize>) -> Hooks {
    let c1 = calls.clone();
    let c2 = calls;
    Hooks {
        admin_ehr_delete: Some(Arc::new(move |ehr_id: String| {
            c1.fetch_add(1, Ordering::SeqCst);
            if ehr_id == KNOWN {
                Ok(())
            } else {
                Err(SmError::ehr_not_found(format!("EHR {ehr_id}")))
            }
        })),
        admin_ehr_delete_all: Some(Arc::new(move |ehr_ids: Vec<String>| {
            c2.fetch_add(1, Ordering::SeqCst);
            Ok(ehr_ids.len() as u64)
        })),
        ..Default::default()
    }
}

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

fn app(admin_enabled: bool) -> (Router, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let backend = Arc::new(Mock::with(hooks(calls.clone())));
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
async fn enabled_delete_all_is_204_bodyless() {
    // `admin_ehr_delete_all.yaml:18-26`: the only declared success responses
    // are `202` (async) and `204 No Content` (sync) — both bodyless.
    let (app, calls) = app(true);
    let (status, body) = send(
        app,
        delete(format!("{BASE}/admin/ehr/all?ehr_id={KNOWN},{OTHER}")),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_empty(), "204 carries no body, got {body:?}");
    assert_eq!(calls.load(Ordering::SeqCst), 1, "one bulk backend call");
}

#[tokio::test]
async fn enabled_delete_all_repeated_param_reaches_backend_with_both_ids() {
    // The repeated `?ehr_id=` form is surfaced from the raw query (the generated
    // Option<String> param would otherwise keep only the first) — the mock
    // records the ids it was handed so the RFC 6570 `{?ehr_id*}` list handling
    // stays covered even though the wire success is bodyless.
    let seen = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let recorder = seen.clone();
    let backend = Arc::new(Mock::with(Hooks {
        admin_ehr_delete_all: Some(Arc::new(move |ehr_ids: Vec<String>| {
            recorder.lock().unwrap().clone_from(&ehr_ids);
            Ok(ehr_ids.len() as u64)
        })),
        ..Default::default()
    }));
    let router = ehrbase_rest::build_with(config(true), backend).expect("router builds");
    let (status, body) = send(
        router,
        delete(format!(
            "{BASE}/admin/ehr/all?ehr_id={KNOWN}&ehr_id={OTHER}"
        )),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_empty(), "204 carries no body, got {body:?}");
    assert_eq!(
        *seen.lock().unwrap(),
        vec![KNOWN.to_owned(), OTHER.to_owned()],
        "both repeated ehr_id values must reach the backend"
    );
}

#[tokio::test]
async fn enabled_delete_all_missing_list_deletes_all() {
    // `parameters/query/ehr_id_Admin.yaml`: `ehr_id` is "an optional parameter
    // to perform the operation on a subset of EHRs" — an absent list means
    // "delete ALL EHRs" (`admin_ehr_delete_all.yaml:5`), expressed to the
    // backend as the empty list.
    let (app, calls) = app(true);
    let (status, body) = send(app, delete(format!("{BASE}/admin/ehr/all"))).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_empty(), "204 carries no body, got {body:?}");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "the all-EHRs request reaches the backend once"
    );
}
