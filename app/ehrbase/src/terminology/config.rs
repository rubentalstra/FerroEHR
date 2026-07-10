//! External-terminology configuration (`figment`), matching the shape in
//! `docs/terminology-validation.md` §4.
//!
//! Loaded from defaults ← optional TOML file (`EHRBASE_VALIDATION_CONFIG`) ←
//! `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_`-prefixed environment (nested keys
//! use `__`, e.g. `..._PROVIDERS__DEFAULT__URL`) — the same env grammar the
//! Docker compose recipe in `docs/design/terminology-server-integration.md` §3
//! writes.
//!
//! Provider selection is **openEHR-bundle-by-default, FHIR opt-in**: with
//! [`ExternalTerminologyConfig::enabled`] `false` (the default, matching
//! `EHRBase`) no remote provider is built and composition validation stays on
//! the in-process `openehr-term` bundle; a FHIR provider is materialised only
//! when a deployment opts in.

use std::collections::BTreeMap;

use figment::Figment;
use figment::providers::{Env, Format, Serialized, Toml};
use serde::{Deserialize, Serialize};

use ehrbase_sm::SmError;

use super::fhir::FhirTerminologyProvider;

/// Default per-provider connect timeout (ms).
const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 2_000;
/// Default per-provider request timeout (ms).
const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 10_000;
/// The provider name selected by [`ExternalTerminologyConfig::default_provider`]
/// when several are configured.
const DEFAULT_PROVIDER_NAME: &str = "default";

/// External-terminology validation configuration (`[validation.external_terminology]`).
///
/// Defaults are the `EHRBase`-matching off state: `enabled = false`,
/// `fail_on_error = false`, no providers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExternalTerminologyConfig {
    /// Whether external-terminology validation is active. When `false` (the
    /// default, matching `EHRBase`), no remote provider is built.
    #[serde(default)]
    pub enabled: bool,
    /// On a terminology-server/connectivity error: `true` = reject the
    /// composition (fail-closed); `false` = accept it (fail-open, matching
    /// `EHRBase`'s default). Consumed by the P15 validation walker — the raw
    /// provider always surfaces the error; this flag decides how the caller
    /// treats it.
    #[serde(default)]
    pub fail_on_error: bool,
    /// The configured terminology-server providers, keyed by name.
    #[serde(default)]
    pub providers: BTreeMap<String, FhirProviderConfig>,
}

/// The kind of terminology server. Only FHIR R4 is supported, matching the
/// reference (`docs/terminology-validation.md` §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// A FHIR R4 terminology server (`$validate-code`/`$expand`/`$subsumes`/
    /// `$lookup`).
    #[default]
    Fhir,
}

/// The FHIR operation used for value-set membership (`value_set_validate`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FhirOperation {
    /// FHIR `ValueSet/$validate-code` — a direct yes/no (least payload).
    #[default]
    ValidateCode,
    /// FHIR `ValueSet/$expand` + a membership test — the fallback for servers
    /// that lack `$validate-code`.
    Expand,
}

/// Configuration for a single FHIR R4 terminology-server provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FhirProviderConfig {
    /// Server kind (`type = "fhir"`).
    #[serde(rename = "type", default)]
    pub kind: ProviderKind,
    /// FHIR R4 base URL, e.g. `https://r4.ontoserver.csiro.au/fhir`.
    pub url: String,
    /// The membership operation for `value_set_validate` (default
    /// `validate_code`).
    #[serde(default)]
    pub operation: FhirOperation,
    /// TCP connect timeout (ms).
    #[serde(default = "default_connect_timeout_ms")]
    pub connect_timeout_ms: u64,
    /// Overall request timeout (ms).
    #[serde(default = "default_request_timeout_ms")]
    pub request_timeout_ms: u64,
    /// Optional name of an `OAuth2` client-credentials client to authenticate to
    /// the TS with.
    ///
    /// PORT NOTE: `OAuth2` client-credentials + mutual-TLS to the TS
    /// (`docs/terminology-validation.md` §3) are a follow-up on top of this
    /// task's core `$validate-code`/`$expand`/`$subsumes`/`$lookup` provider;
    /// the field is accepted so config written for the full design parses, but
    /// no bearer token is attached yet. A configured value that would silently
    /// send unauthenticated requests is surfaced at build time
    /// ([`FhirTerminologyProvider::new`]).
    #[serde(default)]
    pub oauth2_client: Option<String>,
}

const fn default_connect_timeout_ms() -> u64 {
    DEFAULT_CONNECT_TIMEOUT_MS
}

const fn default_request_timeout_ms() -> u64 {
    DEFAULT_REQUEST_TIMEOUT_MS
}

