//! The `ehrbase-rest` backend seams, implemented on [`EhrbaseService`].
//!
//! [`ehr`] implements the [`ehrbase_rest::EhrService`] envelope seam (the whole
//! EHR / `EHR_STATUS` / COMPOSITION / DIRECTORY / CONTRIBUTION surface);
//! [`definition`] implements the generated `DefinitionApi` (templates + stored
//! queries). [`ehrbase_rest::WebTemplateService`] exposes the service-owned
//! `WebTemplate` cache to the REST layer (one resolution for validation, FLAT
//! / STRUCTURED, and `wt+json` — W2-K/F-13-02).
//!
//! [`query`] implements the [`ehrbase_rest::QueryService`] seam (ad-hoc + stored
//! AQL execution on the P16 engine). [`demographic`] implements the
//! [`ehrbase_rest::DemographicService`] seam (PARTY CRUD + `VERSIONED_PARTY` on
//! the shared vobject machinery, no EHR scope); [`admin`] implements
//! [`ehrbase_rest::AdminService`] (physical EHR delete, SM `I_ADMIN_SERVICE`).

mod admin;
mod definition;
mod demographic;
mod ehr;
mod ehr_index;
mod query;
mod relationship;

use std::sync::Arc;

use async_trait::async_trait;

use ehrbase_rest::WebTemplateService;
use ehrbase_sm::ValidityChecker;
use openehr_flat::WebTemplate;
use openehr_its::rest::runtime::ApiError;
use serde_json::Value;

use super::vobject::Kind;
use super::{EhrbaseService, ServiceError};

#[async_trait]
impl WebTemplateService for EhrbaseService {
    async fn web_template(&self, template_id: &str) -> Result<Arc<WebTemplate>, ApiError> {
        Ok(self.web_template_for(template_id).await?)
    }
}

/// SM `I_VALIDITY_CHECKER` (`i_validity_checker.adoc`), realized on the
/// existing validation choke points.
///
/// PORT NOTE: `definitions_valid` checks **template** identifiers only — the
/// spec says "archetype and template identifiers", but there is no archetype
/// store until SM-2 (`docs/design/sm-platform/09-roadmap.md`); content that
/// declares no template resolves `true` (nothing to look up). `content_valid`
/// runs the same per-kind structural validation every commit runs
/// (`validate_for_commit`); an unrecognized root `_type` is `false`.
#[async_trait]
impl ValidityChecker for EhrbaseService {
    async fn definitions_valid(&self, a_content: &Value) -> Result<bool, ApiError> {
        let template_id = a_content
            .pointer("/archetype_details/template_id/value")
            .and_then(Value::as_str);
        match template_id {
            None => Ok(true),
            Some(id) => Ok(self.web_template_for(id).await.is_ok()),
        }
    }

    async fn content_valid(&self, a_content: &Value) -> Result<bool, ApiError> {
        let rm_type = a_content.get("_type").and_then(Value::as_str).unwrap_or("");
        let Some(kind) = Kind::from_type(rm_type) else {
            return Ok(false);
        };
        match self.validate_for_commit(kind, a_content).await {
            Ok(()) => Ok(true),
            Err(ServiceError::ValidationFailed(_) | ServiceError::Unprocessable(_)) => Ok(false),
            Err(other) => Err(other.into()),
        }
    }
}
