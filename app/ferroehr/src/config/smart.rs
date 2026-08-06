//! SMART App Launch configuration
//! (`docs/specs/openehr/ITS-REST/docs/smart_app_launch/master04-service_discovery.adoc`,
//! `.../master08-scopes.adoc`, `.../master09-experimental_features.adoc`).
//!
//! Off by default (`enabled = false`): a stock server serves no
//! `/.well-known/smart-configuration` document and runs no SMART scope gate, so
//! the wire is byte-identical to a non-SMART deployment — the same opt-in
//! extension-group convention the ADMIN/terminology/FHIR groups follow.
//!
//! This is the `[smart]` section of the one server configuration tree; it carries **no loader of its own** — the
//! whole tree is assembled once by `ferroehr::config` and this struct is
//! deserialized as a field of it. The discovery router is mounted from it in
//! `ferroehr_rest::router` and the scope gate reads it in
//! `ferroehr_rest::extensions::access::pep`.

use serde::{Deserialize, Serialize};

/// SMART App Launch resource-server configuration.
///
/// The CDR is the SMART Platform's `org.openehr.rest` resource server (master02
/// §Glossary): it advertises the external Authorization-Server endpoints, parses
/// and enforces the master08 resource-scope grammar, and binds the master07/09
/// launch context to the patient compartment. It never issues tokens, registers
/// clients, or runs the `OAuth2` endpoints (those are Authorization-Server duties).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SmartConfig {
    /// Master switch. When `false`, the discovery document is not served (404)
    /// and the scope gate is inert — a stock server is unchanged.
    #[serde(default)]
    pub enabled: bool,

    /// The SMART *Platform* base URL the discovery document hangs off (master04
    /// §Service Discovery: the doc is served relative to the Platform/gateway
    /// base, not the FHIR base). When unset the discovery router defaults it to
    /// the REST root (`/ferroehr/rest`). A leading path segment is honoured, e.g.
    /// `/gateway/v1` → `/gateway/v1/.well-known/smart-configuration`.
    #[serde(default)]
    pub platform_base_url: Option<String>,

    /// The server's externally-reachable origin (scheme + host [+ port]), e.g.
    /// `https://cdr.example.com`. master04 §Services makes every `services.*`
    /// entry's `baseUrl` an **"Absolute URL to the root of the API (required)"**,
    /// so the discovery document prefixes this origin onto the served base
    /// paths. REQUIRED when SMART is enabled ([`Self::validate`]).
    #[serde(default)]
    pub public_base_url: Option<String>,

    /// The token claim carrying the resolved openEHR `EHR` id for the launch
    /// context (master07 §Context Selection token-response table: `ehrId`).
    /// Default `ehrId`.
    pub ehr_id_claim: String,

    /// The fallback launch-context claim when [`Self::ehr_id_claim`] is absent
    /// (the standard SMART `patient` context attribute, master07). Default
    /// `patient`.
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
    /// are "currently implementation-defined").
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
#[serde(default, deny_unknown_fields)]
pub struct EpisodeConfig {
    /// Advertise + accept episode context. Advisory only (no filtering).
    #[serde(default)]
    pub enabled: bool,
}

/// The Authorization-Server endpoints + `OAuth2` metadata advertised in the
/// `/.well-known/smart-configuration` document (master04 §Authentication
/// Endpoints).
///
/// Every field is operator-supplied; an unset optional endpoint is simply
/// omitted from the document.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
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
    /// `capabilities` the operator additionally advertises (the HL7-defined base
    /// capabilities — `launch-ehr`, `sso-openid-connect`, `client-public`, … —
    /// live in the external SMART App Launch framework; master04 §Capabilities:
    /// the openEHR list is "In addition to those scopes defined in the original
    /// SMART App Launch framework"). Appended to the openEHR capabilities the
    /// CDR derives itself; duplicates are dropped.
    #[serde(default)]
    pub capabilities: Vec<String>,
}

impl Default for SmartConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            platform_base_url: None,
            public_base_url: None,
            ehr_id_claim: "ehrId".to_owned(),
            patient_claim: "patient".to_owned(),
            require_smart_scopes: false,
            episode: EpisodeConfig::default(),
            launch_base64_json: false,
            endpoints: SmartEndpoints::default(),
        }
    }
}

impl SmartConfig {
    /// Boot validation.
    ///
    /// - master06 §Deprecated Flows: the CDR must never advertise the Implicit
    ///   or Resource-Owner-Password grants.
    /// - When SMART is enabled: `public_base_url` is required (master04
    ///   §Services — `baseUrl` is an "Absolute URL … (required)", buildable
    ///   only from a known external origin), and the three core
    ///   Authorization-Server endpoints must be present — `issuer`,
    ///   `authorization_endpoint`, `token_endpoint` (master04 §Authentication
    ///   Endpoints delegates their requiredness to OIDC Discovery / the HL7
    ///   SMART metadata, both of which require them; an enabled Platform
    ///   without them serves an unusable document).
    ///
    /// # Errors
    /// A message naming the offending grant type or the missing required field.
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
        if self.enabled {
            let origin = self.public_base_url.as_deref().unwrap_or("");
            if !(origin.starts_with("http://") || origin.starts_with("https://")) {
                return Err("smart.public_base_url (an absolute http(s) origin, e.g. \
                     'https://cdr.example.com') is required when SMART is \
                     enabled — master04 §Services makes every services.*.baseUrl \
                     an absolute URL"
                    .to_owned());
            }
            if self.endpoints.authorization_endpoint.is_none()
                || self.endpoints.token_endpoint.is_none()
            {
                return Err(
                    "smart.endpoints.authorization_endpoint and .token_endpoint \
                     are required when SMART is enabled (master04 \
                     §Authentication Endpoints via OIDC Discovery / HL7 SMART \
                     metadata requiredness)"
                        .to_owned(),
                );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_off_and_sane() {
        let c = SmartConfig::default();
        assert!(!c.enabled);
        assert!(c.platform_base_url.is_none());
        assert!(c.public_base_url.is_none());
        assert!(c.endpoints.capabilities.is_empty());
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
    fn enabled_requires_origin_and_core_endpoints() {
        let mut c = SmartConfig {
            enabled: true,
            ..SmartConfig::default()
        };
        // Missing everything → the origin requirement fires first.
        assert!(c.validate().is_err());
        c.public_base_url = Some("https://cdr.example.com".to_owned());
        // Origin present but no AS endpoints → still an error.
        assert!(c.validate().is_err());
        c.endpoints.authorization_endpoint = Some("https://as.example/authorize".to_owned());
        c.endpoints.token_endpoint = Some("https://as.example/token".to_owned());
        assert!(c.validate().is_ok());
        // A relative origin is not an absolute URL.
        c.public_base_url = Some("/ferroehr".to_owned());
        assert!(c.validate().is_err());
    }
}
