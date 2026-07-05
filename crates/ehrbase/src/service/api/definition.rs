//! `DefinitionApi` — stored-query CRUD (on the `stored_query` table) and OPT 1.4
//! operational-template CRUD (on the `template_store` table, ingested into the
//! `openehr-its::opt14` model). The `adl2` template methods inherit the generated
//! `NotImplemented` (501) default: ADL2 is OPTIONAL for openEHR CNF platform
//! conformance and untested by the current kit; it awaits the ADL2 text parser.

use async_trait::async_trait;
use serde_json::Value;

use openehr_its::rest::generated::definition::{
    DefinitionApi, DefinitionQueryListParams, DefinitionQueryStoreYamlParams,
    DefinitionQueryVersionGetParams, DefinitionQueryVersionStoreYamlParams,
    DefinitionTemplateAdl14GetParams, DefinitionTemplateAdl14ListParams,
    DefinitionTemplateAdl14UploadParams,
};
use openehr_its::rest::runtime::ApiError;

use crate::service::EhrbaseService;

#[async_trait]
impl DefinitionApi for EhrbaseService {
    // ── OPT 1.4 operational templates (ADL 1.4 — CNF-required) ───────────────
    async fn definition_template_adl1_4_upload(
        &self,
        _params: DefinitionTemplateAdl14UploadParams,
        body: Value,
    ) -> Result<Value, ApiError> {
        // The upload body is the OPT 1.4 canonical XML (decoded upstream to a
        // JSON string by the lenient body reader).
        let xml = body.as_str().ok_or_else(|| {
            ApiError::BadRequest("expected an OPT 1.4 XML template body".to_owned())
        })?;
        Ok(self.store_template(xml).await?)
    }

    async fn definition_template_adl1_4_list(
        &self,
        _params: DefinitionTemplateAdl14ListParams,
    ) -> Result<Vec<Value>, ApiError> {
        Ok(self.list_templates().await?)
    }

    async fn definition_template_adl1_4_get(
        &self,
        params: DefinitionTemplateAdl14GetParams,
    ) -> Result<Value, ApiError> {
        // The canonical retrieval artifact is the stored OPT XML; returned as a
        // JSON string that the dispatcher serves as `application/xml`.
        Ok(Value::String(
            self.get_template_xml(&params.template_id).await?,
        ))
    }

    async fn definition_query_list(
        &self,
        params: DefinitionQueryListParams,
    ) -> Result<Vec<Value>, ApiError> {
        Ok(self
            .list_stored_queries(&params.qualified_query_name)
            .await?)
    }

    async fn definition_query_version_get(
        &self,
        params: DefinitionQueryVersionGetParams,
    ) -> Result<Value, ApiError> {
        Ok(self
            .get_stored_query(&params.qualified_query_name, Some(&params.version))
            .await?)
    }

    async fn definition_query_store_yaml(
        &self,
        params: DefinitionQueryStoreYamlParams,
        body: String,
    ) -> Result<(), ApiError> {
        Ok(self
            .store_query(&params.qualified_query_name, None, body)
            .await?)
    }

    async fn definition_query_version_store_yaml(
        &self,
        params: DefinitionQueryVersionStoreYamlParams,
        body: String,
    ) -> Result<(), ApiError> {
        Ok(self
            .store_query(&params.qualified_query_name, Some(&params.version), body)
            .await?)
    }
}
