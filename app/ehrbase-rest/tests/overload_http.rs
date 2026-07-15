//! Ingress overload-shedding tests, driven through the fully assembled router.
//!
//! No openEHR spec governs server overload — this is our own design (RFC 9110
//! §15.6.4 for the `503` status). These tests exercise the real router stack
//! (`build_with`): a bounded-concurrency + load-shed layer scoped to the API
//! subtree. A handler that parks (blocks on a shared [`Barrier`]) holds its
//! in-flight permit so we can observe what happens to further requests:
//!
//! * beyond the `max_in_flight` cap, an API request is shed immediately with
//!   `503 Service Unavailable` + `Retry-After: 1` and the openEHR error body;
//! * `max_in_flight = 0` installs no layer, so concurrency is unbounded;
//! * the public `/status` endpoint is outside the limit and is never shed,
//!   even while the API permit pool is fully saturated.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use ehrbase_rest::access::authn::config::AuthConfig;
use ehrbase_rest::{AppConfig, ServerConfig};

mod common;
use common::{Hooks, Mock};

const BASE: &str = "/ehrbase/rest/openehr/v1";
/// A syntactically valid EHR id (the `ehr_object` handler decodes it before
/// reaching the backend hook).
const EHR: &str = "3fa85f64-5717-4562-b3fc-2c963f66afa6";

/// Auth-disabled config with an explicit in-flight cap.
fn config(max_in_flight: usize) -> AppConfig {
    AppConfig {
        server: ServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            base_path: BASE.to_owned(),
            max_in_flight,
            swagger_ui: false,
            ..Default::default()
        },
        auth: AuthConfig {
            enabled: false,
            basic: None,
            oidc: None,
            admin_scope: None,
            ..AuthConfig::default()
        },
        ..Default::default()
    }
}

/// A router whose `GET /ehr/{id}` handler parks on `barrier` (holding its
/// in-flight permit) and bumps `entered` on entry, so the test can tell exactly
/// how many requests have acquired a permit.
fn parking_app(max_in_flight: usize, entered: &Arc<AtomicUsize>, barrier: &Arc<Barrier>) -> Router {
    let entered = Arc::clone(entered);
    let barrier = Arc::clone(barrier);
    let hooks = Hooks {
        ehr_object: Some(Arc::new(move |_id| {
            entered.fetch_add(1, Ordering::SeqCst);
            // Block until the test releases the barrier; the ConcurrencyLimit
            // permit acquired in poll_ready is held for this whole duration.
            barrier.wait();
            Ok(json!({
                "_type": "EHR",
                "ehr_id": { "_type": "HIER_OBJECT_ID", "value": EHR }
            }))
        })),
        ..Default::default()
    };
    ehrbase_rest::build_with(config(max_in_flight), Arc::new(Mock::with(hooks)))
        .expect("router builds")
}

fn get_ehr() -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(format!("{BASE}/ehr/{EHR}"))
        .body(Body::empty())
        .expect("request")
}

/// Spin until `counter` reaches `target`, failing if it does not within 5s.
async fn wait_for(counter: &AtomicUsize, target: usize) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while counter.load(Ordering::SeqCst) < target {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {target} in-flight requests (saw {})",
            counter.load(Ordering::SeqCst)
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn requests_beyond_limit_are_shed_with_503() {
    let entered = Arc::new(AtomicUsize::new(0));
    // Two parked handlers + this test thread trip the barrier together.
    let barrier = Arc::new(Barrier::new(3));
    let app = parking_app(2, &entered, &barrier);

    // Two requests occupy both permits and park inside the handler.
    let p1 = tokio::spawn(app.clone().oneshot(get_ehr()));
    let p2 = tokio::spawn(app.clone().oneshot(get_ehr()));
    wait_for(&entered, 2).await;

    // The third request finds no free permit and is shed immediately — it never
    // reaches the handler (the entry counter stays at 2).
    let resp = app.clone().oneshot(get_ehr()).await.expect("response");
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        resp.headers()
            .get(header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok()),
        Some("1")
    );
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    let body: Value = serde_json::from_slice(&bytes).expect("json error body");
    // The standard openEHR `{ error, message }` shape.
    assert_eq!(body["error"], "Service Unavailable");
    assert!(body.get("message").and_then(Value::as_str).is_some());
    assert_eq!(
        entered.load(Ordering::SeqCst),
        2,
        "shed request must not run"
    );

    // Release the parked handlers; both complete normally (were not shed).
    barrier.wait();
    assert_eq!(
        p1.await.expect("join").expect("response").status(),
        StatusCode::OK
    );
    assert_eq!(
        p2.await.expect("join").expect("response").status(),
        StatusCode::OK
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn limit_zero_disables_shedding() {
    let entered = Arc::new(AtomicUsize::new(0));
    // Three concurrent handlers + this test thread.
    let barrier = Arc::new(Barrier::new(4));
    let app = parking_app(0, &entered, &barrier);

    let p1 = tokio::spawn(app.clone().oneshot(get_ehr()));
    let p2 = tokio::spawn(app.clone().oneshot(get_ehr()));
    let p3 = tokio::spawn(app.clone().oneshot(get_ehr()));

    // With no limit installed, all three reach the handler concurrently; if any
    // were shed, the entry count would never reach 3 and this would time out.
    wait_for(&entered, 3).await;

    barrier.wait();
    for p in [p1, p2, p3] {
        assert_eq!(
            p.await.expect("join").expect("response").status(),
            StatusCode::OK
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn status_endpoint_is_never_shed() {
    let entered = Arc::new(AtomicUsize::new(0));
    // One parked handler + this test thread.
    let barrier = Arc::new(Barrier::new(2));
    let app = parking_app(1, &entered, &barrier);

    // Saturate the single API permit.
    let parked = tokio::spawn(app.clone().oneshot(get_ehr()));
    wait_for(&entered, 1).await;

    // A second API request is shed — the limit is genuinely saturated.
    let api = app.clone().oneshot(get_ehr()).await.expect("response");
    assert_eq!(api.status(), StatusCode::SERVICE_UNAVAILABLE);

    // …but `/status` is outside the API subtree and answers normally, so an
    // operator can always probe an overloaded server.
    let status_req = Request::builder()
        .method("GET")
        .uri("/ehrbase/rest/status")
        .body(Body::empty())
        .expect("request");
    let status = app.clone().oneshot(status_req).await.expect("response");
    assert_eq!(status.status(), StatusCode::OK);

    barrier.wait();
    assert_eq!(
        parked.await.expect("join").expect("response").status(),
        StatusCode::OK
    );
}
