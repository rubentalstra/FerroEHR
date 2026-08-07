//! Integration tests for the management surface: the access-level matrix, the
//! Prometheus exposition + route-template label + cardinality guard,
//! separate-port isolation, and the boundary that the surface carries no health
//! route at all (the probes are the always-on public `/health` family).
//!
//! No openEHR spec governs the management surface — our own operational design.

#![expect(
    clippy::expect_used,
    clippy::string_slice,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this integration \
              module's helpers and async bodies; panicking assertions and direct \
              fixture indexing are the intended shape here (the Rust Book ch11)"
)]

use std::sync::OnceLock;

use argon2::Argon2;
use argon2::password_hash::{PasswordHasher, SaltString};
use axum::Router;
use axum::body::Body;
use ferroehr::config::auth::{AuthConfig, BasicConfig, BasicUser};
use ferroehr::config::management::{AccessLevel, EndpointLevels, ManagementConfig};
use ferroehr::config::server::ServerConfig;
use ferroehr::telemetry::build_info::BuildInfo;
use ferroehr::telemetry::health::HealthRegistry;
use ferroehr_rest::config::AppConfig;
use ferroehr_rest::extensions::management::Observability;

use crate::common;
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

fn auth_config(roles: &[&str]) -> AuthConfig {
    AuthConfig {
        enabled: true,
        basic: Some(BasicConfig {
            users: vec![BasicUser {
                username: "admin".to_owned(),
                password_hash: ferroehr::config::secret::Secret::new(hash("pw")),
                password_hash_file: None,
                roles: roles.iter().map(|r| (*r).to_owned()).collect(),
            }],
        }),
        oidc: None,
        ..AuthConfig::default()
    }
}

