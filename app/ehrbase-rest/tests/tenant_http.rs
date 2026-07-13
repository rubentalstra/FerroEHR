//! End-to-end HTTP tests for the tenant admin extension API group:
//! the config gate (`RestConfig::tenancy.enabled`), the `200`/`201`/`204`/`404`/
//! `400`/`409`/`501` wire outcomes for the CRUD verbs, and the JSON body shapes
//! — driven through the assembled router with the shared [`Mock`] platform,
//! whose `tenant_*` hooks back an in-memory store so the CRUD round-trips.
//!
//! Design: the tenancy model is spec-silent — our own extension; the surface is
//! our own, config-gated like the event-subscription group, mounted under
//! `/admin/` and dispatching to the `TenantAdapter` extension.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::type_complexity)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use ehrbase_rest::access::authn::config::AuthConfig;
use ehrbase_rest::{RestConfig, TenancyConfig};
use ehrbase_sm::{CallStatusType, SmError};

mod common;
use common::{Hooks, Mock};

const BASE: &str = "/ehrbase/rest/openehr/v1";
const GROUP: &str = "/ehrbase/rest/openehr/v1/admin/tenant";

/// An in-memory tenant store the hooks share, so the CRUD verbs actually
/// round-trip (create → get → update → delete) through the router.
type Store = Arc<Mutex<BTreeMap<Uuid, Value>>>;

/// Build a tenant record from a create/update body.
fn record(id: Uuid, body: &Value) -> Value {
    json!({
        "id": id.to_string(),
        "name": body.get("name").cloned().unwrap_or(Value::Null),
        "system_id": body.get("system_id").cloned().unwrap_or(Value::Null),
        "created_at": "2026-07-11T00:00:00Z",
    })
}

fn not_found(id: Uuid) -> SmError {
    SmError::new(
        CallStatusType::VersionedObjectDoesNotExist,
        format!("tenant {id} does not exist"),
    )
}

/// `{name, system_id}` are both required and non-empty (→ 400 otherwise).
fn require_fields(body: &Value) -> Result<(), SmError> {
    for f in ["name", "system_id"] {
        let ok = body
            .get(f)
            .and_then(Value::as_str)
            .is_some_and(|s| !s.trim().is_empty());
        if !ok {
            return Err(SmError::new(
                CallStatusType::PreconditionViolation,
                format!("`{f}` required"),
            ));
        }
    }
    Ok(())
}

/// Hooks backed by a shared in-memory store.
fn hooks(store: Store) -> Hooks {
    let (s_list, s_create, s_get, s_update, s_delete) = (
        store.clone(),
        store.clone(),
        store.clone(),
        store.clone(),
        store,
    );
    Hooks {
        tenant_list: Some(Arc::new(move || {
            Ok(s_list.lock().unwrap().values().cloned().collect())
        })),
        tenant_create: Some(Arc::new(move |body: Value| {
            require_fields(&body)?;
            let mut map = s_create.lock().unwrap();
            // Unique name → duplicate is a 409 (Conflict).
            if map.values().any(|v| v["name"] == body["name"]) {
                return Err(SmError::new(
                    CallStatusType::CompositionAlreadyExists,
                    "duplicate name",
                ));
            }
            let id = Uuid::now_v7();
            let rec = record(id, &body);
            map.insert(id, rec.clone());
            Ok(rec)
        })),
        tenant_get: Some(Arc::new(move |id: Uuid| {
            s_get
                .lock()
                .unwrap()
                .get(&id)
                .cloned()
                .ok_or_else(|| not_found(id))
        })),
        tenant_update: Some(Arc::new(move |id: Uuid, body: Value| {
            require_fields(&body)?;
            let mut map = s_update.lock().unwrap();
            if !map.contains_key(&id) {
                return Err(not_found(id));
            }
            let rec = record(id, &body);
            map.insert(id, rec.clone());
            Ok(rec)
        })),
        tenant_delete: Some(Arc::new(move |id: Uuid| {
            if s_delete.lock().unwrap().remove(&id).is_some() {
                Ok(())
            } else {
                Err(not_found(id))
            }
        })),
        ..Default::default()
    }
}

