//! `DefinitionApi` — stored-query CRUD (on the `stored_query` table) and OPT 1.4
//! operational-template CRUD (on the `template_store` table, ingested into the
//! `openehr-its::opt14` model). The `adl2` template methods inherit the generated
//! `NotImplemented` (501) default: ADL2 is OPTIONAL for openEHR CNF platform
//! conformance and untested by the current kit; it awaits the ADL2 text parser.

use async_trait::async_trait;
use serde_json::Value;

use ehrbase_sm::{DefinitionAdl14Service, DefinitionQueryService, Page, QueryDescriptor};
use openehr_flat::{DetailLevel, ExampleType};
use openehr_its::rest::generated::definition::{
    DefinitionApi, DefinitionQueryListParams, DefinitionQueryStoreYamlParams,
    DefinitionQueryVersionGetParams, DefinitionQueryVersionStoreYamlParams,
    DefinitionTemplateAdl14ExampleGetParams, DefinitionTemplateAdl14GetParams,
    DefinitionTemplateAdl14ListParams, DefinitionTemplateAdl14UploadParams,
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

    async fn definition_template_adl1_4_example_get(
        &self,
        params: DefinitionTemplateAdl14ExampleGetParams,
    ) -> Result<Value, ApiError> {
        // `type`/`detail_level` are the dev-OAS `example_type`/`example_detail_level`
        // enums (`definition-validation.openapi.yaml`); an out-of-enum value is a
        // `400 Bad Request` (the endpoint's `400` response). All three detail
        // levels are implemented, so no "unsupported level" fallback applies.
        let level = DetailLevel::from_query(params.detail_level.as_deref())
            .map_err(ApiError::BadRequest)?;
        let kind =
            ExampleType::from_query(params.r#type.as_deref()).map_err(ApiError::BadRequest)?;
        Ok(self
            .template_example(&params.template_id, level, kind)
            .await?)
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
        // The effective version is returned for the caller's `Location` header,
        // but the generated trait method is bodyless (`()`); the dispatcher
        // builds `Location` from the resolved name/version it already holds.
        self.store_query(&params.qualified_query_name, None, body)
            .await?;
        Ok(())
    }

    async fn definition_query_version_store_yaml(
        &self,
        params: DefinitionQueryVersionStoreYamlParams,
        body: String,
    ) -> Result<(), ApiError> {
        self.store_query(&params.qualified_query_name, Some(&params.version), body)
            .await?;
        Ok(())
    }
}

// ── SM Definitions native API (SM-2) ─────────────────────────────────────────
//
// These realize the SM `I_DEFINITION_ADL14` / `I_DEFINITION_QUERY` interfaces
// (`docs/specs/openehr/SM/docs/UML/classes/{i_definition_adl14,i_definition_query}.adoc`).
// They join `Backend` but drive no ITS-REST route — the DEFINITION wire above is
// unchanged (SM-2 is native-API only). The bodies delegate to the `service::definition`
// logic; `ServiceError` → `ApiError` conversion happens at the `?` boundary.

#[async_trait]
impl DefinitionAdl14Service for EhrbaseService {
    async fn has_archetype(&self, an_id: String) -> Result<bool, ApiError> {
        Ok(self.archetype_exists(&an_id).await?)
    }

    async fn valid_archetype(&self, adl: String) -> Result<bool, ApiError> {
        Ok(Self::valid_archetype_source(&adl))
    }

    async fn upload_archetype(&self, adl: String) -> Result<(), ApiError> {
        Ok(self.archetype_upload(&adl).await?)
    }

    async fn get_archetype(&self, an_id: String) -> Result<String, ApiError> {
        Ok(self.archetype_get(&an_id).await?)
    }

    async fn list_archetypes(&self, page: Page) -> Result<Vec<String>, ApiError> {
        Ok(self.archetype_list(page).await?)
    }

    async fn list_matching_archetypes(
        &self,
        id_pattern: String,
        page: Page,
    ) -> Result<Vec<String>, ApiError> {
        Ok(self.archetype_list_matching(&id_pattern, page).await?)
    }

    async fn delete_archetype(&self, an_id: String) -> Result<(), ApiError> {
        Ok(self.archetype_delete(&an_id).await?)
    }

    async fn archetypes_count(&self) -> Result<i64, ApiError> {
        Ok(self.archetype_count().await?)
    }

    async fn has_opt(&self, an_opt_id: String) -> Result<bool, ApiError> {
        Ok(self.opt_exists(&an_opt_id).await?)
    }

    async fn valid_opt(&self, opt_xml: String) -> Result<bool, ApiError> {
        Ok(Self::valid_opt_xml(&opt_xml))
    }

    async fn upload_opt(&self, opt_xml: String) -> Result<(), ApiError> {
        // Delegate to the existing OPT ingestion: parse + structural validation
        // (→ 422 `invalid_template` on failure) and the 409-on-duplicate rule.
        self.store_template(&opt_xml).await?;
        Ok(())
    }

    async fn get_opt(&self, an_opt_id: String) -> Result<String, ApiError> {
        Ok(self.opt_get(&an_opt_id).await?)
    }

    async fn list_opts(&self, page: Page) -> Result<Vec<String>, ApiError> {
        Ok(self.opt_list(page).await?)
    }

    async fn list_matching_opts(
        &self,
        id_pattern: String,
        page: Page,
    ) -> Result<Vec<String>, ApiError> {
        Ok(self.opt_list_matching(&id_pattern, page).await?)
    }

    async fn delete_opt(&self, an_opt_id: String) -> Result<(), ApiError> {
        Ok(self.opt_delete(&an_opt_id).await?)
    }

    async fn opts_count(&self) -> Result<i64, ApiError> {
        Ok(self.opt_count().await?)
    }
}

#[async_trait]
impl DefinitionQueryService for EhrbaseService {
    async fn has_query(&self, a_query_name: String) -> Result<bool, ApiError> {
        Ok(self.query_exists(&a_query_name).await?)
    }

    async fn valid_query(&self, a_query_text: String, a_type: String) -> Result<bool, ApiError> {
        Ok(Self::valid_query_source(&a_query_text, &a_type))
    }

    async fn store_query(
        &self,
        a_query_text: String,
        a_type: String,
        a_query_name: Option<String>,
    ) -> Result<QueryDescriptor, ApiError> {
        Ok(self
            .query_store_sm(a_query_text, &a_type, a_query_name)
            .await?)
    }

    // `store_query_set` keeps the trait default (`NotImplemented`) — the SM
    // entry is an explicit TODO with no defined semantics.

    async fn list_queries(&self, page: Page) -> Result<Vec<QueryDescriptor>, ApiError> {
        Ok(self.query_list(page).await?)
    }

    async fn list_matching_queries(
        &self,
        id_pattern: String,
        artefact_id_pattern: Option<String>,
        page: Page,
    ) -> Result<Vec<QueryDescriptor>, ApiError> {
        Ok(self
            .query_list_matching(&id_pattern, artefact_id_pattern.as_deref(), page)
            .await?)
    }

    async fn delete_query(&self, a_query_name: String) -> Result<(), ApiError> {
        Ok(self.query_delete(&a_query_name).await?)
    }

    async fn queries_count(&self) -> Result<i64, ApiError> {
        Ok(self.query_count().await?)
    }
}