/// Build an app with the management surface enabled, one endpoint (`info`) set
/// to `level`, the Basic user granted `roles`, and RBAC on or off — the
/// `AdminOnly` level gates on the RBAC `admin_role` (issue #1879 retired the
/// deprecated `admin_scope` alias).
async fn app_with(level: AccessLevel, roles: &[&str], rbac_enabled: bool) -> Router {
    let config = AppConfig {
        server: ServerConfig {
            swagger_ui: false,
            ..Default::default()
        },
        auth: auth_config(roles),
        ..Default::default()
    };
    let mut authz_cfg = ferroehr::config::authz::AuthzConfig::default();
    authz_cfg.rbac.enabled = rbac_enabled;
    let resolvers = ferroehr_rest::extensions::access::authz::AuthzResolvers {
        subject: std::sync::Arc::new(|_| Box::pin(async { Ok(None) })),
        template_of_version: std::sync::Arc::new(|_, _| Box::pin(async { Ok(None) })),
    };
    let authz = ferroehr_rest::extensions::access::authz::AuthzHandle::build(
        &authz_cfg,
        &config.server.base_path,
        None,
        resolvers,
    )
    .map(std::sync::Arc::new);
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
    let (_pg, service) = common::test_service().await;
    ferroehr_rest::build_full(config, service, authz, observability).expect("build")
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
    let app = app_with(AccessLevel::Off, &["ADMIN"], true).await;
    assert_eq!(
        status_of(app, get("/management/info")).await,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn public_endpoint_needs_no_auth() {
    let app = app_with(AccessLevel::Public, &["ADMIN"], true).await;
    assert_eq!(
        status_of(app, get("/management/info")).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn private_endpoint_401_then_200() {
    let app = app_with(AccessLevel::Private, &["ADMIN"], true).await;
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
    let app = app_with(AccessLevel::AdminOnly, &["ADMIN"], true).await;
    assert_eq!(
        status_of(app, get("/management/info")).await,
        StatusCode::UNAUTHORIZED
    );
    // 403 authenticated but without the RBAC admin role.
    let app = app_with(AccessLevel::AdminOnly, &["USER"], true).await;
    assert_eq!(
        status_of(app, get_auth("/management/info")).await,
        StatusCode::FORBIDDEN
    );
    // 200 with the admin role.
    let app = app_with(AccessLevel::AdminOnly, &["ADMIN"], true).await;
    assert_eq!(
        status_of(app, get_auth("/management/info")).await,
        StatusCode::OK
    );
    // 200 with RBAC disabled: authenticated is enough (auth-only deployments).
    let app = app_with(AccessLevel::AdminOnly, &["USER"], false).await;
    assert_eq!(
        status_of(app, get_auth("/management/info")).await,
        StatusCode::OK
    );
}

/// The management surface hosts NO health route any more: with the surface
/// enabled and its one configurable endpoint public, every former
/// `/management/health*` path is a plain `404`, while the public health family
/// answers on the same app. (Test adapted to the routing this split
/// deliberately changed — the aggregate-health view is now the
/// `/health/readiness` indicator body.)
#[tokio::test]
async fn management_serves_no_health_route() {
    let app = app_with(AccessLevel::Public, &["ADMIN"], true).await;
    for gone in [
        "/management/health",
        "/management/health/liveness",
        "/management/health/readiness",
    ] {
        assert_eq!(
            status_of(app.clone(), get(gone)).await,
            StatusCode::NOT_FOUND,
            "{gone} must no longer be routed"
        );
    }
    for public in ["/health", "/health/liveness", "/health/readiness"] {
        assert_eq!(
            status_of(app.clone(), get(public)).await,
            StatusCode::OK,
            "{public} must answer unauthenticated"
        );
    }
}

/// The public health family does not depend on the management surface at all:
/// with `management.enabled = false` (the default posture) the three probes
/// still answer, and `/management/info` is absent.
#[tokio::test]
async fn health_family_survives_management_disabled() {
    let config = AppConfig {
        server: ServerConfig {
            swagger_ui: false,
            ..Default::default()
        },
        auth: auth_config(&["ADMIN"]),
        ..Default::default()
    };
    let (_pg, service) = common::test_service().await;
    let app = ferroehr_rest::build_full(config, service, None, Observability::default())
        .expect("build with management disabled");

    for public in ["/health", "/health/liveness", "/health/readiness"] {
        assert_eq!(
            status_of(app.clone(), get(public)).await,
            StatusCode::OK,
            "{public} must answer with the management surface disabled"
        );
    }
    assert_eq!(
        status_of(app, get("/management/info")).await,
        StatusCode::NOT_FOUND
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

async fn app_with_metrics() -> Router {
    let config = AppConfig {
        server: ServerConfig {
            swagger_ui: false,
            ..Default::default()
        },
        auth: AuthConfig {
            enabled: false, // exercise the API path without auth in this test
            ..AuthConfig::default()
        },
        ..Default::default()
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
    let (_pg, service) = common::test_service().await;
    ferroehr_rest::build_full(config, service, None, observability).expect("build")
}

#[tokio::test]
async fn prometheus_has_route_template_label_and_no_ids() {
    let app = app_with_metrics().await;

    // Drive an API request that matches a templated route carrying an id. The
    // HTTP metrics layer must label it by the *template*, never the raw id.
    let ehr_id = "3fa85f64-5717-4562-b3fc-2c963f66afa6";
    let _response = app
        .clone()
        .oneshot(get(&format!(
            "/ferroehr/rest/openehr/v1/ehr/{ehr_id}/composition/x"
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
    let config = AppConfig {
        server: ServerConfig {
            swagger_ui: false,
            ..Default::default()
        },
        auth: AuthConfig {
            enabled: false,
            ..AuthConfig::default()
        },
        ..Default::default()
    };
    let observability = Observability {
        management: ManagementConfig {
            enabled: true,
            port: Some(9099),
            endpoints: EndpointLevels {
                info: AccessLevel::Public,
                ..EndpointLevels::default()
            },
            ..ManagementConfig::default()
        },
        ..Observability::default()
    };
    let (_pg, service) = common::test_service().await;
    let main_app = ferroehr_rest::build_full(config, service, None, observability).expect("build");

    // …the main app 404s the management route.
    assert_eq!(
        status_of(main_app.clone(), get("/management/info")).await,
        StatusCode::NOT_FOUND
    );
    // …but the health family is a sibling of the API tree, not part of the
    // management surface, so it stays on the MAIN port (which is what the
    // orchestrator probes point at).
    for public in ["/health", "/health/liveness", "/health/readiness"] {
        assert_eq!(
            status_of(main_app.clone(), get(public)).await,
            StatusCode::OK,
            "{public} must stay on the main listener in separate-port mode"
        );
    }
}

// ── The on-demand CPU flamegraph (`/management/flamegraph`) ─────────────────

/// Build an app with the management surface enabled and only the `flamegraph`
/// endpoint mounted, at `level`, with the given profiling limits.
async fn app_with_flamegraph(
    level: AccessLevel,
    profiling: ferroehr::config::management::ProfilingConfig,
) -> Router {
    let config = AppConfig {
        server: ServerConfig {
            swagger_ui: false,
            ..Default::default()
        },
        auth: auth_config(&["ADMIN"]),
        ..Default::default()
    };
    let observability = Observability {
        management: ManagementConfig {
            enabled: true,
            endpoints: EndpointLevels {
                flamegraph: level,
                ..EndpointLevels::default()
            },
            profiling,
            ..ManagementConfig::default()
        },
        ..Observability::default()
    };
    let (_pg, service) = common::test_service().await;
    ferroehr_rest::build_full(config, service, None, observability).expect("build")
}

/// Off (the default) means the route is simply absent — a `404` answered
/// before authentication.
#[tokio::test]
async fn flamegraph_off_is_404() {
    let app = app_with_flamegraph(
        AccessLevel::Off,
        ferroehr::config::management::ProfilingConfig::default(),
    )
    .await;
    assert_eq!(
        status_of(app, get("/management/flamegraph")).await,
        StatusCode::NOT_FOUND
    );
}

/// Opted in: a short sample window answers `200` with a rendered SVG.
#[tokio::test]
async fn flamegraph_samples_and_renders_svg() {
    let app = app_with_flamegraph(
        AccessLevel::Public,
        ferroehr::config::management::ProfilingConfig::default(),
    )
    .await;
    let response = app
        .oneshot(get("/management/flamegraph?seconds=1&frequency=99"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .expect("content type"),
        "image/svg+xml"
    );
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let text = String::from_utf8(body.to_vec()).expect("svg is utf-8");
    assert!(
        text.contains("<svg"),
        "the body must be a rendered flamegraph SVG, got ({} bytes): {}",
        text.len(),
        text.get(..400).unwrap_or(&text)
    );
}

/// A request beyond a configured cap is refused with `400` — never silently
/// clamped.
#[tokio::test]
async fn flamegraph_over_cap_is_400() {
    let app = app_with_flamegraph(
        AccessLevel::Public,
        ferroehr::config::management::ProfilingConfig::default(),
    )
    .await;
    assert_eq!(
        status_of(app.clone(), get("/management/flamegraph?seconds=31")).await,
        StatusCode::BAD_REQUEST,
        "seconds beyond management.profiling.max_seconds must refuse"
    );
    assert_eq!(
        status_of(app, get("/management/flamegraph?frequency=1000")).await,
        StatusCode::BAD_REQUEST,
        "frequency beyond management.profiling.max_frequency must refuse"
    );
}
