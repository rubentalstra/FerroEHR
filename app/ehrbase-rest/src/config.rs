//! The REST adapter's runtime configuration view.
//!
//! The configuration *data* lives in the platform tree
//! (`ehrbase::config` — one TOML, one schema); this module holds only the
//! adapter-side assembly the router consumes.

use ehrbase::config::server::{AdminConfig, ServerConfig, TenancyConfig};
use ehrbase::config::auth::AuthConfig;
use ehrbase::config::smart::SmartConfig;

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
            ehrbase::telemetry::provenance::ITS_REST
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
