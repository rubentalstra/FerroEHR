//! End-to-end HTTP tests for the MUST-level ITS-REST protocol tail (B6 cluster
//! 4): the `openEHR-VERSION.*` / `openEHR-AUDIT_DETAILS.*` committal request
//! headers (parse + merge), `If-Match` hardening (malformed → 400), the
//! `OPTIONS /` System-Options-and-Conformance endpoint, and canonical-XML
//! responses for the VERSION family (F-05-06). Driven through the assembled
//! router with the shared [`Mock`] platform.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use ehrbase_rest::RestConfig;
use ehrbase_rest::access::authn::config::AuthConfig;
use ehrbase_sm::UpdateVersion;

mod common;
use common::{Hooks, Mock};

const BASE: &str = "/ehrbase/rest/openehr/v1";
const EHR_ID: &str = "7d44b88c-4199-4bad-97dc-d78268e01398";
const VO_ID: &str = "8849182c-82ad-4088-a07f-48ead4180515";
const OVID: &str = "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys::2";

fn config() -> RestConfig {
    RestConfig {
        smart: Default::default(),
        system: Default::default(),
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
        event_subscription: ehrbase_rest::EventSubscriptionConfig::default(),
        tenancy: ehrbase_rest::TenancyConfig::default(),
        fhir: ehrbase_rest::FhirConfig::default(),
    }
}

fn app(hooks: Hooks) -> Router {
    ehrbase_rest::build_with(config(), Arc::new(Mock::with(hooks))).expect("router builds")
}

