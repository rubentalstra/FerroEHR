//! REST-adapter configuration types.
//!
//! No openEHR spec governs configuration mechanics — our own design. There is **no loader here**: the whole
//! server configuration is one tree ([`crate::config::FerroEhrConfig`]) loaded
//! once by the binary. This module owns the REST-adapter's slice of it:
//!
//! - [`ServerConfig`] — the `[server]` section (the HTTP listener + REST
//!   surface + the `OPTIONS /` System-Options identity).
//! - `AppConfig` — the adapter's runtime view, assembled by the binary (the
//!   composition root) from the root config's `[server]`, `[auth]`, `[admin]`,
//!   `[tenancy]`, `[smart]` sections plus the extension-group mount toggles.
//!   `ferroehr-rest` cannot depend on the `ferroehr` binary crate that owns the
//!   root config, so the binary supplies exactly what the adapter needs
//!   (dependency inversion).
//!
//! [`AdminConfig`] and [`TenancyConfig`] are `[admin]`/`[tenancy]` sections of
//! the root tree that this crate owns and the root references.

use serde::{Deserialize, Serialize};

/// The `[server]` section — the HTTP listener and REST surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    /// Socket address to bind (e.g. `0.0.0.0:8080`).
    pub bind: String,
    /// The ITS-REST base path all API routes hang off.
    pub base_path: String,
    /// Concurrent-request admission cap before the server sheds load with
    /// `503` and `Retry-After` (shed, never queued). `0` disables shedding.
    /// The public `/status`, health, and discovery endpoints are never
    /// limited. No openEHR spec governs server overload — our own design
    /// (RFC 9110 §15.6.4).
    pub max_in_flight: usize,
    /// Serve the Swagger UI + the `OpenAPI` JSON at the REST root. Consider
    /// `false` in production.
    pub swagger_ui: bool,
    /// Permissive CORS (dev only). Production configures explicit origins.
    pub cors_permissive: bool,
    /// This CDR's own openEHR **system identifier** — the identity the
    /// deployment stamps into the data it authors
    /// (`FERROEHR__SERVER__SYSTEM_ID`). Three wire values carry it:
    ///
    /// - `EHR.system_id` at EHR creation — RM ehr
    ///   `master04-ehr_package.adoc` §EHR Identifier Allocation: "the
    ///   `EHR._system_id_` value should be set to the value that would normally
    ///   be used for locally created EHRs";
    /// - the `AUDIT_DETAILS.system_id` server default when the client supplies
    ///   none through `openehr-audit-details` — ITS-REST
    ///   `specifications/docs/overview/Requests_and_responses.md`
    ///   §"openehr-version and openehr-audit-details": "when `system_id` is
    ///   not provided by the client, the server MUST set it to its own
    ///   configured system identifier";
    /// - every `OBJECT_VERSION_ID.creating_system_id` a commit mints — RM
    ///   common `master06-change_control_package.adoc` §Distributed
    ///   Versioning. That value is stored per version, so changing this key
    ///   never rewrites identifiers already committed.
    ///
    /// **Distinct from [`SystemOptionsConfig`] (`[server.identity]`)**: the
    /// identity block is the *display* identity of the `OPTIONS` System-Options
    /// manifest (who supplies the software and which profile it claims);
    /// `system_id` names *which system authored the data*. They are set
    /// independently. With multi-tenancy on, a resolved tenant's own
    /// `system_id` takes precedence over this default for that request.
    ///
    /// Defaults to [`crate::service::DEFAULT_SYSTEM_ID`] — the pre-existing
    /// value, so an unset key is byte-identical to previous behaviour. No
    /// openEHR spec governs the configuration mechanism — our own design; the
    /// specs govern only that a system HAS such an identifier.
    pub system_id: String,
    /// The `OPTIONS /` System-Options manifest identity
    /// (`[server.identity]`). Sourced from config so the public identity and advertised profile
    /// are not string literals in the handler; the live endpoint list is
    /// supplied separately by `ferroehr_rest::router`. This is the *display* identity
    /// of the manifest — the data-authoring identity is [`Self::system_id`].
    pub identity: SystemOptionsConfig,
    /// `[server.tls]` — native TLS termination + client-certificate
    /// authentication (the IHE ATNA ITI-19 node-authentication posture).
    pub tls: TlsConfig,
}

/// Client-certificate (mutual-TLS) policy for the main listener.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ClientAuth {
    /// No client certificate requested.
    #[default]
    Off,
    /// A client certificate is requested and verified when presented;
    /// connections without one are still accepted.
    Optional,
    /// A verified client certificate is mandatory (IHE ATNA ITI-19: mutually
    /// authenticated nodes).
    Required,
}

