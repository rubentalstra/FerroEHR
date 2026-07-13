//! SMART App Launch configuration
//! (`docs/specs/openehr/ITS-REST/docs/smart_app_launch/master04-service_discovery.adoc`,
//! `.../master08-scopes.adoc`, `.../master09-experimental_features.adoc`).
//!
//! Off by default (`enabled = false`): a stock server serves no
//! `/.well-known/smart-configuration` document and runs no SMART scope gate, so
//! the wire is byte-identical to a non-SMART deployment — the same opt-in
//! extension-group convention the ADMIN/terminology/FHIR groups follow
//! (`crate::config` `AdminConfig`/`TerminologyConfig`).
//!
//! Environment binding (`EHRBASE_REST_SMART__*`, `__` = nesting): this struct
//! is the `#[serde(default)] pub smart: SmartConfig` field on
//! [`crate::config::RestConfig`] (alongside `terminology`/`fhir`), so the
//! existing `Env::prefixed("EHRBASE_REST_").split("__")` figment chain in
//! `RestConfig::load` picks it up with no extra code. The discovery router is
//! mounted from it in [`crate::router`] and the scope gate reads it in
//! [`crate::extensions::access::pep`]. [`SmartConfig::load`] is the equivalent
//! standalone loader for tests and for the discovery router.

use figment::Figment;
use figment::providers::{Env, Serialized};
use serde::{Deserialize, Serialize};

/// SMART App Launch resource-server configuration.
///
/// The CDR is the SMART Platform's `org.openehr.rest` resource server (master02
/// §Glossary): it advertises the external Authorization-Server endpoints, parses
/// and enforces the master08 resource-scope grammar, and binds the master07/09
/// launch context to the patient compartment. It never issues tokens, registers
/// clients, or runs the `OAuth2` endpoints (those are Authorization-Server duties
/// — recorded as PORT NOTEs in `docs/design/its-rest/smart.md` §6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartConfig {
    /// Master switch. When `false`, the discovery document is not served (404)
    /// and the scope gate is inert — a stock server is unchanged.
    #[serde(default)]
    pub enabled: bool,

    /// The SMART *Platform* base URL the discovery document hangs off (master04
    /// §Service Discovery: the doc is served relative to the Platform/gateway
    /// base, not the FHIR base). When unset the discovery router defaults it to
    /// the REST root (`/ehrbase/rest`). A leading path segment is honoured, e.g.
    /// `/gateway/v1` → `/gateway/v1/.well-known/smart-configuration`.
    #[serde(default)]
    pub platform_base_url: Option<String>,

    /// The token claim carrying the resolved openEHR `EHR` id for the launch
    /// context (master07 §Context Selection token-response table: `ehrId`).
    /// Default `ehrId`.
    #[serde(default = "default_ehr_id_claim")]
    pub ehr_id_claim: String,

    /// The fallback launch-context claim when [`Self::ehr_id_claim`] is absent
    /// (the standard SMART `patient` context attribute, master07). Default
    /// `patient`.
    #[serde(default = "default_patient_claim")]
    pub patient_claim: String,

    /// Fail-closed switch (master08 §Scopes ¶2: the Platform must validate
    /// requested scopes against access-control policy). When `true`, a Bearer
    /// token that carries **no** matching SMART resource scope for a
    /// scope-governed operation is denied. When `false` (default) SMART is
    /// advisory: the gate enforces only when the token actually carries SMART
    /// resource scopes for that resource family, so a non-SMART token is
    /// unaffected.
    #[serde(default)]
    pub require_smart_scopes: bool,

    /// Episode context (master09 §Experimental: Episode Context) — experimental,
    /// advertised only. `episode.enabled = true` advertises
    /// `context-openehr-episode` and accepts the `launch/episode` scope +
    /// `episodeId` claim, but applies **no** episode-scoped filtering (openEHR
    /// has no first-class Episode resource yet; master09 states the semantics
    /// are "currently implementation-defined"). PORT NOTE, `smart.md` §6.
    #[serde(default)]
    pub episode: EpisodeConfig,

    /// Advertise `launch-base64-json` (master09 §Launch Parameter as a Token) —
    /// experimental. The base64-JSON `launch` object is consumed by the
    /// *Application*, not the CDR; we only advertise the capability. Default off.
    #[serde(default)]
    pub launch_base64_json: bool,

    /// The external Authorization-Server / OIDC endpoints advertised verbatim in
    /// the discovery document (master04 §Authentication Endpoints). The CDR does
    /// not implement any of these; it copies the operator's values.
    #[serde(default)]
    pub endpoints: SmartEndpoints,
}

