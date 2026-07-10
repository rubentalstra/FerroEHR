//! [`TerminologyService`] on [`EhrbaseService`] (SM `I_TERMINOLOGY_SERVICE`).
//!
//! Thin async adapter over the DB-free bundle mapping in
//! [`crate::service::terminology`]; every precondition/error decision lives
//! there (spec citations + PORT NOTEs in that module).

use std::collections::BTreeMap;

use async_trait::async_trait;

use ehrbase_sm::SmError;
use ehrbase_sm::{CallStatusType, TerminologyDescription, TerminologyExtract, TerminologyService};

use crate::aql::TerminologyExpander;
use crate::aql::error::{AqlError, AqlFeatureError, ExecError};
use crate::service::EhrbaseService;
use crate::service::codes::OPENEHR;
use crate::service::terminology as term;

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
            term::get_value_set(OPENEHR, params_uri)
        } else {
            return Err(AqlFeatureError::UnknownTerminologyService(service_api.to_owned()).into());
        };
        let extract = extract.map_err(|e| map_expand_error(e, service_api, params_uri))?;
        Ok(extract
            .terms
            .map(|terms| terms.into_keys().collect())
            .unwrap_or_default())
    }
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

#[async_trait]
impl TerminologyService for EhrbaseService {
    async fn get_terminology_ids(&self) -> Result<Vec<String>, SmError> {
        Ok(term::terminology_ids())
    }

    async fn has_terminology(&self, terminology_id: &str) -> Result<bool, SmError> {
        Ok(term::has_terminology(terminology_id))
    }

    async fn get_terminology_description(
        &self,
        terminology_id: &str,
    ) -> Result<TerminologyDescription, SmError> {
        term::terminology_description(terminology_id)
    }

    async fn has_term(
        &self,
        terminology_id: &str,
        code: &str,
        _at_date: Option<String>,
    ) -> Result<bool, SmError> {
        // `at_date` accepted; single pinned bundle version (module PORT NOTE).
        term::has_term(terminology_id, code)
    }

    async fn get_term(
        &self,
        terminology_id: &str,
        code: &str,
        _attributes: Option<BTreeMap<String, String>>,
        _at_date: Option<String>,
    ) -> Result<TerminologyExtract, SmError> {
        // No per-term meta-model attributes are exposed, so `attributes` (an
        // allow-list filter) is accepted and has no effect.
        term::get_term(terminology_id, code)
    }

    async fn subsumes(
        &self,
        terminology_id: &str,
        ref_code: &str,
        candidate_child_code: &str,
    ) -> Result<bool, SmError> {
        term::subsumes(terminology_id, ref_code, candidate_child_code)
    }

    async fn value_set_validate(
        &self,
        terminology_id: &str,
        value_set_id: &str,
        candidate_code: &str,
        _at_date: Option<String>,
    ) -> Result<bool, SmError> {
        term::value_set_validate(terminology_id, value_set_id, candidate_code)
    }

    async fn has_value_set(
        &self,
        terminology_id: &str,
        value_set_code: &str,
    ) -> Result<bool, SmError> {
        Ok(term::has_value_set(terminology_id, value_set_code))
    }

    async fn get_value_set(
        &self,
        terminology_id: &str,
        value_set_code: &str,
    ) -> Result<TerminologyExtract, SmError> {
        term::get_value_set(terminology_id, value_set_code)
    }
}
