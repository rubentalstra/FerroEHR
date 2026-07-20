//! Axum handlers over the platform health model
//! ([`ehrbase::telemetry::health`] — the model lives in the platform,
//! wire here).

use std::collections::BTreeMap;

use axum::Json;
use axum::response::{IntoResponse, Response};
use http::StatusCode;

use ehrbase::telemetry::health::{AggregateHealth, HealthRegistry, HealthStatus};

struct AggregateHealthResponse(AggregateHealth);

impl IntoResponse for AggregateHealthResponse {
    fn into_response(self) -> Response {
        (self.0.http_status(), Json(self.0)).into_response()
    }
}

/// `GET /management/health` — the aggregate over the indicator registry.
pub(super) async fn aggregate(registry: HealthRegistry) -> Response {
    AggregateHealthResponse(registry.evaluate().await).into_response()
}

/// `GET /management/health/liveness` — process-up probe. No I/O: reaching this
/// handler at all means the process is live.
pub(super) fn liveness() -> Response {
    let body = AggregateHealth {
        status: HealthStatus::Up,
        components: BTreeMap::new(),
    };
    (StatusCode::OK, Json(body)).into_response()
}

/// `GET /management/health/readiness` — DB ping + migrations-applied +
/// audit-sender-alive, via the same indicator registry.
pub(super) async fn readiness(registry: HealthRegistry) -> Response {
    AggregateHealthResponse(registry.evaluate().await).into_response()
}
