//! Integration tests for the management surface (binding doc §6): the
//! access-level matrix, the Prometheus exposition + route-template label +
//! cardinality guard, and separate-port isolation.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, OnceLock};

use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, SaltString};
use axum::Router;
use axum::body::Body;
use ehrbase_rest::RestConfig;
use ehrbase_rest::access::authn::config::{AuthConfig, BasicConfig, BasicUser, Redacted};
use ehrbase_rest::management::{
    AccessLevel, BuildInfo, EndpointLevels, HealthRegistry, ManagementConfig, Observability,
};

mod common;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use tower::ServiceExt;

/// Base64 of `admin:pw` for a Basic credential.
const ADMIN_BASIC: &str = "Basic YWRtaW46cHc=";

fn hash(pw: &str) -> String {
    let salt = SaltString::from_b64("MTIzNDU2Nzg5MDEyMzQ1Ng").expect("salt");
    Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .expect("hash")
        .to_string()
}

fn auth_config(admin_scope: Option<&str>) -> AuthConfig {
    AuthConfig {
        enabled: true,
        basic: Some(BasicConfig {
            users: vec![BasicUser {
                username: "admin".to_owned(),
                password_hash: Redacted(hash("pw")),
                roles: vec!["ADMIN".to_owned()],
            }],
        }),
        oidc: None,
        admin_scope: admin_scope.map(str::to_owned),
    }
}

/// Build an app with the management surface enabled, one endpoint (`info`) set
/// to `level`, and the given admin-scope configuration.
fn app_with(level: AccessLevel, admin_scope: Option<&str>) -> Router {
    let config = RestConfig {
        smart: ehrbase_rest::SmartConfig::default(),
        system: ehrbase_rest::SystemOptionsConfig::default(),
        auth: auth_config(admin_scope),
        swagger_ui: false,
        ..RestConfig::default()
    };
    let observability = Observability {
        management: ManagementConfig {
            enabled: true,
            endpoints: EndpointLevels {
                info: level,
                ..EndpointLevels::default()
            },
            ..ManagementConfig::default()
        },
        ..Observability::default()
    };
    ehrbase_rest::build_full(config, Arc::new(common::Mock::new()), None, observability)
        .expect("build")
}

async fn status_of(app: Router, req: Request<Body>) -> StatusCode {
    app.oneshot(req).await.expect("response").status()
}

fn get(path: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .body(Body::empty())
        .expect("request")
}

fn get_auth(path: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header(header::AUTHORIZATION, ADMIN_BASIC)
        .body(Body::empty())
        .expect("request")
}

