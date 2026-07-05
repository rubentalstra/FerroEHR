//! HTTP dispatch for the `demographic` API group.
//!
//! Each arm rebuilds the operation's `*Params`, decodes any body (JSON only for
//! this group in Stage 1), calls the trait method on [`AppState`], and renders a
//! negotiated response. Handlers currently return `NotImplemented`; that surfaces
//! here as a 501 response.

use axum::response::{IntoResponse, Response};
use http::StatusCode;

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

use super::{BoxResponse, RequestParts};
use crate::error::RestError;
use crate::state::AppState;
use crate::{negotiate, params};

pub(super) fn dispatch(state: AppState, op: &'static str, parts: RequestParts) -> BoxResponse {
    Box::pin(async move {
        run(state, op, parts)
            .await
            .unwrap_or_else(IntoResponse::into_response)
    })
}

#[allow(clippy::too_many_lines)] // one arm per operation; a flat match is clearest
async fn run(
    state: AppState,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let ok = StatusCode::OK;

    match op {
        "agent_create" => {
            let p = params::build::<AgentCreateParams>(&parts.path, q, h)?;
            // TODO(port): P12 — typed XML demographic bodies.
            let body = negotiate::json_value(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                StatusCode::CREATED,
                &state.backend().agent_create(p, body).await?,
            ))
        }
        "agent_get" => {
            let p = params::build::<AgentGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().agent_get(p).await?,
            ))
        }
        "agent_update" => {
            let p = params::build::<AgentUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::json_value(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().agent_update(p, body).await?,
            ))
        }
        "agent_delete" => {
            let p = params::build::<AgentDeleteParams>(&parts.path, q, h)?;
            state.backend().agent_delete(p).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        "group_create" => {
            let p = params::build::<GroupCreateParams>(&parts.path, q, h)?;
            // TODO(port): P12 — typed XML demographic bodies.
            let body = negotiate::json_value(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                StatusCode::CREATED,
                &state.backend().group_create(p, body).await?,
            ))
        }
        "group_get" => {
            let p = params::build::<GroupGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().group_get(p).await?,
            ))
        }
        "group_update" => {
            let p = params::build::<GroupUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::json_value(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().group_update(p, body).await?,
            ))
        }
        "group_delete" => {
            let p = params::build::<GroupDeleteParams>(&parts.path, q, h)?;
            state.backend().group_delete(p).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        "organisation_create" => {
            let p = params::build::<OrganisationCreateParams>(&parts.path, q, h)?;
            // TODO(port): P12 — typed XML demographic bodies.
            let body = negotiate::json_value(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                StatusCode::CREATED,
                &state.backend().organisation_create(p, body).await?,
            ))
        }
        "organisation_get" => {
            let p = params::build::<OrganisationGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().organisation_get(p).await?,
            ))
        }
        "organisation_update" => {
            let p = params::build::<OrganisationUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::json_value(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().organisation_update(p, body).await?,
            ))
        }
        "organisation_delete" => {
            let p = params::build::<OrganisationDeleteParams>(&parts.path, q, h)?;
            state.backend().organisation_delete(p).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        "person_create" => {
            let p = params::build::<PersonCreateParams>(&parts.path, q, h)?;
            // TODO(port): P12 — typed XML demographic bodies.
            let body = negotiate::json_value(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                StatusCode::CREATED,
                &state.backend().person_create(p, body).await?,
            ))
        }
        "person_get" => {
            let p = params::build::<PersonGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().person_get(p).await?,
            ))
        }
        "person_update" => {
            let p = params::build::<PersonUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::json_value(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().person_update(p, body).await?,
            ))
        }
        "person_delete" => {
            let p = params::build::<PersonDeleteParams>(&parts.path, q, h)?;
            state.backend().person_delete(p).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        "role_create" => {
            let p = params::build::<RoleCreateParams>(&parts.path, q, h)?;
            // TODO(port): P12 — typed XML demographic bodies.
            let body = negotiate::json_value(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                StatusCode::CREATED,
                &state.backend().role_create(p, body).await?,
            ))
        }
        "role_get" => {
            let p = params::build::<RoleGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().role_get(p).await?,
            ))
        }
        "role_update" => {
            let p = params::build::<RoleUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::json_value(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().role_update(p, body).await?,
            ))
        }
        "role_delete" => {
            let p = params::build::<RoleDeleteParams>(&parts.path, q, h)?;
            state.backend().role_delete(p).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        "versioned_party_get" => {
            let p = params::build::<VersionedPartyGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().versioned_party_get(p).await?,
            ))
        }
        "versioned_party_revision_history" => {
            let p = params::build::<VersionedPartyRevisionHistoryParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().versioned_party_revision_history(p).await?,
            ))
        }
        "versioned_party_version_get_at_time" => {
            let p = params::build::<VersionedPartyVersionGetAtTimeParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state
                    .backend()
                    .versioned_party_version_get_at_time(p)
                    .await?,
            ))
        }
        "versioned_party_version_get_by_id" => {
            let p = params::build::<VersionedPartyVersionGetByIdParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().versioned_party_version_get_by_id(p).await?,
            ))
        }
        "contribution_create" => {
            let p = params::build::<ContributionCreateParams>(&parts.path, q, h)?;
            // TODO(port): P12 — typed XML demographic bodies.
            let body = negotiate::json_value(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                StatusCode::CREATED,
                // `contribution_*` is defined on both DemographicApi and EhrApi
                // (shared method names); disambiguate on the trait-object backend.
                &DemographicApi::contribution_create(state.backend(), p, body).await?,
            ))
        }
        "contribution_get" => {
            let p = params::build::<ContributionGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &DemographicApi::contribution_get(state.backend(), p).await?,
            ))
        }
        "demographic_tags_get" => {
            let p = params::build::<DemographicTagsGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().demographic_tags_get(p).await?,
            ))
        }
        "agent_tags_get" => {
            let p = params::build::<AgentTagsGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().agent_tags_get(p).await?,
            ))
        }
        "agent_tags_update" => {
            let p = params::build::<AgentTagsUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::json_vec(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().agent_tags_update(p, body).await?,
            ))
        }
        "agent_tags_delete" => {
            let p = params::build::<AgentTagsDeleteParams>(&parts.path, q, h)?;
            state.backend().agent_tags_delete(p).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        "group_tags_get" => {
            let p = params::build::<GroupTagsGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().group_tags_get(p).await?,
            ))
        }
        "group_tags_update" => {
            let p = params::build::<GroupTagsUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::json_vec(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().group_tags_update(p, body).await?,
            ))
        }
        "group_tags_delete" => {
            let p = params::build::<GroupTagsDeleteParams>(&parts.path, q, h)?;
            state.backend().group_tags_delete(p).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        "organisation_tags_get" => {
            let p = params::build::<OrganisationTagsGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().organisation_tags_get(p).await?,
            ))
        }
        "organisation_tags_update" => {
            let p = params::build::<OrganisationTagsUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::json_vec(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().organisation_tags_update(p, body).await?,
            ))
        }
        "organisation_tags_delete" => {
            let p = params::build::<OrganisationTagsDeleteParams>(&parts.path, q, h)?;
            state.backend().organisation_tags_delete(p).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        "person_tags_get" => {
            let p = params::build::<PersonTagsGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().person_tags_get(p).await?,
            ))
        }
        "person_tags_update" => {
            let p = params::build::<PersonTagsUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::json_vec(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().person_tags_update(p, body).await?,
            ))
        }
        "person_tags_delete" => {
            let p = params::build::<PersonTagsDeleteParams>(&parts.path, q, h)?;
            state.backend().person_tags_delete(p).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        "role_tags_get" => {
            let p = params::build::<RoleTagsGetParams>(&parts.path, q, h)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().role_tags_get(p).await?,
            ))
        }
        "role_tags_update" => {
            let p = params::build::<RoleTagsUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::json_vec(h, &parts.body)?;
            Ok(negotiate::respond(
                h,
                ok,
                &state.backend().role_tags_update(p, body).await?,
            ))
        }
        "role_tags_delete" => {
            let p = params::build::<RoleTagsDeleteParams>(&parts.path, q, h)?;
            state.backend().role_tags_delete(p).await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        other => Err(RestError(openehr_its::rest::runtime::ApiError::Internal(
            format!("unrouted demographic operation: {other}"),
        ))),
    }
}
