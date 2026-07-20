//! SUT discovery probe: `OPTIONS /` (the ITS-REST conformance endpoint) +
//! lightweight wire observations, run once per SUT before the case sweep.
//!
//! The probe never gates a case by itself (capabilities are evidenced by
//! cases) — it feeds the report's SUT-identity block
//! and gives bring-your-own-endpoint runs a starting edition observation.

use serde::Serialize;

use crate::edition::Edition;
use crate::engine::harness::{HttpRequest, Method, Transport};

/// What the pre-run probe learned about the SUT.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SutProbe {
    /// The `OPTIONS /` conformance body (ITS-REST overview §Conformance),
    /// verbatim JSON when the SUT serves one.
    pub options_conformance: Option<serde_json::Value>,
    /// `solution`/`vendor` fields extracted from the conformance body.
    pub solution: Option<String>,
    /// The `ETag` form observed on a cheap write-free exchange, when one was
    /// observable (an initial edition hint; per-case observations override).
    pub etag_edition_hint: Option<Edition>,
}

/// Run the probe. Failures are absorbed — an endpoint without `OPTIONS /`
/// support still gets the full case sweep; the probe records absence.
pub async fn probe(transport: &dyn Transport) -> SutProbe {
    let mut out = SutProbe::default();
    let req = HttpRequest {
        method: Method::Options,
        path: "/".to_owned(),
        headers: vec![("accept".to_owned(), "application/json".to_owned())],
        body: None,
        auth: crate::engine::harness::AuthSlot::Regular,
    };
    if let Ok(resp) = transport.send(req).await
        && (200..300).contains(&resp.status)
        && let Ok(body) = serde_json::from_slice::<serde_json::Value>(&resp.body)
    {
        out.solution = body
            .get("solution")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        out.options_conformance = Some(body);
    }
    out
}
