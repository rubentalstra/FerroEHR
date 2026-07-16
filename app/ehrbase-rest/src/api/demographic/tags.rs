//! The `ITEM_TAG` sub-resources — `operations/person_tags_get.yaml`,
//! `person_tags_update.yaml`, `person_tags_delete.yaml` (and the field-identical
//! `agent_*`/`group_*`/`organisation_*`/`role_*`), plus the kind-agnostic
//! collection filter `demographic_tags_get.yaml`. Tags use the **canonical**
//! content negotiation (`Accept_canonical`/`ContentType_canonical`).

use axum::response::Response;

use openehr_its::rest::generated::demographic::{
    AgentTagsDeleteParams, AgentTagsGetParams, AgentTagsUpdateParams, DemographicTagsGetParams,
};
use openehr_its::rest::runtime::ApiError;

use crate::api::RequestParts;
use crate::overview::error::RestError;
use crate::state::AppState;
use crate::{negotiate, params};
use ehrbase::service::demographic::types::PartyKind;
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
            let p = params::build::<AgentTagsGetParams>(&parts.path, q, h)?;
            let resp = state.backend().party_tags_get(kind, p.uid_based_id).await?;
            Ok(negotiate::respond(h, StatusCode::OK, &resp.body))
        }
        "tags_update" => {
            let p = params::build::<AgentTagsUpdateParams>(&parts.path, q, h)?;
            let body = negotiate::json_vec(h, &parts.body)?;
            let resp = state
                .backend()
                .party_tags_update(kind, p.uid_based_id, body)
                .await?;
            // person_tags_update.yaml — 200 (200_PERSON_ItemTagList_updated)
            // with the tag list on `Prefer: return=representation`; 204
            // (204_updated) when `Prefer` is missing or `return=minimal`.
            if negotiate::prefers_representation(h) {
                Ok(negotiate::respond(h, StatusCode::OK, &resp.body))
            } else {
                Ok(negotiate::empty(StatusCode::NO_CONTENT))
            }
        }
        "tags_delete" => {
            let p = params::build::<AgentTagsDeleteParams>(&parts.path, q, h)?;
            state
                .backend()
                .party_tags_delete(kind, p.uid_based_id, p.key)
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
    let resp = state
        .backend()
        .demographic_tags_get(p.tag_key, p.tag_value, p.tag_target_path)
        .await?;
    Ok(negotiate::respond(h, StatusCode::OK, &resp.body))
}
