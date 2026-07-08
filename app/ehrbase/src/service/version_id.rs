//! `OBJECT_VERSION_ID` decoding at the service edge — **one** strict decoder
//! over the BASE identification value types
//! ([`openehr_base::prelude::ObjectVersionId`]), replacing the five divergent
//! hand-rolled `::` splitters this crate used to carry (finding F-13-01/W2-B:
//! `parse_object_id`, `parse_version_uid`, `expected_from_if_match`,
//! `parse_expected_version`, `parse_preceding` — all deleted).
//!
//! Spec: BASE 1.3.0
//! `docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.base_types.object_version_id.adoc`
//! — the lexical form is exactly three `::`-delimited parts,
//! `object_id '::' creating_system_id '::' version_tree_id`. The strict
//! three-part parse lives in `openehr-base` (`ObjectVersionId::from_str`);
//! this module only adds the storage-model typing on top: the `object_id`
//! must be the UUID `vo_id` key, and the `version_tree_id` must be a trunk
//! version (`i32`).
//!
//! PORT NOTE (F-06-09): branch `version_tree_id`s (`N.N.N`) are out of
//! Stage-1 scope — `vo_version.sys_version` is a plain trunk integer — so a
//! well-formed branch id is rejected with an explicit error, not mis-split.

use std::str::FromStr;

use openehr_base::prelude::{ObjectVersionId, Uid};
use openehr_its::rest::runtime::ApiError;
use uuid::Uuid;

use super::ServiceError;

/// Why a version-id string was rejected. Converts into [`ApiError`] (`400`,
/// path/header parameters) and [`ServiceError`] (`422`, payload fields) at the
/// call sites' natural severities.
#[derive(Debug, thiserror::Error)]
pub(super) enum VersionIdError {
    /// Not a well-formed BASE `OBJECT_VERSION_ID` (wrong part count, empty
    /// component, malformed `version_tree_id`).
    #[error("malformed OBJECT_VERSION_ID {raw:?}: {source}")]
    Malformed {
        raw: String,
        source: openehr_base::base_types::identification::lexical::IdError,
    },
    /// The `object_id` part is not a UUID (this CDR keys versioned objects by
    /// UUID `vo_id`).
    #[error("OBJECT_VERSION_ID object_id is not a UUID: {0:?}")]
    NotAUuid(String),
    /// A branch `version_tree_id` — well-formed per BASE, unsupported here
    /// (trunk-only Stage 1; PORT NOTE F-06-09).
    #[error("branch version ids are not supported (trunk-only): {0:?}")]
    Branch(String),
    /// The trunk version does not fit the storage `sys_version` (`i32`).
    #[error("version_tree_id out of range: {0:?}")]
    OutOfRange(String),
}

impl From<VersionIdError> for ApiError {
    fn from(e: VersionIdError) -> Self {
        ApiError::BadRequest(e.to_string())
    }
}

impl From<VersionIdError> for ServiceError {
    fn from(e: VersionIdError) -> Self {
        ServiceError::Unprocessable(e.to_string())
    }
}

/// Parse a full `OBJECT_VERSION_ID` (`{object_id}::{creating_system_id}::{version_tree_id}`)
/// into the storage key pair (`vo_id`, trunk version).
pub(super) fn parse_version_uid(raw: &str) -> Result<(Uuid, i32), VersionIdError> {
    let ovid = ObjectVersionId::from_str(raw).map_err(|source| VersionIdError::Malformed {
        raw: raw.to_owned(),
        source,
    })?;
    let Uid::Uuid(object_id) = ovid.object_id() else {
        return Err(VersionIdError::NotAUuid(raw.to_owned()));
    };
    let tree = ovid.version_tree_id();
    if tree.is_branch() {
        return Err(VersionIdError::Branch(raw.to_owned()));
    }
    let version: i32 = tree
        .trunk_version()
        .parse()
        .map_err(|_| VersionIdError::OutOfRange(raw.to_owned()))?;
    Ok((object_id.value, version))
}

