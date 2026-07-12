//! `OPTIONS /` — **System Options and Conformance** (ITS-REST System API, the
//! single operation `operationId: options`).
//!
//! Oracle:
//! `docs/specs/openehr/ITS-REST/computable/OAS/system-codegen.openapi.yaml`
//! (`servers` `https://{baseUrl}/v1`, `paths` `/` `options`, `security: []`,
//! the `Options` schema, the `Allow` header, the `200_options` response) and
//! `docs/specs/openehr/ITS-REST/specifications/docs/system/Description.md`.
//!
//! The System API is **not** part of the generated ITS-REST contract — the
//! `emit-rest` groups are `ehr`/`query`/`definition`/`admin`/`demographic`
//! only (`crates/openehr-its/src/rest/generated/` has no `system` group) — so
//! this one standalone operation is hand-written here, correctly.
//!
//! This module owns the manifest's *shape and content*; mounting it on the
//! router is the wiring layer's job (see the `TODO(w3e-integrate)` notes on
//! [`route`] and [`SystemOptionsConfig`]). The register the redesign closes is
//! `docs/design/its-rest/system.md` §1 (G-1..G-6).

use std::sync::Arc;

use axum::response::Response;
use axum::routing::{MethodRouter, options};
use http::{HeaderMap, HeaderValue, StatusCode, header};
use serde::Serialize;

use crate::overview::negotiate;

/// The openEHR ITS-REST version this server targets. The OAS `example` value
/// (`1.1.0`, `system-codegen.openapi.yaml` line 114) is illustrative only, not
/// normative — our conformance target is Release 1.0.3.
const OPENEHR_REST_API_VERSION: &str = "1.0.3";

/// The HTTP methods this API surface supports — the `Allow` header the OAS
/// `200_options` response carries (`system-codegen.openapi.yaml` `headers.Allow`,
/// example `GET, POST, PUT, DELETE, OPTIONS`).
const ALLOW_METHODS: &str = "GET, POST, PUT, DELETE, OPTIONS";

/// The API groups the ITS-REST resource specifications define, and that the
/// OAS `Options.endpoints` `example` enumerates
/// (`system-codegen.openapi.yaml` lines 116-121).
///
/// This is the spec-defined **default** only. G-1 requires the *live* list —
/// exactly the groups the router mounts — so the wiring layer passes its
/// actual mounted-group set to [`SystemManifest::new`] rather than relying on
/// this constant (see the `TODO(w3e-integrate)` on [`route`]).
pub const SPEC_ENDPOINTS: &[&str] = &["/ehr", "/demographic", "/definition", "/query", "/admin"];

/// The default conformance profile advertised in the manifest.
///
/// PORT NOTE (`docs/design/its-rest/system.md` G-2): this is a *documented
/// default*, not a value the handler hardcodes — [`SystemManifest`] always
/// reads [`SystemOptionsConfig::conformance_profile`]. The authoritative
/// verdict is the conformance runner's machine-computed profile
/// (`tools/conformance` `reporting/report.rs` / `master03-profiles.adoc`); the
/// manifest MUST NOT out-claim it. The default is the highest profile the CDR
/// targets; the wiring layer should override it from build-time ECC badge data
/// (see the `TODO(w3e-integrate)` on [`SystemOptionsConfig::default`]).
const DEFAULT_CONFORMANCE_PROFILE: &str = "STANDARD";

/// Identity + conformance fields of the System-Options manifest, sourced from
/// configuration so the public identity (G-6) and the advertised conformance
/// profile (G-2) are not string literals baked into the handler.
///
/// TODO(w3e-integrate): `crate::config::RestConfig` should carry (or embed) a
/// `SystemOptionsConfig`, populated from `EHRBASE_*` config keys, and the
/// wiring layer should thread it into [`SystemManifest::new`]. The
/// `conformance_profile` default in particular should be derived from the
/// committed conformance run's machine verdict — a `build.rs` constant read
/// from `docs/conformance/results.json` (like `git_sha`), so the manifest
/// states a *measured* profile — rather than the compile-time default here.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SystemOptionsConfig {
    /// `Options.solution` — the product name.
    pub solution: String,
    /// `Options.solution_version` — the product version.
    pub solution_version: String,
    /// `Options.vendor` — the organisation providing the solution.
    pub vendor: String,
    /// `Options.restapi_specs_version` — the ITS-REST version targeted.
    pub restapi_specs_version: String,
    /// `Options.conformance_profile` — CORE / STANDARD (CNF master03 profiles).
    pub conformance_profile: String,
}

