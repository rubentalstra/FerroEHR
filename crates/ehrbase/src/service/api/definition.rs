//! `DefinitionApi` — the stored-query methods (on the `stored_query` table).
//! The template methods (`definition_template_*`) inherit the generated
//! `NotImplemented` default until template ingestion (P13).

use async_trait::async_trait;
use serde_json::Value;

use openehr_its::rest::generated::definition::{
    DefinitionApi, DefinitionQueryListParams, DefinitionQueryStoreYamlParams,
    DefinitionQueryVersionGetParams, DefinitionQueryVersionStoreYamlParams,
};
use openehr_its::rest::runtime::ApiError;

use crate::service::EhrbaseService;

#[async_trait]
impl DefinitionApi for EhrbaseService {
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
