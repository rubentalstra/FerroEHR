//! The `VERSIONED_PARTY` reads — `operations/versioned_party_get.yaml`,
//! `versioned_party_revision_history.yaml`,
//! `versioned_party_version_get_at_time.yaml`,
//! `versioned_party_version_get_by_id.yaml`. Canonical content negotiation
//! (`Accept_canonical`/`ContentType_canonical`).

use axum::response::Response;
use http::StatusCode;

use openehr_its::rest::generated::demographic::{
    VersionedPartyGetParams, VersionedPartyVersionGetAtTimeParams,
    VersionedPartyVersionGetByIdParams,
};
use openehr_its::rest::runtime::ApiError;

use crate::api::RequestParts;
use crate::overview::error::RestError;
use crate::state::AppState;
use crate::{negotiate, params};
use ehrbase_sm::Platform;

/// The `versioned_party_*` reads.
pub(super) async fn run<S: Platform>(
    state: AppState<S>,
    op: &'static str,
    parts: RequestParts,
) -> Result<Response, RestError> {
    let h = &parts.headers;
    let q = parts.query.as_deref();
    let ok = StatusCode::OK;
    let base = state.config().base_path.clone();

    match op {
        "versioned_party_get" => {
            let p = params::build::<VersionedPartyGetParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .versioned_party_get(p.versioned_object_uid)
                .await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        "versioned_party_revision_history" => {
            let p = params::build::<VersionedPartyGetParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .versioned_party_revision_history(p.versioned_object_uid)
                .await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        "versioned_party_version_get_at_time" => {
            let p = params::build::<VersionedPartyVersionGetAtTimeParams>(&parts.path, q, h)?;
            // 200_VERSION_at_time analogue: ETag(version_uid) + Location of the
            // VERSION resource (…/versioned_party/{uid}/version/{version_uid}).
            let segment = format!("versioned_party/{}/version", p.versioned_object_uid);
            let resp = state
                .backend()
                .versioned_party_version_get_at_time(p.versioned_object_uid, p.version_at_time)
                .await?;
            let mut out = negotiate::respond(h, ok, &resp.body);
            super::set_headers(&mut out, &base, &segment, resp.meta.as_ref());
            Ok(out)
        }
        "versioned_party_version_get_by_id" => {
            let p = params::build::<VersionedPartyVersionGetByIdParams>(&parts.path, q, h)?;
            let resp = state
                .backend()
                .versioned_party_version_get_by_id(p.versioned_object_uid, p.version_uid)
                .await?;
            Ok(negotiate::respond(h, ok, &resp.body))
        }
        other => Err(RestError(ApiError::Internal(format!(
            "unrouted versioned_party operation: {other}"
        )))),
    }
}
