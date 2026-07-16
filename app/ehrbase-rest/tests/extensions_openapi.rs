//! Tests for the generated extension-surface `OpenAPI` document
//! ([`extensions_document`]) and its consistency with the live router.
//!
//! No openEHR spec governs the extension surface — our own operational +
//! extension design. These tests assert (1) the document is non-empty, (2)
//! every documented path actually routes on a fully-enabled server (a `404`
//! would mean the path is documented but not mounted; an auth `401` never
//! occurs here because auth is disabled), and (3) a set of representative
//! extension paths are present so the document cannot silently shrink.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, OnceLock};

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use serde_json::Value;
use tower::ServiceExt;

use ehrbase::config::auth::AuthConfig;
use ehrbase::config::management::{AccessLevel, EndpointLevels, ManagementConfig};
use ehrbase::config::server::{AdminConfig, ServerConfig, TenancyConfig};
use ehrbase::config::smart::SmartConfig;
use ehrbase::telemetry::build_info::BuildInfo;
use ehrbase::telemetry::health::HealthRegistry;
use ehrbase::telemetry::log_reload::LogReload;
use ehrbase_rest::config::AppConfig;
use ehrbase_rest::extensions::management::Observability;
use ehrbase_rest::extensions::openapi::extensions_document;

mod common;

const BASE: &str = "/ehrbase/rest/openehr/v1";

/// The known HTTP methods a documented path item may carry (everything else on
/// a path-item object — `parameters`, `summary`, … — is metadata, not an op).
const HTTP_METHODS: &[&str] = &[
    "get", "put", "post", "delete", "patch", "head", "options", "trace",
];

// ── Test 1: the document is non-empty and covers the extension surface ────────

#[test]
fn extensions_doc_is_non_empty() {
    let doc = serde_json::to_value(extensions_document(&app_config())).expect("serialise doc");
    let paths = doc["paths"].as_object().expect("paths object");
    assert!(!paths.is_empty(), "the extension document has no paths");

    // A floor on coverage so the document cannot silently shrink below the
    // surface enumerated from the router.
    let op_count: usize = paths
        .values()
        .map(|item| {
            item.as_object()
                .map_or(0, |o| o.keys().filter(|k| is_method(k)).count())
        })
        .sum();
    assert!(
        op_count >= 40,
        "expected the extension surface to document >= 40 operations, found {op_count}"
    );

    // Representative paths from each documented extension group must be present.
    for expected in [
        "/ehrbase/rest/status",
        "/management/info",
        "/ehrbase/rest/.well-known/smart-configuration",
        "/ehrbase/rest/api-docs/openapi.json",
        "/ehrbase/rest/openehr/v1/terminology",
        "/ehrbase/rest/openehr/v1/demographic/party_relationship",
        "/ehrbase/rest/openehr/v1/admin/event_subscription",
        "/ehrbase/rest/openehr/v1/admin/tenant",
        "/ehrbase/rest/openehr/v1/fhir/r4/{resource_type}",
    ] {
        assert!(
            paths.contains_key(expected),
            "extension document is missing the path {expected}"
        );
    }
}

// ── Test 2: every documented path routes on a fully-enabled server ────────────

#[tokio::test]
async fn every_documented_path_routes() {
    let app = full_app("ext_openapi_routes").await;

    // Warm the HTTP metrics so `/management/metrics/{name}` has a real metric to
    // resolve (an unknown metric legitimately 404s — that would be a false
    // negative for a routing check).
    let _ = app
        .clone()
        .oneshot(get(&format!(
            "{BASE}/ehr/00000000-0000-0000-0000-000000000000"
        )))
        .await
        .expect("warmup response");
    let metric_name = first_metric_name(&app).await;

    let doc = serde_json::to_value(extensions_document(&app_config())).expect("serialise doc");
    let paths = doc["paths"].as_object().expect("paths object");

    let mut checked = 0usize;
    for (template, item) in paths {
        let item = item.as_object().expect("path item object");
        for method in item.keys().filter(|k| is_method(k)) {
            let concrete = concretize(template, &metric_name);
            let req = Request::builder()
                .method(method.to_uppercase().as_str())
                .uri(&concrete)
                .body(Body::empty())
                .expect("request");
            let resp = app.clone().oneshot(req).await.expect("response");
            let status = resp.status();
            let body = resp.into_body().collect().await.expect("body").to_bytes();
            // A documented op must be *mounted*. Under the real service a mounted
            // handler may legitimately answer 404 for a missing resource (e.g. a
            // stored query that does not exist) — that still means the route is
            // mounted (the Mock backend previously masked this by answering 501
            // everywhere). An *unmounted* path is axum's bare fallback 404 with an
            // empty body; a mounted handler's 404 carries the openEHR
            // `{error,message}` body. So "not mounted" = a 404 with an empty body.
            assert!(
                status != StatusCode::NOT_FOUND || !body.is_empty(),
                "documented op {} {template} (driven as {concrete}) is not mounted (bare 404)",
                method.to_uppercase()
            );
            checked += 1;
        }
    }
    assert!(checked >= 40, "drove only {checked} documented operations");
}

// ── Test 3: every spec-selector family document is non-empty ──────────────────

