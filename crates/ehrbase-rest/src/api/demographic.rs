//! `DemographicApi` implementation (Stage-1 `NotImplemented` stubs; P12 fills them).

use serde_json::Value;

use openehr_its::rest::generated::demographic::{
    AgentCreateParams, AgentDeleteParams, AgentGetParams, AgentTagsDeleteParams,
    AgentTagsGetParams, AgentTagsUpdateParams, AgentUpdateParams, ContributionCreateParams,
    ContributionGetParams, DemographicApi, DemographicTagsGetParams, GroupCreateParams,
    GroupDeleteParams, GroupGetParams, GroupTagsDeleteParams, GroupTagsGetParams,
    GroupTagsUpdateParams, GroupUpdateParams, OrganisationCreateParams, OrganisationDeleteParams,
    OrganisationGetParams, OrganisationTagsDeleteParams, OrganisationTagsGetParams,
    OrganisationTagsUpdateParams, OrganisationUpdateParams, PersonCreateParams, PersonDeleteParams,
    PersonGetParams, PersonTagsDeleteParams, PersonTagsGetParams, PersonTagsUpdateParams,
    PersonUpdateParams, RoleCreateParams, RoleDeleteParams, RoleGetParams, RoleTagsDeleteParams,
    RoleTagsGetParams, RoleTagsUpdateParams, RoleUpdateParams, VersionedPartyGetParams,
    VersionedPartyRevisionHistoryParams, VersionedPartyVersionGetAtTimeParams,
    VersionedPartyVersionGetByIdParams,
};

type Tags = Vec<std::collections::BTreeMap<String, Value>>;

crate::api::stub_api!(DemographicApi, {
    agent_create(AgentCreateParams, Value) -> Value;
    agent_get(AgentGetParams) -> Value;
    agent_update(AgentUpdateParams, Value) -> Value;
    agent_delete(AgentDeleteParams) -> ();
    group_create(GroupCreateParams, Value) -> Value;
    group_get(GroupGetParams) -> Value;
    group_update(GroupUpdateParams, Value) -> Value;
    group_delete(GroupDeleteParams) -> ();
    organisation_create(OrganisationCreateParams, Value) -> Value;
    organisation_get(OrganisationGetParams) -> Value;
    organisation_update(OrganisationUpdateParams, Value) -> Value;
    organisation_delete(OrganisationDeleteParams) -> ();
    person_create(PersonCreateParams, Value) -> Value;
    person_get(PersonGetParams) -> Value;
    person_update(PersonUpdateParams, Value) -> Value;
    person_delete(PersonDeleteParams) -> ();
    role_create(RoleCreateParams, Value) -> Value;
    role_get(RoleGetParams) -> Value;
    role_update(RoleUpdateParams, Value) -> Value;
    role_delete(RoleDeleteParams) -> ();
    versioned_party_get(VersionedPartyGetParams) -> Value;
    versioned_party_revision_history(VersionedPartyRevisionHistoryParams) -> Value;
    versioned_party_version_get_at_time(VersionedPartyVersionGetAtTimeParams) -> Value;
    versioned_party_version_get_by_id(VersionedPartyVersionGetByIdParams) -> Value;
    contribution_create(ContributionCreateParams, Value) -> Value;
    contribution_get(ContributionGetParams) -> Value;
    demographic_tags_get(DemographicTagsGetParams) -> Tags;
    agent_tags_get(AgentTagsGetParams) -> Tags;
    agent_tags_update(AgentTagsUpdateParams, Vec<Value>) -> Tags;
    agent_tags_delete(AgentTagsDeleteParams) -> ();
    group_tags_get(GroupTagsGetParams) -> Tags;
    group_tags_update(GroupTagsUpdateParams, Vec<Value>) -> Tags;
    group_tags_delete(GroupTagsDeleteParams) -> ();
    organisation_tags_get(OrganisationTagsGetParams) -> Tags;
    organisation_tags_update(OrganisationTagsUpdateParams, Vec<Value>) -> Tags;
    organisation_tags_delete(OrganisationTagsDeleteParams) -> ();
    person_tags_get(PersonTagsGetParams) -> Tags;
    person_tags_update(PersonTagsUpdateParams, Vec<Value>) -> Tags;
    person_tags_delete(PersonTagsDeleteParams) -> ();
    role_tags_get(RoleTagsGetParams) -> Tags;
    role_tags_update(RoleTagsUpdateParams, Vec<Value>) -> Tags;
    role_tags_delete(RoleTagsDeleteParams) -> ();
});
