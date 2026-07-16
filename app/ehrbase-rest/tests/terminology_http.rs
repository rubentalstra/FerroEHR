//! End-to-end HTTP tests for the terminology extension API group (SM
//! `I_TERMINOLOGY_SERVICE`, wire design `docs/design/sm-platform/
//! 08-target-architecture.md` §7): the config gate
//! (`RestConfig::terminology.enabled`), the `200`/`404`/`400`/`501` wire
//! outcomes for `get_terminology_ids` / `get_terminology_description` /
//! `get_term` / `subsumes` / `get_value_set` / `value_set_validate`, and the
//! JSON body shapes — driven through the assembled router with the shared
//! [`Mock`] platform whose terminology hooks record whether the backend was
//! consulted.
//!
//! Spec grounding: `docs/specs/openehr/SM/docs/UML/classes/
//! i_terminology_service.adoc` (the nine calls + `Pre_has_*` preconditions).
//! A failed precondition surfaces as `versioned_object_does_not_exist` (the
//! bundle provider's convention), which the adapter maps to HTTP `404`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use ehrbase_rest::access::authn::config::AuthConfig;
use ehrbase_rest::{AppConfig, ServerConfig};
use ehrbase::service::{
    CallStatusType, DefinedTerm, SmError, TermEntry, TerminologyDescription, TerminologyExtract,
};

mod common;
use common::{Hooks, Mock};

const BASE: &str = "/ehrbase/rest/openehr/v1";
/// A terminology the mock knows; anything else is `versioned_object_does_not_exist`.
const KNOWN: &str = "openehr";
const UNKNOWN: &str = "no-such-terminology";

/// Terminology hooks covering the six wire-exposed calls. `calls` counts every
/// backend consult so a test can prove the gate never reaches the backend.
fn hooks(calls: Arc<AtomicUsize>) -> Hooks {
    let c = calls;
    let (c1, c2, c3, c4, c5, c6) = (
        c.clone(),
        c.clone(),
        c.clone(),
        c.clone(),
        c.clone(),
        c.clone(),
    );
    Hooks {
        get_terminology_ids: Some(Arc::new(move || {
            c1.fetch_add(1, Ordering::SeqCst);
            Ok(vec![KNOWN.to_owned(), "openehr_ehr_status".to_owned()])
        })),
        get_terminology_description: Some(Arc::new(move |terminology_id: String| {
            c2.fetch_add(1, Ordering::SeqCst);
            if terminology_id == KNOWN {
                Ok(TerminologyDescription {
                    publisher: "openEHR".to_owned(),
                    available_versions: Some(vec!["3.1.0".to_owned()]),
                    attributes: None,
                    uri: "http://openehr.org/terminology".to_owned(),
                })
            } else {
                Err(SmError::new(
                    CallStatusType::VersionedObjectDoesNotExist,
                    format!("terminology {terminology_id} does not exist"),
                ))
            }
        })),
        get_term: Some(Arc::new(
            move |terminology_id: String, code: String, at_date: Option<String>| {
                c3.fetch_add(1, Ordering::SeqCst);
                Ok(TerminologyExtract {
                    terminology_id,
                    // Echo the at_date so the test can assert it was threaded through.
                    terminology_version: Some(at_date.unwrap_or_else(|| "none".to_owned())),
                    terms: Some(
                        [(
                            code.clone(),
                            TermEntry::Defined(DefinedTerm {
                                code,
                                text: "completed".to_owned(),
                                language: Some("en".to_owned()),
                                is_preferred_term: None,
                            }),
                        )]
                        .into_iter()
                        .collect(),
                    ),
                    relationships: None,
                    relations: None,
                })
            },
        )),
        subsumes: Some(Arc::new(
            move |_tid: String, ref_code: String, candidate: String| {
                c4.fetch_add(1, Ordering::SeqCst);
                // Identity-only subsumption (the openEHR bundle is flat).
                Ok(ref_code == candidate)
            },
        )),
        get_value_set: Some(Arc::new(
            move |terminology_id: String, value_set_id: String| {
                c5.fetch_add(1, Ordering::SeqCst);
                if value_set_id == "known_group" {
                    Ok(TerminologyExtract {
                        terminology_id,
                        terms: Some(
                            [(
                                "532".to_owned(),
                                TermEntry::Bare(ehrbase::service::TermCode {
                                    code: "532".to_owned(),
                                }),
                            )]
                            .into_iter()
                            .collect(),
                        ),
                        ..Default::default()
                    })
                } else {
                    Err(SmError::new(
                        CallStatusType::VersionedObjectDoesNotExist,
                        format!("value set {value_set_id} does not exist"),
                    ))
                }
            },
        )),
        value_set_validate: Some(Arc::new(
            move |_tid: String, _vs: String, candidate_code: String, _at: Option<String>| {
                c6.fetch_add(1, Ordering::SeqCst);
                Ok(candidate_code == "532")
            },
        )),
        ..Default::default()
    }
}

