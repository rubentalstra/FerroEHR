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
mod query;

use std::sync::Arc;

use async_trait::async_trait;

use ehrbase_rest::WebTemplateService;
use openehr_flat::WebTemplate;
use openehr_its::rest::runtime::ApiError;

use super::EhrbaseService;

#[async_trait]
impl WebTemplateService for EhrbaseService {
    async fn web_template(&self, template_id: &str) -> Result<Arc<WebTemplate>, ApiError> {
        Ok(self.web_template_for(template_id).await?)
    }
}