#[tokio::test]
async fn off_endpoint_is_404() {
    let app = app_with(AccessLevel::Off, None);
    assert_eq!(
        status_of(app, get("/management/info")).await,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn public_endpoint_needs_no_auth() {
    let app = app_with(AccessLevel::Public, None);
    assert_eq!(
        status_of(app, get("/management/info")).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn private_endpoint_401_then_200() {
    let app = app_with(AccessLevel::Private, None);
    assert_eq!(
        status_of(app.clone(), get("/management/info")).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status_of(app, get_auth("/management/info")).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn admin_only_401_403_200() {
    // 401 unauthenticated.
    let app = app_with(AccessLevel::AdminOnly, Some("ehrbase:admin"));
    assert_eq!(
        status_of(app, get("/management/info")).await,
        StatusCode::UNAUTHORIZED
    );
    // 403 authenticated (Basic → no scopes) but admin scope required.
    let app = app_with(AccessLevel::AdminOnly, Some("ehrbase:admin"));
    assert_eq!(
        status_of(app, get_auth("/management/info")).await,
        StatusCode::FORBIDDEN
    );
    // 200 authenticated with no admin scope configured (authenticated is enough).
    let app = app_with(AccessLevel::AdminOnly, None);
    assert_eq!(
        status_of(app, get_auth("/management/info")).await,
        StatusCode::OK
    );
}

/// A process-wide Prometheus recorder (installed once; the global `metrics`
/// facade the HTTP layer emits through requires a single global recorder).
fn recorder() -> &'static PrometheusHandle {
    static HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();
    HANDLE.get_or_init(|| {
        PrometheusBuilder::new()
            .install_recorder()
            .expect("install recorder")
    })
}

fn app_with_metrics() -> Router {
    let config = RestConfig {
        smart: ehrbase_rest::SmartConfig::default(),
        system: ehrbase_rest::SystemOptionsConfig::default(),
        auth: AuthConfig {
            enabled: false, // exercise the API path without auth in this test
            ..AuthConfig::default()
        },
        swagger_ui: false,
        ..RestConfig::default()
    };
    let observability = Observability {
        management: ManagementConfig {
            enabled: true,
            endpoints: EndpointLevels {
                prometheus: AccessLevel::Public,
                metrics: AccessLevel::Public,
                ..EndpointLevels::default()
            },
            ..ManagementConfig::default()
        },
        prometheus: Some(recorder().clone()),
        health: HealthRegistry::default(),
        build_info: BuildInfo::current(),
        ..Observability::default()
    };
    ehrbase_rest::build_full(config, Arc::new(common::Mock::new()), None, observability)
        .expect("build")
}

#[tokio::test]
async fn prometheus_has_route_template_label_and_no_ids() {
    let app = app_with_metrics();

    // Drive an API request that matches a templated route carrying an id. The
    // HTTP metrics layer must label it by the *template*, never the raw id.
    let ehr_id = "3fa85f64-5717-4562-b3fc-2c963f66afa6";
    let _ = app
        .clone()
        .oneshot(get(&format!(
            "/ehrbase/rest/openehr/v1/ehr/{ehr_id}/composition/x"
        )))
        .await
        .expect("api response");

    // Scrape the exposition text.
    let resp = app
        .oneshot(get("/management/prometheus"))
        .await
        .expect("prometheus response");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.expect("body").to_bytes();
    let text = String::from_utf8(body.to_vec()).expect("utf8");

    assert!(
        text.contains("http_server_request_duration_seconds"),
        "missing HTTP duration metric:\n{text}"
    );
    assert!(
        text.contains("http_route="),
        "missing route-template label:\n{text}"
    );
    // Cardinality guard: no label VALUE may look like a UUID or a bare number id.
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        for value in label_values(line) {
            assert!(
                !looks_like_id(value),
                "id-shaped label value {value:?} leaked into metric line: {line}"
            );
        }
    }
}

/// Extract the label values (`k="v"`) from an exposition sample line.
fn label_values(line: &str) -> Vec<&str> {
    let Some(open) = line.find('{') else {
        return Vec::new();
    };
    let Some(close) = line.rfind('}') else {
        return Vec::new();
    };
    line[open + 1..close]
        .split(',')
        .filter_map(|pair| pair.split_once('='))
        .map(|(_, v)| v.trim().trim_matches('"'))
        .collect()
}

/// Whether a label value looks like a UUID or a long bare numeric id (the
/// cardinality budget forbids these).
fn looks_like_id(value: &str) -> bool {
    let uuid_shaped =
        value.len() == 36 && value.split('-').map(str::len).collect::<Vec<_>>() == [8, 4, 4, 4, 12];
    let long_number = value.len() >= 6 && value.chars().all(|c| c.is_ascii_digit());
    uuid_shaped || long_number
}

#[tokio::test]
async fn separate_port_mode_keeps_management_off_the_main_app() {
    // With `management.port` set, the main app must NOT mount /management…
    let config = RestConfig {
        smart: ehrbase_rest::SmartConfig::default(),
        system: ehrbase_rest::SystemOptionsConfig::default(),
        auth: AuthConfig {
            enabled: false,
            ..AuthConfig::default()
        },
        swagger_ui: false,
        ..RestConfig::default()
    };
    let observability = Observability {
        management: ManagementConfig {
            enabled: true,
            port: Some(9099),
            probes_enabled: true,
            endpoints: EndpointLevels {
                info: AccessLevel::Public,
                ..EndpointLevels::default()
            },
            ..ManagementConfig::default()
        },
        ..Observability::default()
    };
    let main_app =
        ehrbase_rest::build_full(config, Arc::new(common::Mock::new()), None, observability)
            .expect("build");

    // …the main app 404s the management route.
    assert_eq!(
        status_of(main_app, get("/management/info")).await,
        StatusCode::NOT_FOUND
    );
}
