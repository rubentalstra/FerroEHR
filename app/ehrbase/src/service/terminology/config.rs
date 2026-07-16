//! External-terminology configuration (`figment`), matching the shape in
//! `docs/terminology-validation.md` §4.
//!
//! No openEHR spec governs the transport/config mechanics — our own design,
//! grounded on `docs/terminology-validation.md` (the client) +
//! `docs/design/terminology-server-integration.md` (the self-hostable FHIR R4
//! TS it points at). `BASE/docs/architecture_overview/master12-terminology.adoc`
//! models the concrete backend as an external "terminology query server", which
//! is why this config lives beside the interface realization in
//! `service/terminology/`.
//!
//! This is the `[terminology]` section of the one config tree
//! ([`crate::config::EhrbaseConfig`], `docs/design/configuration.md` §3.15); no
//! loader of its own. [`TerminologyConfig`] groups the extension-API toggle
//! (`api_enabled`) with the external-server validation config
//! ([`ExternalTerminologyConfig`], under `[terminology.external]`).
//!
//! Provider selection is **openEHR-bundle-by-default, FHIR opt-in**: with
//! [`ExternalTerminologyConfig::enabled`] `false` (the default) no remote
//! provider is built and composition validation stays on the in-process
//! `openehr-term` bundle ([`super::bundle`]); a FHIR provider is materialised
//! only when a deployment opts in.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::service::status::SmError;

use super::fhir::FhirTerminologyProvider;

/// Default per-provider connect timeout (ms).
const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 2_000;
/// Default per-provider request timeout (ms).
const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 10_000;
/// The provider name selected by [`ExternalTerminologyConfig::default_provider`]
/// when several are configured.
const DEFAULT_PROVIDER_NAME: &str = "default";

/// The `[terminology]` section: the extension-API toggle + external-server
/// validation config.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TerminologyConfig {
    /// Mount the terminology extension API (SM `I_TERMINOLOGY_SERVICE`). Off by
    /// default — the routes answer `404` unless enabled.
    pub api_enabled: bool,
    /// External terminology-server validation (`[terminology.external]`).
    pub external: ExternalTerminologyConfig,
}

/// External-terminology validation configuration (`[terminology.external]`).
///
/// Defaults are the off state: `enabled = false`, `fail_on_error = false`, no
/// providers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExternalTerminologyConfig {
    /// Whether external-terminology validation is active. When `false` (the
    /// default), no remote provider is built.
    #[serde(default)]
    pub enabled: bool,
    /// On a terminology-server/connectivity error: `true` = reject the
    /// composition (fail-closed); `false` = accept it (fail-open, the default).
    /// Consumed by the composition-validation walker — the raw provider always
    /// surfaces the error; this flag decides how the caller treats it.
    #[serde(default)]
    pub fail_on_error: bool,
    /// The configured terminology-server providers, keyed by name.
    #[serde(default)]
    pub providers: BTreeMap<String, FhirProviderConfig>,
}

/// The kind of terminology server. Only FHIR R4 is supported
/// (`docs/terminology-validation.md` §4).
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
#[serde(deny_unknown_fields)]
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
    /// core `$validate-code`/`$expand`/`$subsumes`/`$lookup` provider; the field
    /// is accepted so config written for the full design parses, but no bearer
    /// token is attached yet. A configured value that would silently send
    /// unauthenticated requests is surfaced at build time
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
    fn a_configured_provider_materialises() {
        let mut providers = BTreeMap::new();
        providers.insert(
            "default".to_owned(),
            FhirProviderConfig {
                kind: ProviderKind::Fhir,
                url: "http://terminology:8090/fhir".to_owned(),
                operation: FhirOperation::ValidateCode,
                connect_timeout_ms: DEFAULT_CONNECT_TIMEOUT_MS,
                request_timeout_ms: DEFAULT_REQUEST_TIMEOUT_MS,
                oauth2_client: None,
            },
        );
        let enabled = ExternalTerminologyConfig {
            enabled: true,
            fail_on_error: false,
            providers: providers.clone(),
        };
        assert!(enabled.default_provider().expect("selected").is_ok());
        // Disabled selects nothing even with a provider present.
        let disabled = ExternalTerminologyConfig {
            enabled: false,
            fail_on_error: false,
            providers,
        };
        assert!(disabled.provider("default").is_none());
        assert!(disabled.default_provider().is_none());
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