impl Default for SystemOptionsConfig {
    fn default() -> Self {
        Self {
            // G-6: `solution` (the product) and `vendor` (the organisation) are
            // distinct — they were the same placeholder before the redesign.
            solution: "EHRbase-RS".to_owned(),
            solution_version: env!("CARGO_PKG_VERSION").to_owned(),
            vendor: "EHRbase-RS project".to_owned(),
            restapi_specs_version: OPENEHR_REST_API_VERSION.to_owned(),
            conformance_profile: DEFAULT_CONFORMANCE_PROFILE.to_owned(),
        }
    }
}

/// The wire body of the `OPTIONS /` response — the OAS `Options` schema
/// (`system-codegen.openapi.yaml` lines 92-121): `solution`,
/// `solution_version`, `vendor`, `restapi_specs_version`,
/// `conformance_profile`, `endpoints: [string]`.
///
/// Borrows from the [`SystemManifest`] so the manifest owns the data and this
/// is a zero-copy view rendered per request.
#[derive(Debug, Serialize)]
struct Options<'a> {
    solution: &'a str,
    solution_version: &'a str,
    vendor: &'a str,
    restapi_specs_version: &'a str,
    conformance_profile: &'a str,
    endpoints: &'a [String],
}

/// The immutable System-Options manifest, built once at wiring time from
/// [`SystemOptionsConfig`] plus the live mounted-group list, then shared
/// (behind an [`Arc`]) by the `OPTIONS /` handler.
#[derive(Debug, Clone)]
pub struct SystemManifest {
    config: SystemOptionsConfig,
    endpoints: Vec<String>,
}

impl SystemManifest {
    /// Build a manifest from its identity/conformance config and the endpoint
    /// list the server actually serves.
    ///
    /// G-1: `endpoints` is the *live* mounted-group set — the wiring layer
    /// passes what the router mounts (see [`route`]), so the manifest never
    /// advertises less (or more) than the server serves. Duplicates are
    /// removed and order preserved; use [`SPEC_ENDPOINTS`] as the spec-defined
    /// default when a live list is not available.
    pub fn new(config: SystemOptionsConfig, endpoints: impl IntoIterator<Item = String>) -> Self {
        let mut seen = Vec::new();
        for ep in endpoints {
            if !seen.contains(&ep) {
                seen.push(ep);
            }
        }
        Self {
            config,
            endpoints: seen,
        }
    }

    /// A borrowing view of the manifest as the OAS `Options` body.
    fn body(&self) -> Options<'_> {
        Options {
            solution: &self.config.solution,
            solution_version: &self.config.solution_version,
            vendor: &self.config.vendor,
            restapi_specs_version: &self.config.restapi_specs_version,
            conformance_profile: &self.config.conformance_profile,
            endpoints: &self.endpoints,
        }
    }

    /// Render the `OPTIONS /` response: a `200 OK` with the `Allow` header and
    /// the `Options` body, honouring `Accept` (G-5).
    ///
    /// Content negotiation goes through [`crate::overview::negotiate::respond`]:
    /// JSON for `application/json`/`*/*`/absent `Accept` (the OAS constrains
    /// `Accept` to `application/json`, `system-codegen.openapi.yaml`
    /// `Accept_JSON` lines 71-78), and `406 Not Acceptable` for an
    /// exclusively-XML `Accept` — the `Options` manifest is not a spec-typed RM
    /// object and has no canonical-XML shape, so XML is genuinely not offered.
    /// The `Allow` header is attached only to the successful (`200`) response.
    pub fn respond(&self, headers: &HeaderMap) -> Response {
        let mut resp = negotiate::respond(headers, StatusCode::OK, &self.body());
        if resp.status() == StatusCode::OK {
            resp.headers_mut()
                .insert(header::ALLOW, HeaderValue::from_static(ALLOW_METHODS));
        }
        resp
    }
}

