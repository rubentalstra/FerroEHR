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
//! AQL execution on the P16 engine). The remaining unimplemented API groups
//! (demographic: a future RM phase; admin: later) are not part of the
//! [`ehrbase_rest::Backend`] seam — their routes answer 501 through the REST
//! layer's generic not-implemented dispatcher (F-13-03).

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