/// Every family the Swagger spec-selector offers must serve a non-empty
/// document — the path-prefix criterion (standard ITS-REST groups) and the
/// tag criterion (server extensions) must each match at least one operation on
/// a fully-enabled server.
#[tokio::test]
async fn every_family_document_is_non_empty() {
    let app = full_app("ext_openapi_family").await;
    // The family slugs offered by `FAMILIES` in `extensions::openapi` (private
    // there; the selector URLs are the stable public contract).
    for slug in [
        "ehr",
        "query",
        "definition",
        "demographic",
        "admin",
        "management",
        "terminology",
        "relationships",
        "events",
        "tenancy",
        "fhir",
        "smart",
    ] {
        let uri = format!("/ehrbase/rest/api-docs/ehrbase-{slug}.openapi.json");
        let resp = app
            .clone()
            .oneshot(get(&uri))
            .await
            .expect("family response");
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "family {slug} must serve ({uri})"
        );
        let bytes = resp.into_body().collect().await.expect("body").to_bytes();
        let v: Value = serde_json::from_slice(&bytes).expect("family json");
        let paths = v["paths"].as_object().expect("paths object");
        assert!(
            !paths.is_empty(),
            "family {slug} document has no paths (its selector criterion matched nothing)"
        );
    }
}

// ── Test 4: the once-built served document equals a fresh extensions_document ─

/// the `OpenAPI` document is built once at router assembly and served as
/// pre-serialized bytes. The served bytes must be byte-for-byte the same
/// document `extensions_document(cfg)` produces fresh — the optimization is a
/// pure serving-mechanics change, no content change.
#[tokio::test]
async fn served_openapi_json_equals_fresh_document() {
    let app = full_app("ext_openapi_served").await;
    let resp = app
        .clone()
        .oneshot(get("/ehrbase/rest/api-docs/openapi.json"))
        .await
        .expect("openapi.json response");
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    let served: Value = serde_json::from_slice(&bytes).expect("served openapi json");

    let fresh = serde_json::to_value(extensions_document(&app_config()))
        .expect("fresh extensions_document");
    assert_eq!(
        served, fresh,
        "the once-built served OpenAPI document must equal a fresh extensions_document(cfg)"
    );
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn is_method(key: &str) -> bool {
    HTTP_METHODS.contains(&key)
}

/// Substitute `{param}` template segments with a value that routes: the real
/// metric name for `{name}` (so `/management/metrics/{name}` resolves), any
/// non-empty token otherwise (malformed uuids yield `400`, never a routing
/// `404`).
fn concretize(template: &str, metric_name: &str) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let close = rest[open..].find('}').expect("closing brace") + open;
        let param = &rest[open + 1..close];
        out.push_str(if param == "name" { metric_name } else { "x" });
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .expect("request")
}

/// The first metric name the management list endpoint reports (after warmup).
async fn first_metric_name(app: &Router) -> String {
    let resp = app
        .clone()
        .oneshot(get("/management/metrics"))
        .await
        .expect("metrics list response");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "metrics list must be mounted"
    );
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    let v: Value = serde_json::from_slice(&bytes).expect("metrics json");
    let names = v["names"].as_array().expect("names array");
    assert!(!names.is_empty(), "no metrics available after warmup");
    names[0].as_str().expect("metric name").to_owned()
}

/// The global Prometheus recorder (install-once per test process).
fn recorder() -> &'static PrometheusHandle {
    static HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();
    HANDLE.get_or_init(|| {
        PrometheusBuilder::new()
            .install_recorder()
            .expect("install recorder")
    })
}

/// A no-op reloadable-filter handle so `/management/loggers` mounts.
fn log_reload() -> LogReload {
    LogReload::new(
        "info",
        Arc::new(|| "info".to_owned()),
        Arc::new(|_f: &str| Ok(())),
    )
}

/// The `AppConfig` used by both the served document and [`full_app`], so the
/// documented paths and the mounted routes are derived from one config (same
/// base path). Auth off + every surface enabled, so every documented path is
/// mounted.
fn app_config() -> AppConfig {
    AppConfig {
        server: ServerConfig {
            swagger_ui: true,
            ..Default::default()
        },
        auth: AuthConfig {
            enabled: false,
            ..AuthConfig::default()
        },
        admin: AdminConfig { enabled: true },
        tenancy: TenancyConfig {
            enabled: true,
            ..TenancyConfig::default()
        },
        smart: SmartConfig {
            enabled: true,
            ..SmartConfig::default()
        },
        fhir_api_enabled: true,
        terminology_api_enabled: true,
        events_admin_api: true,
    }
}

/// A server with auth off and every extension surface enabled, so every
/// documented path is mounted.
async fn full_app(name: &str) -> Router {
    let config = app_config();
    let public = EndpointLevels {
        health: AccessLevel::Public,
        info: AccessLevel::Public,
        metrics: AccessLevel::Public,
        prometheus: AccessLevel::Public,
        env: AccessLevel::Public,
        loggers: AccessLevel::Public,
    };
    let observability = Observability {
        management: ManagementConfig {
            enabled: true,
            probes_enabled: true,
            endpoints: public,
            ..ManagementConfig::default()
        },
        prometheus: Some(recorder().clone()),
        log_reload: Some(log_reload()),
        health: HealthRegistry::default(),
        build_info: BuildInfo::current(),
        ..Observability::default()
    };
    let (_pg, service) = common::test_service(name).await;
    ehrbase_rest::build_full(config, service, None, observability).expect("build")
}
