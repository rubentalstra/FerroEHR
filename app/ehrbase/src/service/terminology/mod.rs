//! The **Terminology** component of the platform crate: the concrete
//! realization of the SM `I_TERMINOLOGY_SERVICE` interface
//! ([`crate::service::TerminologyService`]) on [`EhrbaseService`], plus the AQL
//! terminology seam.
//!
//! Spec: `docs/specs/openehr/SM/docs/openehr_platform/
//! master12-terminology_service.adoc` + `UML/classes/i_terminology_service.adoc`
//! (the 9 calls + preconditions) and the extract model
//! (`terminology_extract.adoc` &c.). Context:
//! `BASE/docs/architecture_overview/master12-terminology.adoc` models the
//! concrete backend as an external "terminology query server"; the SM defines a
//! **single** interface. The interface/provider split is therefore *logical*,
//! realized here by one trait impl selecting among providers:
//!
//! - [`bundle`] — the in-process `openehr-term` bundle (TERM 3.1.0): the
//!   enumerable local default.
//! - [`fhir`] — [`FhirTerminologyProvider`], a remote FHIR R4 TS client (opt-in
//!   via [`ExternalTerminologyConfig`], [`config`]).
//!
//! # Provider routing (G-4)
//!
//! - **Enumeration** (`get_terminology_ids`, `has_terminology`,
//!   `get_terminology_description`) is answered **only by the bundle** — a FHIR
//!   TS is a validation/expansion backend, not an enumerable openEHR terminology
//!   (`fhir.rs` PORT NOTE). A FHIR-only deployment still answers these.
//! - **Lookup / validation** (`has_term`, `get_term`, `subsumes`,
//!   `value_set_validate`, `has_value_set`, `get_value_set`) is answered by the
//!   bundle when it knows the terminology, else routed to the configured FHIR
//!   provider, else falls through to the bundle's `Pre_has_terminology` →
//!   `NotFound`. With no FHIR provider configured (the default) this is
//!   byte-identical to a bundle-only service.

mod bundle;
mod config;
mod fhir;

pub mod types;

pub use config::{
    ExternalTerminologyConfig, FhirOperation, FhirProviderConfig, ProviderKind, TerminologyConfig,
};
pub use fhir::FhirTerminologyProvider;

use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::service::status::CallStatusType;
use crate::service::status::SmError;
use crate::service::terminology::types::{TerminologyDescription, TerminologyExtract};

use crate::aql::TerminologyExpander;
use crate::aql::error::{AqlError, AqlFeatureError, ExecError};
use crate::service::EhrbaseService;
use crate::versioning::OPENEHR;

/// The `service_api` identifier for the in-process openEHR terminology bundle.
///
/// PORT NOTE (B4): master03 §TERMINOLOGY's `service_api` examples are all
/// *external* servers (FHIR, Ocean, Better, Apelon) and the spec defines it as
/// "an identifier for the kind/flavour of terminology service" with an
/// implementation-defined value set — there is no standard identifier for a
/// local in-process bundle. We adopt the openEHR terminology's own id
/// `"openehr"`, and route the value set (`params_uri`) as a group / code-set id
/// within that terminology.
const BUNDLE_SERVICE_API: &str = "openehr";

/// Whether a `service_api` names a FHIR terminology service (any FHIR version):
/// the master03 examples are `hl7.org/fhir/4.0`, `/3.0`, `/1.0`, `/r4`.
fn is_fhir_service_api(service_api: &str) -> bool {
    service_api.to_ascii_lowercase().starts_with("hl7.org/fhir")
}

// ─── SM `I_TERMINOLOGY_SERVICE` on the service (provider routing) ─────────────

impl EhrbaseService {
    pub fn get_terminology_ids(&self) -> Result<Vec<String>, SmError> {
        // Enumeration is the bundle's (G-4).
        Ok(bundle::terminology_ids())
    }

    pub fn has_terminology(&self, terminology_id: &str) -> Result<bool, SmError> {
        // Enumeration is the bundle's (G-4).
        Ok(bundle::has_terminology(terminology_id))
    }

    pub fn get_terminology_description(
        &self,
        terminology_id: &str,
    ) -> Result<TerminologyDescription, SmError> {
        // Enumeration is the bundle's (G-4).
        bundle::terminology_description(terminology_id)
    }

    pub async fn has_term(
        &self,
        terminology_id: &str,
        code: &str,
        at_date: Option<String>,
    ) -> Result<bool, SmError> {
        if bundle::has_terminology(terminology_id) {
            // `at_date` no-op on the single-version bundle (G-1 bundle PORT NOTE).
            bundle::has_term(terminology_id, code)
        } else if let Some(p) = &self.external_terminology {
            p.has_term(terminology_id, code, at_date).await
        } else {
            bundle::has_term(terminology_id, code)
        }
    }

