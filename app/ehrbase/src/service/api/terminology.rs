//! [`TerminologyService`] on [`EhrbaseService`] (SM `I_TERMINOLOGY_SERVICE`).
//!
//! Thin async adapter over the DB-free bundle mapping in
//! [`crate::service::terminology`]; every precondition/error decision lives
//! there (spec citations + PORT NOTEs in that module).

use std::collections::BTreeMap;

use async_trait::async_trait;

use ehrbase_sm::SmError;
use ehrbase_sm::{TerminologyDescription, TerminologyExtract, TerminologyService};

use crate::service::EhrbaseService;
use crate::service::terminology as term;

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
