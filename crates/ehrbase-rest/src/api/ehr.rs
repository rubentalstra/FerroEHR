//! `EhrApi` implementation (Stage-1 `NotImplemented` stubs; P12 fills them).

use serde_json::Value;

use openehr_its::rest::generated::ehr::{
    CompositionCreateParams, CompositionDeleteParams, CompositionGetParams,
    CompositionTagsDeleteParams, CompositionTagsGetParams, CompositionTagsUpdateParams,
    CompositionUpdateParams, ContributionCreateParams, ContributionGetParams,
    DirectoryCreateParams, DirectoryDeleteParams, DirectoryGetAtTimeParams,
    DirectoryGetByVersionIdParams, DirectoryUpdateParams, EhrApi, EhrCreateParams,
    EhrCreateWithIdParams, EhrGetByIdParams, EhrGetBySubjectParams, EhrStatusGetAtTimeParams,
    EhrStatusGetByVersionIdParams, EhrStatusTagsDeleteParams, EhrStatusTagsGetParams,
    EhrStatusTagsUpdateParams, EhrStatusUpdateParams, EhrTagsGetParams,
    VersionedCompositionGetParams, VersionedCompositionRevisionHistoryParams,
    VersionedCompositionVersionGetAtTimeParams, VersionedCompositionVersionGetByIdParams,
    VersionedEhrStatusGetParams, VersionedEhrStatusRevisionHistoryParams,
    VersionedEhrStatusVersionGetAtTimeParams, VersionedEhrStatusVersionGetByIdParams,
};

type Tags = Vec<std::collections::BTreeMap<String, Value>>;

crate::api::stub_api!(EhrApi, {
    ehr_get_by_subject(EhrGetBySubjectParams) -> Value;
    ehr_create(EhrCreateParams, Option<Value>) -> Value;
    ehr_get_by_id(EhrGetByIdParams) -> Value;
    ehr_create_with_id(EhrCreateWithIdParams, Option<Value>) -> Value;
    ehr_status_get_by_version_id(EhrStatusGetByVersionIdParams) -> Value;
    ehr_status_get_at_time(EhrStatusGetAtTimeParams) -> Value;
    ehr_status_update(EhrStatusUpdateParams, Value) -> Value;
    versioned_ehr_status_get(VersionedEhrStatusGetParams) -> Value;
    versioned_ehr_status_revision_history(VersionedEhrStatusRevisionHistoryParams) -> Value;
    versioned_ehr_status_version_get_at_time(VersionedEhrStatusVersionGetAtTimeParams) -> Value;
    versioned_ehr_status_version_get_by_id(VersionedEhrStatusVersionGetByIdParams) -> Value;
    composition_create(CompositionCreateParams, Value) -> Value;
    composition_get(CompositionGetParams) -> Value;
    composition_update(CompositionUpdateParams, Value) -> Value;
    composition_delete(CompositionDeleteParams) -> ();
    versioned_composition_get(VersionedCompositionGetParams) -> Value;
    versioned_composition_revision_history(VersionedCompositionRevisionHistoryParams) -> Value;
    versioned_composition_version_get_at_time(VersionedCompositionVersionGetAtTimeParams) -> Value;
    versioned_composition_version_get_by_id(VersionedCompositionVersionGetByIdParams) -> Value;
    directory_get_at_time(DirectoryGetAtTimeParams) -> Value;
    directory_update(DirectoryUpdateParams, Value) -> Value;
    directory_create(DirectoryCreateParams, Value) -> Value;
    directory_delete(DirectoryDeleteParams) -> ();
    directory_get_by_version_id(DirectoryGetByVersionIdParams) -> Value;
    contribution_create(ContributionCreateParams, Value) -> Value;
    contribution_get(ContributionGetParams) -> Value;
    ehr_tags_get(EhrTagsGetParams) -> Tags;
    composition_tags_get(CompositionTagsGetParams) -> Tags;
    composition_tags_update(CompositionTagsUpdateParams, Vec<Value>) -> Tags;
    composition_tags_delete(CompositionTagsDeleteParams) -> ();
    ehr_status_tags_get(EhrStatusTagsGetParams) -> Tags;
    ehr_status_tags_update(EhrStatusTagsUpdateParams, Vec<Value>) -> Tags;
    ehr_status_tags_delete(EhrStatusTagsDeleteParams) -> ();
});