    pub async fn get_term(
        &self,
        terminology_id: &str,
        code: &str,
        attributes: Option<BTreeMap<String, String>>,
        at_date: Option<String>,
    ) -> Result<TerminologyExtract, SmError> {
        if bundle::has_terminology(terminology_id) {
            // No meta-model attributes exist for the openEHR bundle (G-3 bundle
            // PORT NOTE); `at_date` is a no-op on the pinned version (G-1).
            bundle::get_term(terminology_id, code)
        } else if let Some(p) = &self.external_terminology {
            p.get_term(terminology_id, code, attributes, at_date).await
        } else {
            bundle::get_term(terminology_id, code)
        }
    }

    pub async fn subsumes(
        &self,
        terminology_id: &str,
        ref_code: &str,
        candidate_child_code: &str,
    ) -> Result<bool, SmError> {
        if bundle::has_terminology(terminology_id) {
            bundle::subsumes(terminology_id, ref_code, candidate_child_code)
        } else if let Some(p) = &self.external_terminology {
            p.subsumes(terminology_id, ref_code, candidate_child_code)
                .await
        } else {
            bundle::subsumes(terminology_id, ref_code, candidate_child_code)
        }
    }

    pub async fn value_set_validate(
        &self,
        terminology_id: &str,
        value_set_id: &str,
        candidate_code: &str,
        at_date: Option<String>,
    ) -> Result<bool, SmError> {
        if bundle::has_terminology(terminology_id) {
            bundle::value_set_validate(terminology_id, value_set_id, candidate_code)
        } else if let Some(p) = &self.external_terminology {
            p.value_set_validate(terminology_id, value_set_id, candidate_code, at_date)
                .await
        } else {
            bundle::value_set_validate(terminology_id, value_set_id, candidate_code)
        }
    }

    pub async fn has_value_set(
        &self,
        terminology_id: &str,
        value_set_code: &str,
    ) -> Result<bool, SmError> {
        if bundle::has_terminology(terminology_id) {
            Ok(bundle::has_value_set(terminology_id, value_set_code))
        } else if let Some(p) = &self.external_terminology {
            p.has_value_set(terminology_id, value_set_code).await
        } else {
            Ok(bundle::has_value_set(terminology_id, value_set_code))
        }
    }

    pub async fn get_value_set(
        &self,
        terminology_id: &str,
        value_set_code: &str,
    ) -> Result<TerminologyExtract, SmError> {
        if bundle::has_terminology(terminology_id) {
            bundle::get_value_set(terminology_id, value_set_code)
        } else if let Some(p) = &self.external_terminology {
            p.get_value_set(terminology_id, value_set_code).await
        } else {
            bundle::get_value_set(terminology_id, value_set_code)
        }
    }
}

// ─── the AQL `TERMINOLOGY()` seam ─────────────────────────────────────────────
//
// [`EhrbaseService`] implements the `TerminologyExpander` trait
// (`crate::aql::terminology`) so the AQL semantic-analysis pass
// (`aql/terminology.rs::expand_matches`) can resolve `TERMINOLOGY('expand', …)`
// operands through this service's providers.

#[async_trait]
impl TerminologyExpander for EhrbaseService {
    /// Resolve `TERMINOLOGY('expand', service_api, params_uri)` to the value
    /// set's codes: route by `service_api` (FHIR → the configured remote
    /// provider; `"openehr"` → the in-process bundle), fetch the expansion via
    /// the SM `get_value_set`, and return its code keys.
    ///
    /// A missing FHIR provider or an unrecognised `service_api` is a 400
    /// ([`AqlFeatureError::UnknownTerminologyService`]); an unknown value set is
    /// a 400 ([`AqlFeatureError::TerminologyValueSetNotFound`]); any other
    /// (server/transport) failure is a 500 ([`ExecError::Terminology`]).
    async fn expand(&self, service_api: &str, params_uri: &str) -> Result<Vec<String>, AqlError> {
        let extract = if is_fhir_service_api(service_api) {
            let provider = self.external_terminology.as_ref().ok_or_else(|| {
                AqlFeatureError::UnknownTerminologyService(format!(
                    "{service_api} (no FHIR terminology server configured)"
                ))
            })?;
            // The FHIR provider ignores `terminology_id` for `$expand`; the
            // value set is identified by `params_uri` (its URL).
            provider.get_value_set(service_api, params_uri).await
        } else if service_api == BUNDLE_SERVICE_API {
            // The bundle's value sets live under the `"openehr"` terminology;
            // `params_uri` is the group / code-set id.
            bundle::get_value_set(OPENEHR, params_uri)
        } else {
            return Err(AqlFeatureError::UnknownTerminologyService(service_api.to_owned()).into());
        };
        let extract = extract.map_err(|e| map_expand_error(e, service_api, params_uri))?;
        Ok(extract
            .terms
            .map(|terms| terms.into_keys().collect())
            .unwrap_or_default())
    }