/// Episode-context sub-config (master09 §Experimental: Episode Context).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EpisodeConfig {
    /// Advertise + accept episode context. Advisory only (no filtering).
    #[serde(default)]
    pub enabled: bool,
}

/// The Authorization-Server endpoints + `OAuth2` metadata advertised in the
/// `/.well-known/smart-configuration` document (master04 §Authentication
/// Endpoints). Every field is operator-supplied; an unset optional endpoint is
/// simply omitted from the document.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SmartEndpoints {
    /// The token/OIDC `issuer`. When unset the discovery router falls back to
    /// the configured OIDC bearer issuer (`auth.oidc.issuer`).
    #[serde(default)]
    pub issuer: Option<String>,
    /// The JWKS document URL (`jwks_uri`).
    #[serde(default)]
    pub jwks_uri: Option<String>,
    /// The `OAuth2` `authorization_endpoint`.
    #[serde(default)]
    pub authorization_endpoint: Option<String>,
    /// The `OAuth2` `token_endpoint`.
    #[serde(default)]
    pub token_endpoint: Option<String>,
    /// The dynamic-client `registration_endpoint` (master03 recommends
    /// out-of-band registration; advertised only when the AS offers it).
    #[serde(default)]
    pub registration_endpoint: Option<String>,
    /// The token `introspection_endpoint`.
    #[serde(default)]
    pub introspection_endpoint: Option<String>,
    /// The token `revocation_endpoint`.
    #[serde(default)]
    pub revocation_endpoint: Option<String>,
    /// The user `management_endpoint`.
    #[serde(default)]
    pub management_endpoint: Option<String>,
    /// `token_endpoint_auth_methods_supported` (e.g. `client_secret_basic`,
    /// `private_key_jwt`).
    #[serde(default)]
    pub token_endpoint_auth_methods_supported: Vec<String>,
    /// `grant_types_supported`. master06 §Deprecated Flows: `implicit` and the
    /// resource-owner-password grant MUST NOT appear — [`SmartConfig::validate`]
    /// rejects them.
    #[serde(default)]
    pub grant_types_supported: Vec<String>,
    /// `response_types_supported` (e.g. `code`).
    #[serde(default)]
    pub response_types_supported: Vec<String>,
    /// `code_challenge_methods_supported` (e.g. `S256`).
    #[serde(default)]
    pub code_challenge_methods_supported: Vec<String>,
    /// `scopes_supported`. When empty the discovery router advertises a default
    /// list reflecting the scopes the CDR actually enforces.
    #[serde(default)]
    pub scopes_supported: Vec<String>,
}

impl Default for SmartConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            platform_base_url: None,
            ehr_id_claim: default_ehr_id_claim(),
            patient_claim: default_patient_claim(),
            require_smart_scopes: false,
            episode: EpisodeConfig::default(),
            launch_base64_json: false,
            endpoints: SmartEndpoints::default(),
        }
    }
}

impl SmartConfig {
    /// Standalone loader: defaults, then `EHRBASE_REST_SMART__` environment
    /// variables (`__` = nesting, e.g. `EHRBASE_REST_SMART__EPISODE__ENABLED`).
    /// Produces the same keys as adding this struct as a `smart` field on
    /// `RestConfig` (where the shared TOML file + `EHRBASE_REST_` env are already
    /// handled by `RestConfig::load` — the recommended integration path).
    ///
    /// # Errors
    /// A [`figment::Error`] if a value fails to parse.
    #[allow(clippy::result_large_err)] // figment::Error is large by design
    pub fn load() -> Result<Self, figment::Error> {
        Figment::from(Serialized::defaults(SmartConfig::default()))
            .merge(Env::prefixed("EHRBASE_REST_SMART__").split("__"))
            .extract()
    }