impl ExternalTerminologyConfig {
    /// Load configuration: defaults, then an optional TOML file (path in
    /// `EHRBASE_VALIDATION_CONFIG`), then
    /// `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_`-prefixed environment variables
    /// (nested keys use `__`).
    ///
    /// # Errors
    /// Returns a [`figment::Error`] if a value fails to parse.
    #[allow(clippy::result_large_err)] // figment::Error is large by design
    pub fn load() -> Result<Self, figment::Error> {
        let mut fig = Figment::from(Serialized::defaults(Self::default()));
        if let Ok(path) = std::env::var("EHRBASE_VALIDATION_CONFIG") {
            fig = fig.merge(Toml::file(path));
        }
        fig.merge(Env::prefixed("EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_").split("__"))
            .extract()
    }

    /// Build the named provider, or `None` when external terminology is disabled
    /// or no provider carries that name.
    ///
    /// # Errors
    /// [`SmError`] if the named provider's configuration is invalid (empty URL
    /// or an un-buildable HTTP client).
    #[must_use]
    pub fn provider(&self, name: &str) -> Option<Result<FhirTerminologyProvider, SmError>> {
        if !self.enabled {
            return None;
        }
        self.providers
            .get(name)
            .map(|cfg| FhirTerminologyProvider::new(name, cfg))
    }

    /// Build the default provider: the one named `default`, or the single
    /// configured provider when exactly one exists. `None` when external
    /// terminology is disabled or the selection is ambiguous/empty.
    ///
    /// # Errors
    /// [`SmError`] if the selected provider's configuration is invalid.
    #[must_use]
    pub fn default_provider(&self) -> Option<Result<FhirTerminologyProvider, SmError>> {
        if !self.enabled {
            return None;
        }
        let (name, cfg) = self
            .providers
            .get_key_value(DEFAULT_PROVIDER_NAME)
            .or_else(|| {
                // Exactly one configured provider → use it unambiguously.
                let mut it = self.providers.iter();
                match (it.next(), it.next()) {
                    (Some(only), None) => Some(only),
                    _ => None,
                }
            })?;
        Some(FhirTerminologyProvider::new(name, cfg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_by_default() {
        let c = ExternalTerminologyConfig::default();
        assert!(!c.enabled);
        assert!(!c.fail_on_error);
        assert!(c.providers.is_empty());
        assert!(c.default_provider().is_none());
    }

    #[test]
    #[allow(clippy::result_large_err)] // figment::Jail closure signature
    fn env_builds_a_provider() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_ENABLED", "true");
            jail.set_env(
                "EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_PROVIDERS__DEFAULT__TYPE",
                "fhir",
            );
            jail.set_env(
                "EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_PROVIDERS__DEFAULT__URL",
                "http://terminology:8090/fhir",
            );
            let c = ExternalTerminologyConfig::load().expect("load");
            assert!(c.enabled);
            let cfg = c.providers.get("default").expect("default provider");
            assert_eq!(cfg.kind, ProviderKind::Fhir);
            assert_eq!(cfg.url, "http://terminology:8090/fhir");
            assert_eq!(cfg.operation, FhirOperation::ValidateCode);
            assert_eq!(cfg.connect_timeout_ms, DEFAULT_CONNECT_TIMEOUT_MS);
            // Selection materialises the provider.
            assert!(c.default_provider().expect("selected").is_ok());
            Ok(())
        });
    }

    #[test]
    #[allow(clippy::result_large_err)] // figment::Jail closure signature
    fn disabled_config_selects_nothing_even_with_a_provider() {
        figment::Jail::expect_with(|jail| {
            jail.set_env(
                "EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_PROVIDERS__DEFAULT__URL",
                "http://terminology:8090/fhir",
            );
            let c = ExternalTerminologyConfig::load().expect("load");
            assert!(!c.enabled);
            assert!(c.provider("default").is_none());
            assert!(c.default_provider().is_none());
            Ok(())
        });
    }

    #[test]
    fn empty_url_is_rejected_at_build() {
        let mut providers = BTreeMap::new();
        providers.insert(
            "default".to_owned(),
            FhirProviderConfig {
                kind: ProviderKind::Fhir,
                url: "   ".to_owned(),
                operation: FhirOperation::ValidateCode,
                connect_timeout_ms: DEFAULT_CONNECT_TIMEOUT_MS,
                request_timeout_ms: DEFAULT_REQUEST_TIMEOUT_MS,
                oauth2_client: None,
            },
        );
        let c = ExternalTerminologyConfig {
            enabled: true,
            fail_on_error: false,
            providers,
        };
        assert!(c.provider("default").expect("present").is_err());
    }
}
