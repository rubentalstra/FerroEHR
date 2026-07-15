//! REST-adapter configuration types.
//!
//! No openEHR spec governs configuration mechanics — our own design
//! (`docs/design/configuration.md`). There is **no loader here**: the whole
//! server configuration is one tree ([`ehrbase::config::EhrbaseConfig`]) loaded
//! once by the binary. This module owns the REST-adapter's slice of it:
//!
//! - [`ServerConfig`] — the `[server]` section (the HTTP listener + REST
//!   surface + the `OPTIONS /` System-Options identity, §3.1).
//! - [`AppConfig`] — the adapter's runtime view, assembled by the binary (the
//!   composition root) from the root config's `[server]`, `[auth]`, `[admin]`,
//!   `[tenancy]`, `[smart]` sections plus the extension-group mount toggles.
//!   `ehrbase-rest` cannot depend on the `ehrbase` binary crate that owns the
//!   root config, so the binary supplies exactly what the adapter needs
//!   (dependency inversion).
//!
//! [`AdminConfig`] and [`TenancyConfig`] are `[admin]`/`[tenancy]` sections of
//! the root tree that this crate owns and the root references.

use serde::{Deserialize, Serialize};

use crate::api::system::options::SystemOptionsConfig;
use crate::extensions::access::authn::AuthConfig;
use crate::smart::config::SmartConfig;

/// The `[server]` section — the HTTP listener and REST surface (§3.1).
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
    /// The `OPTIONS /` System-Options manifest identity (`[server.identity]`,
    /// §3.1). Sourced from config so the public identity and advertised profile
    /// are not string literals in the handler; the live endpoint list is
    /// supplied separately by [`crate::router`].
    pub identity: SystemOptionsConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            base_path: default_base_path(),
            max_in_flight: default_max_in_flight(),
            swagger_ui: true,
            cors_permissive: false,
            identity: SystemOptionsConfig::default(),
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

    /// The `/ehrbase/rest` root (the base path with the trailing `/openehr/v1`
    /// removed), where status/health/docs live.
    fn rest_root(&self) -> String {
        self.base_path
            .strip_suffix("/openehr/v1")
            .unwrap_or(&self.base_path)
            .to_owned()
    }
}

fn default_bind() -> String {
    "0.0.0.0:8080".to_owned()
}

fn default_base_path() -> String {
    "/ehrbase/rest/openehr/v1".to_owned()
}

const fn default_max_in_flight() -> usize {
    // 256 bounds the worst-case buffered-request memory to a sane envelope
    // (the W-11 knee ladder OOM-killed the container at 1024 in-flight clinical
    // commits) while still permitting ~10k req/s at 25 ms latency
    // (throughput = in-flight / latency, Little's law).
    256
}

/// Multi-tenancy configuration (`[tenancy]`, §3.8).
///
/// Off by default: with `enabled = false` the tenant middleware is never
/// installed, the pool takes no per-acquire hook, and the `/admin/tenant` CRUD
/// answers `404` — a single-tenant deployment is unchanged. When on, each
/// request's tenant is resolved from `claim` (a JWT-claim path; dotted paths
/// walk nested objects) with an optional dev-only `header` override, then
/// applied as `SET ehrbase.tenant_id` for RLS scoping.
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

/// Configuration of the ADMIN API group (`[admin]`, §3.7; SM `I_ADMIN_SERVICE`).
///
/// PORT NOTE: gating the admin surface behind an opt-in flag — when inactive
/// the admin controllers are not registered, so the routes are absent (`404`),
/// never a `403`. Physical, irreversible deletion is dangerous, so the group
/// stays off by default. No openEHR spec governs this gate — our own design.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AdminConfig {
    /// Whether the ADMIN API group is active. When `false`, every admin route
    /// answers `404` without touching the backend.
    pub enabled: bool,
}

/// The REST adapter's runtime configuration view, assembled by the binary from
/// the root [`ehrbase::config::EhrbaseConfig`]. Not a serde root — it is built
/// in code, so extension-group toggles are plain `bool`s lifted from their
/// owning sections (`[fhir].api_enabled`, `[terminology].api_enabled`,
/// `[events].admin_api`).
#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    /// `[server]` — listener + REST surface + System-Options identity.
    pub server: ServerConfig,
    /// `[auth]` — authentication (Basic + OAuth2/OIDC).
    pub auth: AuthConfig,
    /// `[admin]` — the ADMIN API group toggle.
    pub admin: AdminConfig,
    /// `[tenancy]` — multi-tenancy.
    pub tenancy: TenancyConfig,
    /// `[smart]` — SMART App Launch resource-server posture.
    pub smart: SmartConfig,
    /// `[fhir].api_enabled` — mount the FHIR R4 inbound façade + admin mapping.
    pub fhir_api_enabled: bool,
    /// `[terminology].api_enabled` — mount the terminology extension API.
    pub terminology_api_enabled: bool,
    /// `[events].admin_api` — mount the `/admin/event_subscription` CRUD.
    pub events_admin_api: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_defaults_are_sane() {
        let c = ServerConfig::default();
        assert_eq!(c.base_path, "/ehrbase/rest/openehr/v1");
        assert_eq!(c.max_in_flight, 256);
        assert!(c.swagger_ui);
        assert!(!c.cors_permissive);
        assert_eq!(c.swagger_ui_path(), "/ehrbase/rest/swagger-ui");
        assert_eq!(c.openapi_json_path(), "/ehrbase/rest/api-docs/openapi.json");
        // The System-Options identity carries the tested development-edition
        // contract identity (shared provenance), not the retired `1.0.3` label.
        assert_eq!(
            c.identity.restapi_specs_version,
            crate::extensions::provenance::ITS_REST
        );
    }

    #[test]
    fn app_config_defaults_off() {
        let c = AppConfig::default();
        assert!(c.auth.enabled);
        assert!(!c.admin.enabled);
        assert!(!c.tenancy.enabled);
        assert!(!c.smart.enabled);
        assert!(!c.fhir_api_enabled);
        assert!(!c.terminology_api_enabled);
        assert!(!c.events_admin_api);
    }
}
