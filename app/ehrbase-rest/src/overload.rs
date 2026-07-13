//! Ingress overload protection: bounded in-flight concurrency + load shedding.
//!
//! No openEHR spec governs server overload behaviour — this is our own design
//! (RFC 9110 §15.6.4 is the HTTP authority for the `503 Service Unavailable`
//! status). Under sustained offered load beyond backend (DB pool) capacity, a
//! server that accepts and queues every request unboundedly — each awaiting a
//! `sqlx` pool connection for up to the acquire timeout — grows its in-flight
//! set until the process is killed for exhausting memory. Instead the router
//! caps the number of concurrently in-flight API requests and sheds the excess
//! *immediately* with `503` + `Retry-After`, so the server degrades with clean
//! errors rather than dying.
//!
//! The mechanism is entirely stock `tower`: [`tower::limit::ConcurrencyLimit`]
//! holds a shared semaphore and only admits a request once it can acquire a
//! permit (the permit is held for the request's whole lifetime);
//! [`tower::load_shed::LoadShed`] wraps it and — because `ConcurrencyLimit`
//! reports "not ready" when no permit is free — returns an
//! [`Overloaded`](tower::load_shed::error::Overloaded) error synchronously
//! instead of waiting. [`axum::error_handling::HandleErrorLayer`] turns that
//! error back into an infallible response via [`handle_overload`]. The layer is
//! wired in [`crate::router`]; see that module's doc for the layer order.

use axum::BoxError;
use axum::response::{IntoResponse, Response};
use http::{HeaderValue, header};
use tower::load_shed::error::Overloaded;

use openehr_its::rest::runtime::ApiError;

use crate::overview::error::RestError;

/// The `Retry-After` hint (in seconds) sent on a shed response — a short,
/// fixed backoff (the load is transient by definition).
const RETRY_AFTER_SECONDS: &str = "1";

/// [`HandleErrorLayer`](axum::error_handling::HandleErrorLayer) handler for the
/// overload-shedding stack: map a shed request to `503 Service Unavailable`
/// with the standard openEHR `{ error, message }` body (via [`RestError`]) and
/// a `Retry-After` header (RFC 9110 §15.6.4). Any other error is not expected
/// from this stack — `ConcurrencyLimit` over the infallible router only ever
/// yields [`Overloaded`] — so it degrades to a `500` defensively.
pub(crate) async fn handle_overload(err: BoxError) -> Response {
    if err.is::<Overloaded>() {
        let mut resp = RestError(ApiError::ServiceUnavailable(
            "the server is temporarily overloaded; retry shortly".to_owned(),
        ))
        .into_response();
        resp.headers_mut().insert(
            header::RETRY_AFTER,
            HeaderValue::from_static(RETRY_AFTER_SECONDS),
        );
        resp
    } else {
        RestError(ApiError::Internal(format!(
            "unexpected overload-layer error: {err}"
        )))
        .into_response()
    }
}
