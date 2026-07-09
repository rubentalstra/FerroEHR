//! Wire-string decoding for the EHR-core routes: path/header identifiers into
//! the SM catalog's native argument types (`uuid::Uuid`,
//! [`ObjectVersionId`](openehr_base::prelude::ObjectVersionId)).
//!
//! The SM native API is protocol-free (ADR-011): the catalog takes RM/BASE
//! identifier types, not raw wire strings. Turning a path parameter such as
//! `{ehr_id}` or `{version_uid}` into those types is the protocol adapter's
//! decode job — it lives here, at the ITS-REST edge, and every malformed value
//! surfaces as `400 Bad Request` ([`ApiError::BadRequest`]).
//!
//! Spec: BASE 1.3.0 `object_version_id.adoc` — an `OBJECT_VERSION_ID` is exactly
//! three `::`-delimited parts (`object_id '::' creating_system_id '::'
//! version_tree_id`); the strict parse is `ObjectVersionId::from_str`. A
//! `uid_based_id`/`versioned_object_uid` path segment is either a bare
//! `HIER_OBJECT_ID` (a UUID) or a full `OBJECT_VERSION_ID`.

use std::str::FromStr;

use openehr_base::prelude::ObjectVersionId;
use openehr_its::rest::runtime::ApiError;
use uuid::Uuid;

/// Parse an `{ehr_id}` path parameter into its UUID.
///
/// # Errors
/// [`ApiError::BadRequest`] if `raw` is not a UUID.
pub(crate) fn parse_ehr_id(raw: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| ApiError::BadRequest(format!("invalid EHR id: {raw}")))
}

/// Parse a UUID path parameter (a bare `HIER_OBJECT_ID`: a versioned-object or
/// contribution uid).
///
/// # Errors
/// [`ApiError::BadRequest`] if `raw` is not a UUID.
pub(crate) fn parse_uuid(raw: &str, what: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| ApiError::BadRequest(format!("invalid {what}: {raw}")))
}

/// Parse a full `{version_uid}` path parameter into a BASE
/// [`ObjectVersionId`](openehr_base::prelude::ObjectVersionId).
///
/// # Errors
/// [`ApiError::BadRequest`] if `raw` is not a well-formed `OBJECT_VERSION_ID`.
pub(crate) fn parse_version_uid(raw: &str) -> Result<ObjectVersionId, ApiError> {
    ObjectVersionId::from_str(raw)
        .map_err(|e| ApiError::BadRequest(format!("malformed OBJECT_VERSION_ID {raw:?}: {e}")))
}

/// A decoded `uid_based_id`/`versioned_object_uid` path segment: the
/// versioned-object UUID, plus the full `OBJECT_VERSION_ID` when the segment
/// carried a version (`{object_id}::{system}::{version}`).
pub(crate) struct UidBasedId {
    /// The versioned-object UUID (`object_id`).
    pub(crate) vo_id: Uuid,
    /// The full version id, when the segment named a specific version.
    pub(crate) version: Option<ObjectVersionId>,
}

/// Parse a `uid_based_id`/`versioned_object_uid` path segment: a bare
/// `HIER_OBJECT_ID` (UUID → no version) or a full `OBJECT_VERSION_ID`.
///
/// # Errors
/// [`ApiError::BadRequest`] if a `::`-carrying value is not a valid
/// `OBJECT_VERSION_ID`, or a bare value is not a UUID or its `object_id` is not.
pub(crate) fn parse_uid_based_id(raw: &str) -> Result<UidBasedId, ApiError> {
    if raw.contains("::") {
        let ovid = parse_version_uid(raw)?;
        let vo_id = object_id_uuid(&ovid).ok_or_else(|| {
            ApiError::BadRequest(format!(
                "OBJECT_VERSION_ID object_id is not a UUID: {raw:?}"
            ))
        })?;
        Ok(UidBasedId {
            vo_id,
            version: Some(ovid),
        })
    } else {
        Ok(UidBasedId {
            vo_id: parse_uuid(raw, "versioned_object_uid")?,
            version: None,
        })
    }
}

/// The `object_id` of an `OBJECT_VERSION_ID` as a UUID (this CDR keys versioned
/// objects by UUID), or `None` if the `object_id` is some other `UID` form.
pub(crate) fn object_id_uuid(ovid: &ObjectVersionId) -> Option<Uuid> {
    use openehr_base::prelude::Uid;
    match ovid.object_id() {
        Uid::Uuid(u) => Some(u.value),
        _ => None,
    }
}

/// The `preceding_version_uid` (`If-Match`) as an [`ObjectVersionId`], if the
/// header value is a well-formed (quoted or bare) `OBJECT_VERSION_ID`. `None`
/// when no precondition can be extracted (none is then enforced).
pub(crate) fn if_match_ovid(if_match: &str) -> Option<ObjectVersionId> {
    let token = if_match.trim().trim_matches('"');
    if token.is_empty() {
        return None;
    }
    ObjectVersionId::from_str(token).ok()
}
