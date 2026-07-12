//! Subject-proxy FHIR-frame executor configuration + client
//! (`docs/design/sm-platform/10-subject-proxy.md` §2.2, G-4).
//!
//! The Subject Proxy Service exists so a caller "need not know about the
//! particular standard, representational model, query language or API of the
//! data source … there is no need to even assume that an openEHR back-end
//! system is the source"
//! (`SM/docs/openehr_platform/master10-subject_proxy_service.adoc` §Overview);
//! an `API_CALL`/`fhir_get` `DATA_FRAME` (`data_frame.adoc`) therefore retrieves
//! from a remote HL7 FHIR system and yields an `HL7_FHIR_SAMPLE`
//! (`hl7_fhir_sample.adoc`).
//!
//! No openEHR spec governs the transport specifics — our own design, mirroring
//! the external-terminology provider (`crate::service::terminology::config`) and
//! `docs/design/terminology-server-integration.md`. Configuration is
//! **opt-in and fail-closed**: only systems named here are reachable; a frame
//! whose `system_id` matches no configured system is a typed rejection, never
//! an arbitrary outbound request.
//!
//! Loaded from defaults ← optional TOML file (`EHRBASE_SUBJECT_PROXY_CONFIG`) ←
//! `EHRBASE_SUBJECT_PROXY_`-prefixed environment (nested keys use `__`, e.g.
//! `EHRBASE_SUBJECT_PROXY__SYSTEMS__PAS__BASE_URL`).

use std::collections::BTreeMap;
use std::time::Duration;

use figment::Figment;
use figment::providers::{Env, Format, Serialized, Toml};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use ehrbase_sm::SmError;

/// Default per-system connect timeout (ms).
const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 2_000;
/// Default per-system request timeout (ms).
const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 10_000;

/// Subject-proxy FHIR-frame configuration (`[subject_proxy]`): the named FHIR
/// systems an `API_CALL`/`fhir_get` frame may retrieve from.
///
/// Empty (the default) = no FHIR system is reachable; every FHIR frame is a
/// typed rejection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubjectProxyConfig {
    /// The reachable FHIR systems, keyed by the frame's `SYSTEM_CALL.system_id`.
    #[serde(default)]
    pub systems: BTreeMap<String, SpFhirSystem>,
}

/// One configured FHIR system (`data_frame.adoc` `SYSTEM_CALL.system_id`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpFhirSystem {
    /// FHIR R4 base URL, e.g. `https://fhir.example.org/r4` — the frame's
    /// `query_text` is resolved relative to this after `$subject_id`
    /// substitution.
    pub base_url: String,
    /// TCP connect timeout (ms).
    #[serde(default = "default_connect_timeout_ms")]
    pub connect_timeout_ms: u64,
    /// Overall request timeout (ms).
    #[serde(default = "default_request_timeout_ms")]
    pub request_timeout_ms: u64,
}

const fn default_connect_timeout_ms() -> u64 {
    DEFAULT_CONNECT_TIMEOUT_MS
}

const fn default_request_timeout_ms() -> u64 {
    DEFAULT_REQUEST_TIMEOUT_MS
}

impl SubjectProxyConfig {
    /// Load configuration: defaults, then an optional TOML file (path in
    /// `EHRBASE_SUBJECT_PROXY_CONFIG`), then `EHRBASE_SUBJECT_PROXY_`-prefixed
    /// environment variables (nested keys use `__`).
    ///
    /// # Errors
    /// Returns a [`figment::Error`] if a value fails to parse.
    #[allow(clippy::result_large_err)] // figment::Error is large by design
    pub fn load() -> Result<Self, figment::Error> {
        let mut fig = Figment::from(Serialized::defaults(Self::default()));
        if let Ok(path) = std::env::var("EHRBASE_SUBJECT_PROXY_CONFIG") {
            fig = fig.merge(Toml::file(path));
        }
        fig.merge(Env::prefixed("EHRBASE_SUBJECT_PROXY_").split("__"))
            .extract()
    }

    /// Build the FHIR executor, or `None` when no systems are configured.
    ///
    /// # Errors
    /// [`SmError`] if a system's URL is empty or its HTTP client cannot be built.
    pub fn build(&self) -> Result<Option<SubjectProxyFhir>, SmError> {
        if self.systems.is_empty() {
            return Ok(None);
        }
        let mut clients = BTreeMap::new();
        for (name, sys) in &self.systems {
            clients.insert(name.clone(), FhirSystemClient::new(name, sys)?);
        }
        Ok(Some(SubjectProxyFhir { clients }))
    }
}

