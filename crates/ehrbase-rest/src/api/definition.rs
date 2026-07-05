//! `DefinitionApi` implementation (Stage-1 `NotImplemented` stubs; P12 fills them).

use serde_json::Value;

use openehr_its::rest::generated::definition::{
    DefinitionApi, DefinitionQueryListParams, DefinitionQueryStoreYamlParams,
    DefinitionQueryVersionGetParams, DefinitionQueryVersionStoreYamlParams,
    DefinitionTemplateAdl2ExampleGetParams, DefinitionTemplateAdl2GetParams,
    DefinitionTemplateAdl2ListParams, DefinitionTemplateAdl2UploadParams,
    DefinitionTemplateAdl2VersionGetParams, DefinitionTemplateAdl14ExampleGetParams,
    DefinitionTemplateAdl14GetParams, DefinitionTemplateAdl14ListParams,
    DefinitionTemplateAdl14UploadParams,
};

type Template = std::collections::BTreeMap<String, Value>;

crate::api::stub_api!(DefinitionApi, {
    definition_template_adl1_4_list(DefinitionTemplateAdl14ListParams) -> Vec<Value>;
    definition_template_adl1_4_upload(DefinitionTemplateAdl14UploadParams, Value) -> Value;
    definition_template_adl1_4_get(DefinitionTemplateAdl14GetParams) -> Value;
    definition_template_adl1_4_example_get(DefinitionTemplateAdl14ExampleGetParams) -> Value;
    definition_template_adl2_list(DefinitionTemplateAdl2ListParams) -> Vec<Value>;
    definition_template_adl2_upload(DefinitionTemplateAdl2UploadParams, Value) -> Value;
    definition_template_adl2_get(DefinitionTemplateAdl2GetParams) -> Template;
    definition_template_adl2_example_get(DefinitionTemplateAdl2ExampleGetParams) -> Value;
    definition_template_adl2_version_get(DefinitionTemplateAdl2VersionGetParams) -> Template;
    definition_query_list(DefinitionQueryListParams) -> Vec<Value>;
    definition_query_store_yaml(DefinitionQueryStoreYamlParams, String) -> ();
    definition_query_version_get(DefinitionQueryVersionGetParams) -> Value;
    definition_query_version_store_yaml(DefinitionQueryVersionStoreYamlParams, String) -> ();
});