async fn send(app: Router, req: Request<Body>) -> (StatusCode, header::HeaderMap, String) {
    let resp = app.oneshot(req).await.expect("response");
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

// ── OPTIONS / (R32) ────────────────────────────────────────────────────────

#[tokio::test]
async fn options_root_is_system_options_and_conformance() {
    let req = Request::builder()
        .method("OPTIONS")
        .uri("/")
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(app(Hooks::default()), req).await;

    assert_eq!(status, StatusCode::OK);
    // The `Allow` header lists the supported methods.
    assert_eq!(
        h.get(header::ALLOW).and_then(|v| v.to_str().ok()),
        Some("GET, POST, PUT, DELETE, OPTIONS")
    );
    // The `Options` conformance manifest body.
    let v: Value = serde_json::from_str(&body).expect("options body");
    assert_eq!(v["restapi_specs_version"], "1.0.3");
    assert_eq!(v["conformance_profile"], "STANDARD");
    assert!(v["endpoints"].as_array().is_some_and(|e| !e.is_empty()));
}

// ── committal headers (openEHR-VERSION.* / openEHR-AUDIT_DETAILS.*) ──────────

#[tokio::test]
async fn committal_headers_merge_into_the_commit() {
    let captured: Arc<Mutex<Option<UpdateVersion>>> = Arc::new(Mutex::new(None));
    let sink = captured.clone();
    let hooks = Hooks {
        update_composition: Some(Arc::new(move |_e, _vo, uv| {
            *sink.lock().unwrap() = Some(uv);
            Ok(OVID.to_owned())
        })),
        ..Default::default()
    };

    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{EHR_ID}/composition/{VO_ID}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, format!("\"{OVID}\""))
        .header("openEHR-VERSION.lifecycle_state", "code_string=\"523\"")
        .header("openEHR-AUDIT_DETAILS.change_type", "code_string=\"251\"")
        .header(
            "openEHR-AUDIT_DETAILS.description",
            "value=\"An updated composition\"",
        )
        .header(
            "openEHR-AUDIT_DETAILS.committer",
            "name=\"John Doe\", external_ref.id=\"BC8132EA-8F4A-11E7-BB31-BE2E44B06B34\", \
             external_ref.namespace=\"demographic\", external_ref.type=\"PERSON\"",
        )
        .body(Body::from(r#"{"_type":"COMPOSITION"}"#))
        .unwrap();
    let (status, _h, _body) = send(app(hooks), req).await;
    assert!(status.is_success(), "update succeeded, got {status}");

    let uv = captured.lock().unwrap().take().expect("uv captured");
    assert_eq!(uv.lifecycle_state.code_string, "523");
    assert_eq!(uv.audit.change_type.code_string, "251");
    assert_eq!(
        uv.audit.description.as_deref(),
        Some("An updated composition")
    );
    let committer = serde_json::to_value(&uv.audit.committer).unwrap();
    assert_eq!(committer["name"], "John Doe");
    assert_eq!(committer["external_ref"]["type"], "PERSON");
}

// ── If-Match hardening (F-01-09/F-02-08) ─────────────────────────────────────

#[tokio::test]
async fn malformed_if_match_is_rejected_not_bypassed() {
    // A required If-Match that is not a well-formed OBJECT_VERSION_ID must be a
    // client error (400), never a silent skip of the precondition. The backend
    // hook must never be reached.
    let hooks = Hooks {
        update_composition: Some(Arc::new(|_e, _vo, _uv| {
            panic!("backend reached despite malformed If-Match");
        })),
        ..Default::default()
    };
    let req = Request::builder()
        .method("PUT")
        .uri(format!("{BASE}/ehr/{EHR_ID}/composition/{VO_ID}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::IF_MATCH, "\"not-an-object-version-id\"")
        .body(Body::from(r#"{"_type":"COMPOSITION"}"#))
        .unwrap();
    let (status, _h, _body) = send(app(hooks), req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ── VERSION-family canonical XML (F-05-06 / ECC-COM-022, ECC-SIG-001) ────────

#[tokio::test]
async fn versioned_composition_serves_xml() {
    let hooks = Hooks {
        get_versioned_composition: Some(Arc::new(|_e, vo| {
            Ok(json!({
                "_type": "VERSIONED_OBJECT",
                "uid": { "_type": "HIER_OBJECT_ID", "value": vo.to_string() },
                "owner_id": {
                    "_type": "OBJECT_REF", "namespace": "local", "type": "EHR",
                    "id": { "_type": "HIER_OBJECT_ID", "value": EHR_ID }
                },
                "time_created": { "_type": "DV_DATE_TIME", "value": "2024-01-01T00:00:00Z" }
            }))
        })),
        ..Default::default()
    };
    let req = Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{EHR_ID}/versioned_composition/{VO_ID}"))
        .header(header::ACCEPT, "application/xml")
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(app(hooks), req).await;

    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(
        h.get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()),
        Some("application/xml")
    );
    assert!(body.contains("<versioned_composition"), "root: {body}");
    assert!(body.contains(VO_ID), "uid present: {body}");
}

#[tokio::test]
async fn composition_version_serves_xml_with_signature() {
    // ECC-SIG-001: the ORIGINAL_VERSION XML carries the `<signature>` element.
    let hooks = Hooks {
        composition_original_version: Some(Arc::new(|_e, ovid| {
            Ok(json!({
                "_type": "ORIGINAL_VERSION",
                "contribution": {
                    "_type": "OBJECT_REF", "namespace": "local", "type": "CONTRIBUTION",
                    "id": { "_type": "HIER_OBJECT_ID", "value": "c1" }
                },
                "commit_audit": {
                    "_type": "AUDIT_DETAILS",
                    "system_id": "ehrbase-rs",
                    "time_committed": { "_type": "DV_DATE_TIME", "value": "2024-01-01T00:00:00Z" },
                    "change_type": {
                        "_type": "DV_CODED_TEXT", "value": "creation",
                        "defining_code": { "_type": "CODE_PHRASE",
                            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                            "code_string": "249" }
                    },
                    "committer": { "_type": "PARTY_IDENTIFIED", "name": "clinician" }
                },
                "signature": "-----BEGIN PGP SIGNATURE-----\nDEADBEEF\n-----END PGP SIGNATURE-----",
                "uid": { "_type": "OBJECT_VERSION_ID", "value": ovid.value },
                "lifecycle_state": {
                    "_type": "DV_CODED_TEXT", "value": "complete",
                    "defining_code": { "_type": "CODE_PHRASE",
                        "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                        "code_string": "532" }
                }
            }))
        })),
        ..Default::default()
    };
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "{BASE}/ehr/{EHR_ID}/versioned_composition/{VO_ID}/version/{OVID}"
        ))
        .header(header::ACCEPT, "application/xml")
        .body(Body::empty())
        .unwrap();
    let (status, h, body) = send(app(hooks), req).await;

    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(
        h.get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()),
        Some("application/xml")
    );
    assert!(body.contains("<original_version"), "root: {body}");
    assert!(body.contains("<signature"), "signature element: {body}");
    assert!(body.contains("DEADBEEF"), "signature value: {body}");
}
