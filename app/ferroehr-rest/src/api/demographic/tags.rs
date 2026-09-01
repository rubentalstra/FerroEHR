// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The `ITEM_TAG` sub-resources — `operations/person_tags_get.yaml`,
//! `person_tags_update.yaml`, `person_tags_delete.yaml` (and the field-identical
//! `agent_*`/`group_*`/`organisation_*`/`role_*`), plus the kind-agnostic
//! collection filter `demographic_tags_get.yaml`. Tags use the **canonical**
//! content negotiation (`Accept_canonical`/`ContentType_canonical`).
//!
//! **Each kind decodes through its OWN generated params type.** The five
//! families' parameter sets are field-identical in Release-1.1.0, so one type
//! would decode all five today — but that is a property of the current release,
//! not a contract. Routing every kind through `Agent*Params` would mean a
//! future release that adds a parameter to (say) `person_tags_get` alone would
//! be silently mis-decoded on four other families, with no compile error. The
//! per-kind match below makes any such divergence a build failure instead.

use axum::response::Response;

use openehr_its::rest::generated::demographic::{
    AgentTagsDeleteParams, AgentTagsGetParams, AgentTagsUpdateParams, DemographicTagsGetParams,
    GroupTagsDeleteParams, GroupTagsGetParams, GroupTagsUpdateParams, OrganisationTagsDeleteParams,
    OrganisationTagsGetParams, OrganisationTagsUpdateParams, PersonTagsDeleteParams,
    PersonTagsGetParams, PersonTagsUpdateParams, RoleTagsDeleteParams, RoleTagsGetParams,
    RoleTagsUpdateParams,
};
use openehr_its::rest::runtime::ApiError;

use crate::api::RequestParts;
use crate::api::item_tags;
use crate::overview::error::RestError;
use crate::state::AppState;
use crate::{negotiate, params};
use ferroehr::service::demographic::types::PartyKind;
use http::StatusCode;

/// The per-kind `ITEM_TAG` operations (`tags_get`/`tags_update`/`tags_delete`).
pub(super) async fn run(
    state: AppState,
    kind: PartyKind,
    action: &str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let seg = kind.segment();

    match action {
        "tags_get" => {
            // Each kind's own generated params type (see the module doc).
            let uid_based_id = match kind {
                PartyKind::Agent => {
                    params::build::<AgentTagsGetParams>(&parts.path, q, h)?.uid_based_id
                }
                PartyKind::Group => {
                    params::build::<GroupTagsGetParams>(&parts.path, q, h)?.uid_based_id
                }
                PartyKind::Organisation => {
                    params::build::<OrganisationTagsGetParams>(&parts.path, q, h)?.uid_based_id
                }
                PartyKind::Person => {
                    params::build::<PersonTagsGetParams>(&parts.path, q, h)?.uid_based_id
                }
                PartyKind::Role => {
                    params::build::<RoleTagsGetParams>(&parts.path, q, h)?.uid_based_id
                }
            };
            let tags = state.backend().party_tags_get(kind, uid_based_id).await?;
            Ok(negotiate::respond(
                h,
                StatusCode::OK,
                &openehr_its::json::to_canonical_value(&tags),
            ))
        }
        "tags_update" => {
            let uid_based_id = match kind {
                PartyKind::Agent => {
                    params::build::<AgentTagsUpdateParams>(&parts.path, q, h)?.uid_based_id
                }
                PartyKind::Group => {
                    params::build::<GroupTagsUpdateParams>(&parts.path, q, h)?.uid_based_id
                }
                PartyKind::Organisation => {
                    params::build::<OrganisationTagsUpdateParams>(&parts.path, q, h)?.uid_based_id
                }
                PartyKind::Person => {
                    params::build::<PersonTagsUpdateParams>(&parts.path, q, h)?.uid_based_id
                }
                PartyKind::Role => {
                    params::build::<RoleTagsUpdateParams>(&parts.path, q, h)?.uid_based_id
                }
            };
            let body = item_tags::write_body(h, &parts.body)?;
            let tags = state
                .backend()
                .party_tags_update(kind, uid_based_id, body)
                .await?;
            // person_tags_update.yaml — 200 (200_PERSON_ItemTagList_updated)
            // with the tag list on `Prefer: return=representation`; 204
            // (204_updated) when `Prefer` is missing or `return=minimal`,
            // with `Preference-Applied` declaring which.
            Ok(negotiate::write_collection(
                h,
                StatusCode::NO_CONTENT,
                StatusCode::OK,
                &openehr_its::json::to_canonical_value(&tags),
            ))
        }
        "tags_delete" => {
            let (uid_based_id, key) = match kind {
                PartyKind::Agent => {
                    let p = params::build::<AgentTagsDeleteParams>(&parts.path, q, h)?;
                    (p.uid_based_id, p.key)
                }
                PartyKind::Group => {
                    let p = params::build::<GroupTagsDeleteParams>(&parts.path, q, h)?;
                    (p.uid_based_id, p.key)
                }
                PartyKind::Organisation => {
                    let p = params::build::<OrganisationTagsDeleteParams>(&parts.path, q, h)?;
                    (p.uid_based_id, p.key)
                }
                PartyKind::Person => {
                    let p = params::build::<PersonTagsDeleteParams>(&parts.path, q, h)?;
                    (p.uid_based_id, p.key)
                }
                PartyKind::Role => {
                    let p = params::build::<RoleTagsDeleteParams>(&parts.path, q, h)?;
                    (p.uid_based_id, p.key)
                }
            };
            state
                .backend()
                .party_tags_delete(kind, uid_based_id, key)
                .await?;
            Ok(negotiate::empty(StatusCode::NO_CONTENT))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted demographic tag operation: {seg}_{other}"
        )))),
    }
}

/// `GET /demographic/tags` — the kind-agnostic `ITEM_TAG` collection filter
/// (`demographic_tags_get.yaml`).
pub(super) async fn run_collection(
    state: AppState,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let p = params::build::<DemographicTagsGetParams>(&parts.path, parts.query.as_deref(), h)?;
    let tags = state
        .backend()
        .demographic_tags_get(p.tag_key, p.tag_value, p.tag_target_path)
        .await?;
    Ok(negotiate::respond(
        h,
        StatusCode::OK,
        &openehr_its::json::to_canonical_value(&tags),
    ))
}
