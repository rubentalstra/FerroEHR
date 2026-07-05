//! Server configuration, loaded with `figment` (defaults ← optional TOML file
//! ← environment).

use figment::providers::{Env, Format, Toml};
use figment::{Figment, providers::Serialized};
use serde::{Deserialize, Serialize};

use crate::auth::AuthConfig;

/// Top-level REST server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestConfig {
    /// Socket address to bind (e.g. `0.0.0.0:8080`).
    #[serde(default = "defaults::bind")]
    pub bind: String,
    /// The ITS-REST base path all API routes hang off.
    #[serde(default = "defaults::base_path")]
    pub base_path: String,
    /// Serve the Swagger UI at `{base_path}/../swagger-ui` (and the `OpenAPI` JSON).
    #[serde(default = "defaults::enabled_flag")]
    pub swagger_ui: bool,
    /// Enable a permissive CORS policy (dev). Production should configure an
    /// explicit origin list (Stage 2).
    #[serde(default)]
    pub cors_permissive: bool,
    /// Authentication configuration.
    #[serde(default)]
    pub auth: AuthConfig,
}

impl Default for RestConfig {
    fn default() -> Self {
        Self {
            bind: defaults::bind(),
            base_path: defaults::base_path(),
            swagger_ui: true,
            cors_permissive: false,
            auth: AuthConfig::default(),
        }
    }
}

impl RestConfig {
    /// Load configuration: defaults, then an optional TOML file (path in
    /// `EHRBASE_REST_CONFIG`), then `EHRBASE_REST_`-prefixed environment
    /// variables (nested keys use `__`, e.g. `EHRBASE_REST_AUTH__ENABLED`).
    ///
    /// # Errors
    /// Returns a [`figment::Error`] if a value fails to parse.
    #[allow(clippy::result_large_err)] // figment::Error is large by design
    pub fn load() -> Result<Self, figment::Error> {
        let mut fig = Figment::from(Serialized::defaults(RestConfig::default()));
        if let Ok(path) = std::env::var("EHRBASE_REST_CONFIG") {
            fig = fig.merge(Toml::file(path));
        }
        fig.merge(Env::prefixed("EHRBASE_REST_").split("__"))
            .extract()
    }

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

mod defaults {
    pub(super) fn bind() -> String {
        "0.0.0.0:8080".to_owned()
    }
    pub(super) fn base_path() -> String {
        "/ehrbase/rest/openehr/v1".to_owned()
    }
    pub(super) const fn enabled_flag() -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let c = RestConfig::default();
        assert_eq!(c.base_path, "/ehrbase/rest/openehr/v1");
        assert!(c.auth.enabled);
        assert_eq!(c.swagger_ui_path(), "/ehrbase/rest/swagger-ui");
        assert_eq!(c.openapi_json_path(), "/ehrbase/rest/api-docs/openapi.json");
    }

    #[test]
    #[allow(clippy::result_large_err)] // figment::Jail closure signature
    fn env_overrides_apply() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("EHRBASE_REST_BIND", "127.0.0.1:9000");
            jail.set_env("EHRBASE_REST_AUTH__ENABLED", "false");
            let c = RestConfig::load().expect("load");
            assert_eq!(c.bind, "127.0.0.1:9000");
            assert!(!c.auth.enabled);
            Ok(())
        });
    }
}
