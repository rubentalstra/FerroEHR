//! End-to-end HTTP tests for the event-subscription admin extension API group
//!: the config gate
//! (`RestConfig::event_subscription.enabled`), the `200`/`201`/`204`/`404`/`400`/
//! `501` wire outcomes for the CRUD verbs, and the JSON body shapes — driven
//! through the assembled router with the shared [`Mock`] platform, whose
//! `event_subscription_*` hooks back an in-memory store so the CRUD round-trips.
//!
//! Design: event/subscription semantics are spec-silent; the surface is our own, config-gated like the terminology
//! group, mounted under `/admin/` and dispatching to the
//! `EventSubscriptionAdapter` extension.
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
use ehrbase_rest::{AppConfig, ServerConfig};
use ehrbase_sm::{CallStatusType, SmError};

mod common;
use common::{Hooks, Mock};

const BASE: &str = "/ehrbase/rest/openehr/v1";
const GROUP: &str = "/ehrbase/rest/openehr/v1/admin/event_subscription";

/// An in-memory subscription store the hooks share, so the CRUD verbs actually
/// round-trip (create → get → update → delete) through the router.
type Store = Arc<Mutex<BTreeMap<Uuid, Value>>>;

/// Build a subscription record from a create/update body (predicates default to
/// JSON `null` = wildcard; enabled defaults to true).
fn record(id: Uuid, body: &Value) -> Value {
    let pred = |k: &str| body.get(k).cloned().unwrap_or(Value::Null);
    json!({
        "id": id.to_string(),
        "name": body.get("name").cloned().unwrap_or(Value::Null),
        "kind": pred("kind"),
        "change_type": pred("change_type"),
        "template_id": pred("template_id"),
        "archetype": pred("archetype"),
        "enabled": body.get("enabled").and_then(Value::as_bool).unwrap_or(true),
        "created_at": "2026-07-10T00:00:00Z",
    })
}

fn not_found(id: Uuid) -> SmError {
    SmError::new(
        CallStatusType::VersionedObjectDoesNotExist,
        format!("event subscription {id} does not exist"),
    )
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
        event_subscription_list: Some(Arc::new(move || {
            Ok(s_list.lock().unwrap().values().cloned().collect())
        })),
        event_subscription_create: Some(Arc::new(move |body: Value| {
            // `name` required — a missing/blank one is a client error (400).
            let name = body
                .get("name")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    SmError::new(CallStatusType::PreconditionViolation, "name required")
                })?;
            let mut map = s_create.lock().unwrap();
            // Unique name → duplicate is a 409 (Conflict).
            if map.values().any(|v| v["name"] == json!(name)) {
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
        event_subscription_get: Some(Arc::new(move |id: Uuid| {
            s_get
                .lock()
                .unwrap()
                .get(&id)
                .cloned()
                .ok_or_else(|| not_found(id))
        })),
        event_subscription_update: Some(Arc::new(move |id: Uuid, body: Value| {
            let mut map = s_update.lock().unwrap();
            if !map.contains_key(&id) {
                return Err(not_found(id));
            }
            // Name is immutable — keep the stored name.
            let name = map[&id]["name"].clone();
            let mut rec = record(id, &body);
            rec["name"] = name;
            map.insert(id, rec.clone());
            Ok(rec)
        })),
        event_subscription_delete: Some(Arc::new(move |id: Uuid| {
            if s_delete.lock().unwrap().remove(&id).is_some() {
                Ok(())
            } else {
                Err(not_found(id))
            }
        })),
        ..Default::default()
    }
}

fn config(enabled: bool) -> AppConfig {
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
        events_admin_api: enabled,
        ..Default::default()
    }
}

fn app(enabled: bool) -> Router {
    let backend = Arc::new(Mock::with(hooks(Store::default())));
    ehrbase_rest::build_with(config(enabled), backend).expect("router builds")
}

/// An enabled app with no hooks → every call hits the trait's mandatory path
/// (the mock returns `501`).
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
    let create_body =
        json!({ "name": "vitals", "kind": "COMPOSITION", "template_id": "vitals.v2" });
    let (status, created) = send(app.clone(), req("POST", GROUP, Some(create_body))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["name"], "vitals");
    assert_eq!(created["kind"], "COMPOSITION");
    assert_eq!(created["template_id"], "vitals.v2");
    // Unset predicates are wildcards (null).
    assert_eq!(created["change_type"], Value::Null);
    assert_eq!(created["enabled"], true);
    let id = created["id"].as_str().unwrap().to_owned();

    // Get by id — 200.
    let (status, got) = send(app.clone(), req("GET", &format!("{GROUP}/{id}"), None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got["id"], id);

    // Update — 200, predicates replaced, name immutable.
    let update_body = json!({ "name": "ignored", "kind": "EHR_STATUS", "enabled": false });
    let (status, updated) = send(
        app.clone(),
        req("PUT", &format!("{GROUP}/{id}"), Some(update_body)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["name"], "vitals", "name is immutable");
    assert_eq!(updated["kind"], "EHR_STATUS");
    assert_eq!(updated["enabled"], false);

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
async fn create_without_name_is_400() {
    let (status, _) = send(
        app(true),
        req("POST", GROUP, Some(json!({ "kind": "COMPOSITION" }))),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn duplicate_name_is_409() {
    let app = app(true);
    let body = json!({ "name": "dup" });
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