    /// Evaluate the Boolean `TERMINOLOGY()` operations (QUERY master03
    /// §TERMINOLOGY, third usage — `TERMINOLOGY('validate', …) = true`):
    /// `validate` → `value_set_validate` (args from the `params_uri` query
    /// string: `url` = the value set, `system` = the terminology, `code` = the
    /// candidate); `subsumes` → `subsumes` (`system`, `codeA`, `codeB`).
    /// `lookup`/`map` return complex structures with no boolean semantics —
    /// typed reject.
    async fn boolean_operation(
        &self,
        operation: &str,
        service_api: &str,
        params_uri: &str,
    ) -> Result<bool, AqlError> {
        let args = uri_query_args(params_uri);
        let arg = |name: &'static str| -> Result<&str, AqlError> {
            args.get(name)
                .map(String::as_str)
                .ok_or_else(|| AqlFeatureError::TerminologyParams(name).into())
        };
        let fhir = is_fhir_service_api(service_api);
        if !fhir && service_api != BUNDLE_SERVICE_API {
            return Err(AqlFeatureError::UnknownTerminologyService(service_api.to_owned()).into());
        }
        let fhir_provider = || {
            self.external_terminology.as_ref().ok_or_else(|| {
                AqlFeatureError::UnknownTerminologyService(format!(
                    "{service_api} (no FHIR terminology server configured)"
                ))
            })
        };
        let result = match operation.to_ascii_lowercase().as_str() {
            "validate" => {
                let (system, url, code) = (arg("system")?, arg("url")?, arg("code")?);
                if fhir {
                    fhir_provider()?
                        .value_set_validate(system, url, code, None)
                        .await
                } else {
                    bundle::value_set_validate(OPENEHR, url, code)
                }
            }
            "subsumes" => {
                let (system, code_a, code_b) = (arg("system")?, arg("codeA")?, arg("codeB")?);
                if fhir {
                    fhir_provider()?.subsumes(system, code_a, code_b).await
                } else {
                    bundle::subsumes(system, code_a, code_b)
                }
            }
            other => {
                return Err(
                    AqlFeatureError::UnsupportedTerminologyOperation(other.to_owned()).into(),
                );
            }
        };
        result.map_err(|e| map_expand_error(e, service_api, params_uri))
    }

    /// Expand a terminology URI operand (`matches { terminology://… }` —
    /// QUERY master03 §matches/URI): the URI identifies a value set; matching
    /// is membership of its expansion. Routed to the in-process bundle for a
    /// `terminology://openehr/<set>` URI, else to the configured FHIR provider
    /// (the URI is the value-set identifier).
    async fn expand_uri(&self, uri: &str) -> Result<Vec<String>, AqlError> {
        if let Some(rest) = uri
            .strip_prefix("terminology://openehr/")
            .or_else(|| uri.strip_prefix("terminology://openEHR/"))
        {
            let set = rest.split('?').next().unwrap_or(rest);
            let extract = bundle::get_value_set(OPENEHR, set)
                .map_err(|e| map_expand_error(e, "openehr", uri))?;
            return Ok(extract
                .terms
                .map(|terms| terms.into_keys().collect())
                .unwrap_or_default());
        }
        let provider = self.external_terminology.as_ref().ok_or_else(|| {
            AqlFeatureError::UnknownTerminologyService(format!(
                "{uri} (no FHIR terminology server configured for URI operands)"
            ))
        })?;
        let extract = provider
            .get_value_set("", uri)
            .await
            .map_err(|e| map_expand_error(e, "fhir", uri))?;
        Ok(extract
            .terms
            .map(|terms| terms.into_keys().collect())
            .unwrap_or_default())
    }
}

/// The query args of a `params_uri` (`system=…&code=…&url=…`), from the part
/// after the first `?` when present, else the whole string. Values are
/// form-urlencoded (`+` → space) and percent-decoded (`urlencoding`, WHATWG
/// recovery semantics).
fn uri_query_args(params_uri: &str) -> BTreeMap<String, String> {
    let query = params_uri.split_once('?').map_or(params_uri, |(_, q)| q);
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(k, v)| {
            let plus_decoded = v.replace('+', " ");
            (
                k.trim().to_owned(),
                String::from_utf8_lossy(&urlencoding::decode_binary(plus_decoded.as_bytes()))
                    .into_owned(),
            )
        })
        .collect()
}

/// Map a terminology-service [`SmError`] raised during `expand` onto the AQL
/// error taxonomy: a "does not exist" status is a bad query (400 — the value
/// set is unknown); anything else is an upstream server fault (500).
fn map_expand_error(e: SmError, service_api: &str, params_uri: &str) -> AqlError {
    match e.status {
        CallStatusType::VersionedObjectDoesNotExist => {
            AqlFeatureError::TerminologyValueSetNotFound {
                service_api: service_api.to_owned(),
                value_set: params_uri.to_owned(),
            }
            .into()
        }
        _ => ExecError::Terminology(e.message).into(),
    }
}