/// `[server.tls]` — native TLS on the main listener.
///
/// Off by default (deployments commonly terminate TLS at an ingress); with it
/// on, the protocol floor is the rustls safe default (TLS 1.2+, strong suites
/// — the IETF BCP 195 posture), and `client_auth` adds the IHE ATNA ITI-19
/// mutual-TLS node authentication against an explicit trust anchor. The
/// separate-port management listener stays plain HTTP (an internal surface).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct TlsConfig {
    /// Terminate TLS natively (`FERROEHR__SERVER__TLS__ENABLED`).
    pub enabled: bool,
    /// The server certificate chain, PEM (`FERROEHR__SERVER__TLS__CERT_FILE`).
    pub cert_file: Option<String>,
    /// The server private key, PEM (`FERROEHR__SERVER__TLS__KEY_FILE`).
    pub key_file: Option<String>,
    /// Client-certificate policy (`FERROEHR__SERVER__TLS__CLIENT_AUTH`):
    /// `off` | `optional` | `required`.
    pub client_auth: ClientAuth,
    /// The CA bundle client certificates must chain to, PEM
    /// (`FERROEHR__SERVER__TLS__CLIENT_CA_FILE`). Required when `client_auth`
    /// is not `off` — an explicit trust anchor, never the web PKI.
    pub client_ca_file: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0:8080".to_owned(),
            base_path: "/ferroehr/rest/openehr/v1".to_owned(),
            // 256 bounds the worst-case buffered-request memory to a sane
            // envelope (the knee ladder OOM-killed the container at 1024
            // in-flight clinical commits) while still permitting ~10k req/s at
            // 25 ms latency (throughput = in-flight / latency, Little's law).
            max_in_flight: 256,
            swagger_ui: true,
            cors_permissive: false,
            // The service layer's own default, so an unset `[server] system_id`
            // boots exactly as the service does.
            system_id: crate::service::DEFAULT_SYSTEM_ID.to_owned(),
            identity: SystemOptionsConfig::default(),
            tls: TlsConfig::default(),
        }
    }
}

impl ServerConfig {
    /// The Swagger UI mount path, derived from the base path's parent.
    #[must_use]
    pub fn swagger_ui_path(&self) -> String {
        format!("{}/swagger-ui", self.rest_root())
    }

    /// The `OpenAPI` document path.
    #[must_use]
    pub fn openapi_json_path(&self) -> String {
        format!("{}/api-docs/openapi.json", self.rest_root())
    }

    /// The `/ferroehr/rest` root (the base path with the trailing `/openehr/v1`
    /// removed), where status/health/docs live.
    fn rest_root(&self) -> String {
        self.base_path
            .strip_suffix("/openehr/v1")
            .unwrap_or(&self.base_path)
            .to_owned()
    }
}

/// Multi-tenancy configuration (`[tenancy]`).
///
/// Off by default: with `enabled = false` the tenant middleware is never
/// installed, the pool takes no per-acquire hook, and the `/admin/tenant` CRUD
/// answers `404` — a single-tenant deployment is unchanged. When on, each
/// request's tenant is resolved from `claim` (a JWT-claim path; dotted paths
/// walk nested objects) with an optional dev-only `header` override, then
/// applied as `SET ferroehr.tenant_id` for RLS scoping.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TenancyConfig {
    /// Whether multi-tenancy is active.
    pub enabled: bool,
    /// The JWT-claim path carrying the tenant key (a tenant name or uuid). A
    /// dotted path (e.g. `realm_access.tenant`) walks nested claim objects.
    pub claim: String,
    /// Optional dev-only request-header override for the tenant key. When set
    /// and present on the request it wins over the JWT claim. Leave unset in
    /// production (a client-supplied header must not select a tenant).
    pub header: Option<String>,
}

impl Default for TenancyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            claim: "tenant".to_owned(),
            header: None,
        }
    }
}

/// Configuration of the ADMIN API group (`[admin]`; SM `I_ADMIN_SERVICE`).
///
/// NOTE: gating the admin surface behind an opt-in flag — when inactive every
/// admin route answers `405 Method Not Allowed` with an empty `Allow`
/// ("If a method is recognized but not allowed for the target resource, the
/// response SHOULD be `405 Method Not Allowed` status code" —
/// `docs/specs/openehr/ITS-REST/specifications/docs/overview/Requests_and_responses.md`
/// §"HTTP Methods"), never a `403`. Physical, irreversible deletion is
/// dangerous, so the group stays off by default. No openEHR spec governs the
/// gate itself — our own design.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AdminConfig {
    /// Whether the ADMIN API group is active. When `false`, every admin route
    /// answers `405 Method Not Allowed` without touching the backend.
    pub enabled: bool,
}

/// `[server.identity]` — the `OPTIONS` System-Options manifest identity: the
/// **display** identity this deployment advertises (product, vendor, the
/// contract edition and conformance profile it claims).
///
/// Deliberately NOT the data-authoring identity: what a commit stamps into
/// `EHR.system_id`, `AUDIT_DETAILS.system_id`, and
/// `OBJECT_VERSION_ID.creating_system_id` is [`ServerConfig::system_id`]. A
/// rebrand changes this block and nothing in the stored data; a change of
/// `system_id` changes what newly authored data says about its origin and
/// leaves the manifest alone.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
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
            // `solution` (the product) and `vendor` (the organisation) are
            // distinct — they were the same placeholder before the redesign.
            solution: "FerroEHR".to_owned(),
            solution_version: env!("CARGO_PKG_VERSION").to_owned(),
            vendor: "FerroEHR project".to_owned(),
            restapi_specs_version: crate::telemetry::provenance::ITS_REST.to_owned(),
            conformance_profile: crate::telemetry::provenance::CONFORMANCE_PROFILE.to_owned(),
        }
    }
}