/// Build the `OPTIONS` [`MethodRouter`] serving the System-Options manifest.
///
/// The handler is stateless (the manifest is captured), so the returned
/// `MethodRouter<S>` composes with a router of any state type. It is generic
/// so the wiring layer can mount it on the `AppState`-typed application router.
///
/// TODO(w3e-integrate): `crate::router` should
///   1. build a [`SystemManifest`] from the server config (a
///      [`SystemOptionsConfig`]) and the **live** mounted-group list — the
///      groups `crate::api::api_router` actually merges (`/ehr`,
///      `/demographic`, `/definition`, `/query`, `/admin`, plus any
///      config-gated extension surfaces it chooses to advertise), not
///      [`SPEC_ENDPOINTS`] hardcoded (closes G-1);
///   2. mount `system::options::route(manifest)` at the **API base-path root**
///      (`cfg.base_path`, e.g. `OPTIONS /ehrbase/rest/openehr/v1`) — the root
///      the OAS `servers`/`paths` describe (closes G-3);
///   3. keep the existing bare-`/` mount as a compatibility alias for naive
///      probes (`docs/design/its-rest/system.md` §2.4 — harmless, helps naive
///      clients);
///   4. mount both **above** the `CorsLayer` (that layer treats every
///      `OPTIONS` as a CORS preflight and short-circuits it — the reason the
///      current handler is added after the middleware stack in
///      `router.rs:118-121`).
pub fn route<S>(manifest: Arc<SystemManifest>) -> MethodRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    options(move |headers: HeaderMap| {
        let manifest = Arc::clone(&manifest);
        async move { manifest.respond(&headers) }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
                http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                HeaderValue::from_str(v).unwrap(),
            );
        }
        h
    }

    fn manifest() -> SystemManifest {
        SystemManifest::new(
            SystemOptionsConfig::default(),
            SPEC_ENDPOINTS.iter().map(|s| (*s).to_owned()),
        )
    }

    async fn body_json(resp: Response) -> serde_json::Value {
        let bytes = resp.into_body().collect().await.expect("body").to_bytes();
        serde_json::from_slice(&bytes).expect("json body")
    }

    #[tokio::test]
    async fn options_returns_200_with_allow_header_and_all_six_fields() {
        let resp = manifest().respond(&HeaderMap::new());
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::ALLOW).unwrap(),
            ALLOW_METHODS,
            "Allow header enumerates supported methods"
        );
        assert_eq!(
            resp.headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("application/json"),
        );

        let v = body_json(resp).await;
        // All six OAS `Options` properties are present and correctly typed.
        assert!(v["solution"].is_string());
        assert!(v["solution_version"].is_string());
        assert!(v["vendor"].is_string());
        assert_eq!(v["restapi_specs_version"], "1.0.3");
        assert_eq!(v["conformance_profile"], "STANDARD");
        assert!(v["endpoints"].is_array());
    }

    #[tokio::test]
    async fn solution_and_vendor_are_distinct_and_config_driven() {
        // G-6: `solution` (product) and `vendor` (organisation) differ, and
        // both come from config — not a shared placeholder.
        let cfg = SystemOptionsConfig {
            solution: "MySolution".to_owned(),
            vendor: "MyOrg".to_owned(),
            ..SystemOptionsConfig::default()
        };
        let m = SystemManifest::new(cfg, SPEC_ENDPOINTS.iter().map(|s| (*s).to_owned()));
        let v = body_json(m.respond(&HeaderMap::new())).await;
        assert_eq!(v["solution"], "MySolution");
        assert_eq!(v["vendor"], "MyOrg");
        assert_ne!(v["solution"], v["vendor"]);
    }

    #[tokio::test]
    async fn conformance_profile_comes_from_config_not_a_literal() {
        // G-2: the handler reports whatever the config carries — so it can be
        // reconciled with / capped at the runner's machine verdict.
        let cfg = SystemOptionsConfig {
            conformance_profile: "CORE".to_owned(),
            ..SystemOptionsConfig::default()
        };
        let m = SystemManifest::new(cfg, SPEC_ENDPOINTS.iter().map(|s| (*s).to_owned()));
        let v = body_json(m.respond(&HeaderMap::new())).await;
        assert_eq!(v["conformance_profile"], "CORE");
    }

    #[tokio::test]
    async fn endpoints_reflect_the_supplied_live_list() {
        // G-1: the manifest advertises exactly the groups it was built with.
        let live = ["/ehr", "/query", "/definition", "/demographic", "/admin"];
        let m = SystemManifest::new(
            SystemOptionsConfig::default(),
            live.iter().map(|s| (*s).to_owned()),
        );
        let v = body_json(m.respond(&HeaderMap::new())).await;
        let got: Vec<String> = v["endpoints"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(got, live);
    }

    #[tokio::test]
    async fn duplicate_endpoints_are_deduplicated_order_preserved() {
        let m = SystemManifest::new(
            SystemOptionsConfig::default(),
            ["/ehr", "/query", "/ehr", "/admin"]
                .iter()
                .map(|s| (*s).to_owned()),
        );
        let v = body_json(m.respond(&HeaderMap::new())).await;
        let got: Vec<String> = v["endpoints"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(got, ["/ehr", "/query", "/admin"]);
    }

    #[tokio::test]
    async fn json_accept_is_honoured() {
        let resp = manifest().respond(&headers(&[("accept", "application/json")]));
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn xml_only_accept_is_not_acceptable() {
        // G-5: `Options` has no canonical-XML shape (it is not a spec-typed RM
        // object); an exclusively-XML `Accept` negotiates to 406.
        let resp = manifest().respond(&headers(&[("accept", "application/xml")]));
        assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
        // The failed negotiation carries no `Allow` header.
        assert!(resp.headers().get(header::ALLOW).is_none());
    }
}