/// The built FHIR-frame executor: one `reqwest` client per configured system.
#[derive(Debug, Clone)]
pub struct SubjectProxyFhir {
    clients: BTreeMap<String, FhirSystemClient>,
}

/// A configured FHIR system's base URL + HTTP client.
#[derive(Debug, Clone)]
struct FhirSystemClient {
    /// Base URL, trailing `/` stripped.
    base: String,
    client: reqwest::Client,
}

impl FhirSystemClient {
    fn new(name: &str, sys: &SpFhirSystem) -> Result<Self, SmError> {
        let base = sys.base_url.trim().trim_end_matches('/').to_owned();
        if base.is_empty() {
            return Err(SmError::exception(format!(
                "subject-proxy FHIR system '{name}' has an empty base_url"
            )));
        }
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(sys.connect_timeout_ms))
            .timeout(Duration::from_millis(sys.request_timeout_ms))
            .build()
            .map_err(|e| {
                SmError::exception(format!(
                    "building subject-proxy FHIR client for system '{name}': {e}"
                ))
            })?;
        Ok(Self { base, client })
    }
}

/// A successful FHIR retrieve: the resource JSON plus the `meta.lastUpdated`
/// effective time when present (`SAMPLE.effective_time`, master10 §Samples).
#[derive(Debug, Clone)]
pub struct FhirFetch {
    /// The retrieved FHIR resource (canonical JSON).
    pub resource: Value,
    /// `resource.meta.lastUpdated`, the real-world time the data pertains to.
    pub effective_time: Option<String>,
}

impl SubjectProxyFhir {
    /// Whether a system with this `system_id` is configured (the fail-closed
    /// gate — an unconfigured system is never reached).
    #[must_use]
    pub fn has_system(&self, system_id: &str) -> bool {
        self.clients.contains_key(system_id)
    }

    /// GET `base_url/query_path` from the named system with
    /// `Accept: application/fhir+json`. `query_path` is the frame's `query_text`
    /// with `$subject_id` already substituted.
    ///
    /// `Ok` on a `200` FHIR body; `Err(reason)` on an unconfigured system, a
    /// non-2xx status, a timeout, or a malformed body — the caller turns an
    /// `Err` into an unavailable `SAMPLE` so the primary→fallback pipeline runs
    /// (`data_frame.adoc`).
    pub async fn get(&self, system_id: &str, query_path: &str) -> Result<FhirFetch, String> {
        let Some(sys) = self.clients.get(system_id) else {
            return Err(format!("FHIR system {system_id:?} is not configured"));
        };
        let url = format!("{}/{}", sys.base, query_path.trim_start_matches('/'));
        let response = sys
            .client
            .get(&url)
            .header(reqwest::header::ACCEPT, "application/fhir+json")
            .send()
            .await
            .map_err(|e| {
                let kind = if e.is_timeout() {
                    "timeout"
                } else if e.is_connect() {
                    "connect error"
                } else {
                    "transport error"
                };
                format!("FHIR system {system_id:?} GET {url}: {kind}: {e}")
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!(
                "FHIR system {system_id:?} GET {url} returned HTTP {}",
                status.as_u16()
            ));
        }
        let resource: Value = response.json().await.map_err(|e| {
            format!("FHIR system {system_id:?} GET {url}: malformed FHIR response: {e}")
        })?;
        let effective_time = resource
            .pointer("/meta/lastUpdated")
            .and_then(Value::as_str)
            .map(str::to_owned);
        Ok(FhirFetch {
            resource,
            effective_time,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_builds_no_executor() {
        assert!(
            SubjectProxyConfig::default()
                .build()
                .expect("build")
                .is_none()
        );
    }

    #[test]
    fn systems_build_an_executor() {
        let mut systems = BTreeMap::new();
        systems.insert(
            "pas".to_owned(),
            SpFhirSystem {
                base_url: "http://fhir.example.org/r4/".to_owned(),
                connect_timeout_ms: 500,
                request_timeout_ms: 800,
            },
        );
        let fhir = SubjectProxyConfig { systems }
            .build()
            .expect("build")
            .expect("some");
        assert!(fhir.has_system("pas"));
        assert!(!fhir.has_system("other"));
    }

    #[test]
    fn empty_base_url_is_rejected() {
        let mut systems = BTreeMap::new();
        systems.insert(
            "pas".to_owned(),
            SpFhirSystem {
                base_url: "   ".to_owned(),
                connect_timeout_ms: 500,
                request_timeout_ms: 800,
            },
        );
        assert!(SubjectProxyConfig { systems }.build().is_err());
    }
}