fn config(enabled: bool) -> RestConfig {
    RestConfig {
        smart: ehrbase_rest::SmartConfig::default(),
        system: ehrbase_rest::SystemOptionsConfig::default(),
        base_path: BASE.to_owned(),
        swagger_ui: false,
        auth: AuthConfig {
            enabled: false,
            basic: None,
            oidc: None,
            admin_scope: None,
        },
        tenancy: TenancyConfig {
            enabled,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn app(enabled: bool) -> Router {
    let backend = Arc::new(Mock::with(hooks(Store::default())));
    ehrbase_rest::build_with(config(enabled), backend).expect("router builds")
}

/// An enabled app with no hooks → every CRUD call hits the trait's mandatory
/// path (the mock returns `501`).
fn app_unhooked() -> Router {
    let backend = Arc::new(Mock::new());
    ehrbase_rest::build_with(config(true), backend).expect("router builds")
}

async fn send(app: Router, req: Request<Body>) -> (StatusCode, Value) {
    let resp = app.oneshot(req).await.expect("response");
    let status = resp.status();
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, body)
}

fn req(method: &str, uri: &str, body: Option<Value>) -> Request<Body> {
    let builder = Request::builder().method(method).uri(uri);
    match body {
        Some(v) => builder
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&v).unwrap()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

#[tokio::test]
async fn disabled_group_is_404_and_never_touches_backend() {
    let (status, _) = send(app(false), req("GET", GROUP, None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn crud_round_trip() {
    let app = app(true);

    // Empty list.
    let (status, body) = send(app.clone(), req("GET", GROUP, None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);

    // Create — 201 with the stored record + a generated id.
    let create_body = json!({ "name": "acme", "system_id": "acme.example.org" });
    let (status, created) = send(app.clone(), req("POST", GROUP, Some(create_body))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["name"], "acme");
    assert_eq!(created["system_id"], "acme.example.org");
    let id = created["id"].as_str().unwrap().to_owned();

    // Get by id — 200.
    let (status, got) = send(app.clone(), req("GET", &format!("{GROUP}/{id}"), None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got["id"], id);

    // Update — 200, system_id changed.
    let update_body = json!({ "name": "acme", "system_id": "acme-2.example.org" });
    let (status, updated) = send(
        app.clone(),
        req("PUT", &format!("{GROUP}/{id}"), Some(update_body)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["system_id"], "acme-2.example.org");

    // List now has one.
    let (_status, list) = send(app.clone(), req("GET", GROUP, None)).await;
    assert_eq!(list.as_array().unwrap().len(), 1);

    // Delete — 204.
    let (status, _) = send(app.clone(), req("DELETE", &format!("{GROUP}/{id}"), None)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Gone — 404.
    let (status, _) = send(app, req("GET", &format!("{GROUP}/{id}"), None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_without_required_fields_is_400() {
    // Missing system_id.
    let (status, _) = send(app(true), req("POST", GROUP, Some(json!({ "name": "x" })))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // Missing name.
    let (status, _) = send(
        app(true),
        req("POST", GROUP, Some(json!({ "system_id": "s" }))),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn duplicate_name_is_409() {
    let app = app(true);
    let body = json!({ "name": "dup", "system_id": "s" });
    let (status, _) = send(app.clone(), req("POST", GROUP, Some(body.clone()))).await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _) = send(app, req("POST", GROUP, Some(body))).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn unknown_id_is_404() {
    let app = app(true);
    let id = Uuid::now_v7();
    let (status, _) = send(app.clone(), req("GET", &format!("{GROUP}/{id}"), None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(app, req("DELETE", &format!("{GROUP}/{id}"), None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn malformed_id_is_400() {
    let (status, _) = send(app(true), req("GET", &format!("{GROUP}/not-a-uuid"), None)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn unhooked_call_is_501() {
    let (status, _) = send(app_unhooked(), req("GET", GROUP, None)).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
}