/// Parse a `uid_based_id`/`versioned_object_uid` path parameter: either a bare
/// `HIER_OBJECT_ID` (a UUID, → no version) or a full `OBJECT_VERSION_ID`
/// (strict three-part, → its trunk version).
pub(super) fn parse_uid_based_id(raw: &str) -> Result<(Uuid, Option<i32>), VersionIdError> {
    if raw.contains("::") {
        let (vo_id, version) = parse_version_uid(raw)?;
        Ok((vo_id, Some(version)))
    } else {
        let vo_id = Uuid::parse_str(raw).map_err(|_| VersionIdError::NotAUuid(raw.to_owned()))?;
        Ok((vo_id, None))
    }
}

/// The expected (current) trunk version from an `If-Match` header value: a
/// quoted or bare `OBJECT_VERSION_ID` (strict BASE parse — the `object_id`
/// need not be a UUID for precondition purposes), or a bare integer. `None`
/// when no precondition can be extracted (none is then enforced).
pub(super) fn expected_from_if_match(if_match: &str) -> Option<i32> {
    let token = if_match.trim().trim_matches('"');
    if let Ok(ovid) = ObjectVersionId::from_str(token) {
        let tree = ovid.version_tree_id();
        if tree.is_branch() {
            return None;
        }
        return tree.trunk_version().parse().ok();
    }
    token.parse().ok()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    const VO: &str = "018f4a5e-9df1-7d1e-8b6f-2b8c00000001";

    #[test]
    fn version_uid_strict_three_part() {
        let raw = format!("{VO}::ehrbase-rs.local::3");
        let (vo_id, version) = parse_version_uid(&raw).unwrap();
        assert_eq!(vo_id, Uuid::parse_str(VO).unwrap());
        assert_eq!(version, 3);

        // Two parts (the old `rsplit`/`nth(1)` splitters disagreed here).
        assert!(matches!(
            parse_version_uid(&format!("{VO}::2")),
            Err(VersionIdError::Malformed { .. })
        ));
        // Four parts is not a valid lexical form either.
        assert!(matches!(
            parse_version_uid(&format!("{VO}::a::b::2")),
            Err(VersionIdError::Malformed { .. })
        ));
        // Non-integer version tail.
        assert!(matches!(
            parse_version_uid(&format!("{VO}::sys::latest")),
            Err(VersionIdError::Malformed { .. })
        ));
        // Non-UUID object id.
        assert!(matches!(
            parse_version_uid("not-a-uuid::sys::1"),
            Err(VersionIdError::NotAUuid(_))
        ));
    }

    #[test]
    fn branch_ids_are_rejected_explicitly() {
        // PORT NOTE F-06-09: well-formed branch id, trunk-only storage.
        assert!(matches!(
            parse_version_uid(&format!("{VO}::sys::2.1.4")),
            Err(VersionIdError::Branch(_))
        ));
    }

    #[test]
    fn uid_based_id_accepts_bare_hier_object_id() {
        let (vo_id, version) = parse_uid_based_id(VO).unwrap();
        assert_eq!(vo_id, Uuid::parse_str(VO).unwrap());
        assert_eq!(version, None);

        let (vo_id, version) = parse_uid_based_id(&format!("{VO}::sys::2")).unwrap();
        assert_eq!(vo_id, Uuid::parse_str(VO).unwrap());
        assert_eq!(version, Some(2));

        // A `::`-carrying id must be a *valid* OBJECT_VERSION_ID — the old
        // splitter silently returned (uuid, None) for a malformed tail.
        assert!(parse_uid_based_id(&format!("{VO}::sys")).is_err());
        assert!(parse_uid_based_id("garbage").is_err());
    }

    #[test]
    fn if_match_extraction() {
        // Full OBJECT_VERSION_ID (BASE-strict; object_id may be any UID form).
        assert_eq!(expected_from_if_match("\"abc::sys::3\""), Some(3));
        assert_eq!(expected_from_if_match("abc::sys::3"), Some(3));
        // Bare integer.
        assert_eq!(expected_from_if_match("2"), Some(2));
        // Unparseable → no precondition.
        assert_eq!(expected_from_if_match("garbage"), None);
        // Malformed OVID shapes do not leak a version out of the wrong slot.
        assert_eq!(expected_from_if_match("a::b::c::3"), None);
        assert_eq!(expected_from_if_match("abc::3"), None);
    }
}
