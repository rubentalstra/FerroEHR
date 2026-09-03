// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Subject-proxy FHIR-frame executor configuration + client.
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
//! the external-terminology provider (`crate::service::terminology`).
//! Configuration is **opt-in and fail-closed**: only systems named here are
//! reachable; a frame whose `system_id` matches no configured system is a typed
//! rejection, never an arbitrary outbound request.
//!
//! This is the `[subject_proxy]` section of the one config tree
//! ([`crate::config::FerroEhrConfig`]); no loader of its own. The env form
//! `FERROEHR__SUBJECT_PROXY__SYSTEMS__PAS__BASE_URL` binds through the tree's
//! mechanical mapping.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 6): FHIR resources are an external standard \
              with no RM type (typed-FHIR evaluation tracked separately)"
)]

use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::service::status::SmError;

/// Subject-proxy FHIR-frame configuration (`[subject_proxy]`): the named FHIR
/// systems an `API_CALL`/`fhir_get` frame may retrieve from.
///
/// Empty (the default) = no FHIR system is reachable; every FHIR frame is a
/// typed rejection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SubjectProxyConfig {
    /// The reachable FHIR systems, keyed by the frame's `SYSTEM_CALL.system_id`.
    pub systems: BTreeMap<String, SpFhirSystem>,
}

/// One configured FHIR system (`data_frame.adoc` `SYSTEM_CALL.system_id`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpFhirSystem {
    /// The remote FHIR server's base URL, e.g. `https://fhir.example.org/r4`
    /// — the frame's `query_text` is resolved relative to this after
    /// `$subject_id` substitution; empty is a boot error
    /// ([`SubjectProxyConfig::build`]). The FHIR release is the REMOTE's
    /// property: the proxy relays `application/fhir+json` bodies untyped and
    /// decodes nothing release-specific, so no release is claimed here (the
    /// terminology integration's posture).
    pub base_url: String,
    /// TCP connect timeout (ms).
    pub connect_timeout_ms: u64,
    /// Overall request timeout (ms).
    pub request_timeout_ms: u64,
}

impl Default for SpFhirSystem {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            connect_timeout_ms: 2_000,
            request_timeout_ms: 10_000,
        }
    }
}

impl SubjectProxyConfig {
    /// Build the FHIR executor, or `None` when no systems are configured.
    ///
    /// # Errors
    /// [`SmError`] (`exception`) — a system's `base_url` is empty/blank, or its
    /// HTTP client cannot be built.
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
                    "building the subject-proxy FHIR client for system '{name}' failed"
                ))
                .with_source(e)
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
    /// `Ok` on a 2xx FHIR body; the caller turns an `Err` into an unavailable
    /// `SAMPLE` so the primary→fallback pipeline runs (`data_frame.adoc`).
    ///
    /// # Errors
    /// A reason string on: an unconfigured `system_id`, a transport failure
    /// (timeout / connect / other), a non-2xx HTTP status, or a body that is
    /// not valid JSON.
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

    /// A blank or absent `base_url` fails the boot build, naming the key: the
    /// section reads its defaults from one `Default` impl, so serde no longer
    /// reports the missing key and this is the only gate.
    #[test]
    fn empty_base_url_is_rejected() {
        for system in [
            SpFhirSystem {
                base_url: "   ".to_owned(),
                connect_timeout_ms: 500,
                request_timeout_ms: 800,
            },
            SpFhirSystem::default(),
        ] {
            let systems = BTreeMap::from([("pas".to_owned(), system)]);
            let err = SubjectProxyConfig { systems }
                .build()
                .expect_err("a system with no base_url must be rejected");
            assert!(
                err.message.contains("base_url") && err.message.contains("pas"),
                "got {}",
                err.message
            );
        }
    }

    /// The section's defaults are the literals its `Default` impl declares.
    #[test]
    fn system_defaults_are_the_documented_timeouts() {
        let sys = SpFhirSystem::default();
        assert_eq!(sys.connect_timeout_ms, 2_000);
        assert_eq!(sys.request_timeout_ms, 10_000);
    }
}