fn config(terminology_enabled: bool) -> AppConfig {
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
        terminology_api_enabled: terminology_enabled,
        ..Default::default()
    }
}

fn app(terminology_enabled: bool) -> (Router, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let backend = Arc::new(Mock::with(hooks(calls.clone())));
    let router =
        ehrbase_rest::build_with(config(terminology_enabled), backend).expect("router builds");
    (router, calls)
}

/// An app with the terminology group enabled but **no** hooks — every call
/// hits the trait default (`501`).
fn app_unhooked() -> Router {
    let backend = Arc::new(Mock::new());
    ehrbase_rest::build_with(config(true), backend).expect("router builds")
}

async fn send(app: Router, req: Request<Body>) -> (StatusCode, String) {
    let resp = app.oneshot(req).await.expect("response");
    let status = resp.status();
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn get(uri: String) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn disabled_terminology_is_404_and_never_touches_backend() {
    let (app, calls) = app(false);
    let (status, _) = send(app, get(format!("{BASE}/terminology"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "backend must not be called when the group is disabled"
    );
}

#[tokio::test]
async fn terminology_ids_returns_list() {
    let (app, calls) = app(true);
    let (status, body) = send(app, get(format!("{BASE}/terminology"))).await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).expect("json body");
    let ids = v["terminology_ids"].as_array().expect("terminology_ids");
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0], KNOWN);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn terminology_description_found_is_200() {
    let (app, _calls) = app(true);
    let (status, body) = send(app, get(format!("{BASE}/terminology/{KNOWN}"))).await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["publisher"], "openEHR");
    assert_eq!(v["uri"], "http://openehr.org/terminology");
}

#[tokio::test]
async fn terminology_description_unknown_maps_to_404() {
    let (app, _calls) = app(true);
    // The bundle's `versioned_object_does_not_exist` surfaces as HTTP 404.
    let (status, _) = send(app, get(format!("{BASE}/terminology/{UNKNOWN}"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_term_lookup_returns_extract_and_threads_at_date() {
    let (app, _calls) = app(true);
    let (status, body) = send(
        app,
        get(format!(
            "{BASE}/terminology/{KNOWN}/term/532?at_date=2024-01-01"
        )),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["terminology_id"], KNOWN);
    // `at_date` was threaded through to the seam (echoed as terminology_version).
    assert_eq!(v["terminology_version"], "2024-01-01");
    assert_eq!(v["terms"]["532"]["text"], "completed");
}

#[tokio::test]
async fn subsumes_returns_bool() {
    let (app, _calls) = app(true);
    // Identity → true.
    let (status, body) = send(
        app,
        get(format!(
            "{BASE}/terminology/{KNOWN}/subsumes?ref_code=532&candidate=532"
        )),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["subsumes"], true);
}

#[tokio::test]
async fn subsumes_missing_required_query_is_400_without_backend() {
    let (app, calls) = app(true);
    // `candidate` absent → 400 before the backend is consulted.
    let (status, _) = send(
        app,
        get(format!("{BASE}/terminology/{KNOWN}/subsumes?ref_code=532")),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn get_value_set_expand_returns_extract() {
    let (app, _calls) = app(true);
    let (status, body) = send(
        app,
        get(format!("{BASE}/terminology/{KNOWN}/value_set/known_group")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["terminology_id"], KNOWN);
    assert!(v["terms"].get("532").is_some());
}

#[tokio::test]
async fn get_value_set_unknown_maps_to_404() {
    let (app, _calls) = app(true);
    let (status, _) = send(
        app,
        get(format!(
            "{BASE}/terminology/{KNOWN}/value_set/no_such_group"
        )),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn value_set_validate_returns_valid() {
    let (app, _calls) = app(true);
    let (status, body) = send(
        app,
        get(format!(
            "{BASE}/terminology/{KNOWN}/value_set/known_group/validate?candidate_code=532"
        )),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(v["valid"], true);
}

#[tokio::test]
async fn value_set_validate_missing_candidate_is_400() {
    let (app, calls) = app(true);
    let (status, _) = send(
        app,
        get(format!(
            "{BASE}/terminology/{KNOWN}/value_set/known_group/validate"
        )),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn unhooked_terminology_call_is_501() {
    // Enabled group, no backend override → the trait default (501).
    let app = app_unhooked();
    let (status, _) = send(app, get(format!("{BASE}/terminology"))).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}