    /// Boot validation (master06 §Deprecated Flows): the CDR must never advertise
    /// the Implicit or Resource-Owner-Password grants. Returns the offending
    /// grant name.
    ///
    /// # Errors
    /// A message naming a deprecated grant type present in
    /// `endpoints.grant_types_supported`.
    pub fn validate(&self) -> Result<(), String> {
        for grant in &self.endpoints.grant_types_supported {
            let g = grant.to_ascii_lowercase();
            if g == "implicit" || g == "password" || g == "resource_owner_password" {
                return Err(format!(
                    "grant type '{grant}' is deprecated and MUST NOT be advertised \
                     (master06 §Deprecated Flows)"
                ));
            }
        }
        Ok(())
    }

    /// Emit a boot warning when SMART is enabled but no Bearer/OIDC mechanism is
    /// configured: SMART scopes ride only Bearer tokens (Basic carries none), so
    /// an enabled-but-bearerless deployment can serve discovery yet never
    /// enforce a scope. Non-fatal.
    pub fn warn_if_bearerless(&self, oidc_configured: bool) {
        if self.enabled && !oidc_configured {
            tracing::warn!(
                "SMART is enabled but no OIDC bearer validation is configured; \
                 SMART scopes only ride Bearer tokens, so the scope gate will \
                 never engage (master06 §Authentication)"
            );
        }
    }
}

fn default_ehr_id_claim() -> String {
    "ehrId".to_owned()
}

fn default_patient_claim() -> String {
    "patient".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_off_and_sane() {
        let c = SmartConfig::default();
        assert!(!c.enabled);
        assert!(c.platform_base_url.is_none());
        assert_eq!(c.ehr_id_claim, "ehrId");
        assert_eq!(c.patient_claim, "patient");
        assert!(!c.require_smart_scopes);
        assert!(!c.episode.enabled);
        assert!(!c.launch_base64_json);
        assert!(c.endpoints.issuer.is_none());
    }

    #[test]
    fn validate_rejects_deprecated_grants() {
        let mut c = SmartConfig::default();
        c.endpoints.grant_types_supported = vec!["authorization_code".to_owned()];
        assert!(c.validate().is_ok());

        c.endpoints.grant_types_supported = vec!["implicit".to_owned()];
        assert!(c.validate().is_err());

        c.endpoints.grant_types_supported = vec!["PASSWORD".to_owned()];
        assert!(c.validate().is_err());
    }

    #[test]
    #[allow(clippy::result_large_err)] // figment::Jail closure signature
    fn env_binding_matches_nested_convention() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("EHRBASE_REST_SMART__ENABLED", "true");
            jail.set_env("EHRBASE_REST_SMART__EHR_ID_CLAIM", "openehr_ehr_id");
            jail.set_env("EHRBASE_REST_SMART__REQUIRE_SMART_SCOPES", "true");
            jail.set_env("EHRBASE_REST_SMART__EPISODE__ENABLED", "true");
            jail.set_env("EHRBASE_REST_SMART__LAUNCH_BASE64_JSON", "true");
            let c = SmartConfig::load().expect("load");
            assert!(c.enabled);
            assert_eq!(c.ehr_id_claim, "openehr_ehr_id");
            assert!(c.require_smart_scopes);
            assert!(c.episode.enabled);
            assert!(c.launch_base64_json);
            Ok(())
        });
    }

    /// The nested form: proves that once `SmartConfig` is a `smart` field on
    /// `RestConfig`, the existing `EHRBASE_REST_` figment chain binds it with no
    /// extra code.
    #[test]
    #[allow(clippy::result_large_err)] // figment::Jail closure signature
    fn nested_under_rest_prefix() {
        #[derive(Debug, Default, Serialize, Deserialize)]
        struct Wrapper {
            #[serde(default)]
            smart: SmartConfig,
        }
        figment::Jail::expect_with(|jail| {
            jail.set_env("EHRBASE_REST_SMART__ENABLED", "true");
            jail.set_env(
                "EHRBASE_REST_SMART__ENDPOINTS__AUTHORIZATION_ENDPOINT",
                "https://as.example/auth",
            );
            let w: Wrapper = Figment::from(Serialized::defaults(Wrapper::default()))
                .merge(Env::prefixed("EHRBASE_REST_").split("__"))
                .extract()
                .expect("extract");
            assert!(w.smart.enabled);
            assert_eq!(
                w.smart.endpoints.authorization_endpoint.as_deref(),
                Some("https://as.example/auth")
            );
            Ok(())
        });
    }
}
