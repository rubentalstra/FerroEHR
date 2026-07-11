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
//! this module adds the storage-model typing on top: the `object_id` must be
//! the UUID `vo_id` key, and the `version_tree_id` decodes into a [`TreeId`] —
//! `trunk_version [ '.' branch_number '.' branch_version ]` (RM common
//! master06 §Version tree: "To support branching, a further pair of numbers is
//! added … Both of these numbers also start at '1'"). Branch ids are
//! first-class (A1 rm-common-change-control-R7; the former trunk-only
//! rejection F-06-09 is retired).

use std::fmt;
use std::str::FromStr;

use openehr_base::base_types::identification::version_tree_id::VersionTreeId;
use openehr_base::prelude::{ObjectVersionId, Uid};
use openehr_its::rest::runtime::ApiError;
use uuid::Uuid;

use super::ServiceError;

/// A decoded `VERSION_TREE_ID`: the trunk version plus, for a branch version,
/// the `(branch_number, branch_version)` pair (both `>= 1` per BASE
/// `VERSION_TREE_ID`; RM common master06 §Version tree).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TreeId {
    /// The trunk version this id sits on (first lexical part).
    pub(super) trunk: i32,
    /// `None` for a trunk version; `Some((branch_number, branch_version))` for
    /// a branch version.
    pub(super) branch: Option<(i32, i32)>,
}

impl TreeId {
    /// A trunk version id.
    pub(super) const fn trunk(version: i32) -> Self {
        Self {
            trunk: version,
            branch: None,
        }
    }

    /// A branch version id (`trunk.branch_number.branch_version`).
    pub(super) const fn branch(trunk: i32, branch_number: i32, branch_version: i32) -> Self {
        Self {
            trunk,
            branch: Some((branch_number, branch_version)),
        }
    }

    /// Whether this is a trunk version id.
    pub(super) const fn is_trunk(self) -> bool {
        self.branch.is_none()
    }

    /// The storage triple `(trunk_version, branch_number, branch_version)` —
    /// `(t, 0, 0)` for a trunk row.
    pub(super) const fn columns(self) -> (i32, i32, i32) {
        match self.branch {
            None => (self.trunk, 0, 0),
            Some((b, v)) => (self.trunk, b, v),
        }
    }

    /// A [`TreeId`] from the storage triple; `(t, 0, 0)` is a trunk id.
    pub(super) const fn from_columns(trunk: i32, branch_number: i32, branch_version: i32) -> Self {
        if branch_number == 0 {
            Self::trunk(trunk)
        } else {
            Self::branch(trunk, branch_number, branch_version)
        }
    }

    /// Decode a BASE [`VersionTreeId`]'s lexical parts into the typed form.
    fn from_version_tree(tree: &VersionTreeId, raw: &str) -> Result<Self, VersionIdError> {
        let out_of_range = || VersionIdError::OutOfRange(raw.to_owned());
        let trunk: i32 = tree.trunk_version().parse().map_err(|_| out_of_range())?;
        if !tree.is_branch() {
            return Ok(Self::trunk(trunk));
        }
        let branch_number: i32 = tree
            .branch_number()
            .ok_or_else(out_of_range)?
            .parse()
            .map_err(|_| out_of_range())?;
        let branch_version: i32 = tree
            .branch_version()
            .ok_or_else(out_of_range)?
            .parse()
            .map_err(|_| out_of_range())?;
        Ok(Self::branch(trunk, branch_number, branch_version))
    }
}

impl fmt::Display for TreeId {
    /// The wire `version_tree_id` form: `N` or `N.B.V`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.branch {
            None => write!(f, "{}", self.trunk),
            Some((b, v)) => write!(f, "{}.{b}.{v}", self.trunk),
        }
    }
}

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
    /// A `version_tree_id` part does not fit the storage columns (`i32`).
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

impl From<VersionIdError> for ehrbase_sm::SmError {
    /// A malformed version id in an SM catalog argument is an argument-validity
    /// precondition failure (→ `400` at the wire, matching the
    /// [`From<VersionIdError> for ApiError`] `BadRequest` row).
    fn from(e: VersionIdError) -> Self {
        ehrbase_sm::SmError::precondition(e.to_string())
    }
}

/// Parse a bare `VERSION_TREE_ID` lexical value (`N` or `N.B.V`) into a
/// [`TreeId`] — the SM catalog's version argument form.
pub(super) fn parse_tree_id(raw: &str) -> Result<TreeId, VersionIdError> {
    let tree =
        VersionTreeId::from_str(raw).map_err(|_| VersionIdError::OutOfRange(raw.to_owned()))?;
    TreeId::from_version_tree(&tree, raw)
}

/// Parse a full `OBJECT_VERSION_ID` (`{object_id}::{creating_system_id}::{version_tree_id}`)
/// into the storage key pair (`vo_id`, [`TreeId`]).
pub(super) fn parse_version_uid(raw: &str) -> Result<(Uuid, TreeId), VersionIdError> {
    let (vo_id, _, tree) = parse_object_version_id(raw)?;
    Ok((vo_id, tree))
}

/// Parse a full `OBJECT_VERSION_ID` into all three components:
/// (`vo_id` = the `object_id` UUID, `creating_system_id`, [`TreeId`]).
/// The EHR-Extract import path preserves the wrapped `ORIGINAL_VERSION`'s
/// **complete** 3-part identity — object id, originating system, and version
/// tree id — so the imported version keeps its source identity (RM common
/// master06 §"Distributed Versioning": "if the version was imported,
/// `creating_system_id` will already have been set to the identifier of the
/// system of original creation").
pub(super) fn parse_object_version_id(raw: &str) -> Result<(Uuid, String, TreeId), VersionIdError> {
    let ovid = ObjectVersionId::from_str(raw).map_err(|source| VersionIdError::Malformed {
        raw: raw.to_owned(),
        source,
    })?;
    let Uid::Uuid(object_id) = ovid.object_id() else {
        return Err(VersionIdError::NotAUuid(raw.to_owned()));
    };
    let tree = TreeId::from_version_tree(&ovid.version_tree_id(), raw)?;
    // The strict `ObjectVersionId::from_str` above validated exactly three
    // non-empty `::`-delimited parts, so the middle part is present.
    let creating_system_id = raw.split("::").nth(1).unwrap_or_default().to_owned();
    Ok((object_id.value, creating_system_id, tree))
}

/// Decompose an already-parsed [`ObjectVersionId`] (the SM catalog's native
/// version-id argument) into the storage key pair (`vo_id`, [`TreeId`]) — the
/// ObjectVersionId-typed analogue of [`parse_version_uid`], used by the SM
/// service impls that receive a typed `OBJECT_VERSION_ID`.
pub(super) fn components(ovid: &ObjectVersionId) -> Result<(Uuid, TreeId), VersionIdError> {
    let raw = ovid.value.clone();
    let Uid::Uuid(object_id) = ovid.object_id() else {
        return Err(VersionIdError::NotAUuid(raw));
    };
    let tree = TreeId::from_version_tree(&ovid.version_tree_id(), &raw)?;
    Ok((object_id.value, tree))
}

/// Parse a `uid_based_id`/`versioned_object_uid` path parameter: either a bare
/// `HIER_OBJECT_ID` (a UUID, → no version) or a full `OBJECT_VERSION_ID`
/// (strict three-part, → its [`TreeId`]).
pub(super) fn parse_uid_based_id(raw: &str) -> Result<(Uuid, Option<TreeId>), VersionIdError> {
    if raw.contains("::") {
        let (vo_id, tree) = parse_version_uid(raw)?;
        Ok((vo_id, Some(tree)))
    } else {
        let vo_id = Uuid::parse_str(raw).map_err(|_| VersionIdError::NotAUuid(raw.to_owned()))?;
        Ok((vo_id, None))
    }
}

/// The expected (current) version from an `If-Match` header value: a quoted or
/// bare `OBJECT_VERSION_ID` (strict BASE parse — the `object_id` need not be a
/// UUID for precondition purposes), or a bare trunk integer. `None` when no
/// precondition can be extracted (none is then enforced).
pub(super) fn expected_from_if_match(if_match: &str) -> Option<TreeId> {
    let token = if_match.trim().trim_matches('"');
    if let Ok(ovid) = ObjectVersionId::from_str(token) {
        return TreeId::from_version_tree(&ovid.version_tree_id(), token).ok();
    }
    token.parse().ok().map(TreeId::trunk)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    const VO: &str = "018f4a5e-9df1-7d1e-8b6f-2b8c00000001";

    #[test]
    fn version_uid_strict_three_part() {
        let raw = format!("{VO}::ehrbase-rs.local::3");
        let (vo_id, tree) = parse_version_uid(&raw).unwrap();
        assert_eq!(vo_id, Uuid::parse_str(VO).unwrap());
        assert_eq!(tree, TreeId::trunk(3));

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
    fn object_version_id_preserves_all_three_parts() {
        // The import path keeps the source's full 3-part identity.
        let raw = format!("{VO}::sourceSystem.example.org::4");
        let (vo_id, csid, tree) = parse_object_version_id(&raw).unwrap();
        assert_eq!(vo_id, Uuid::parse_str(VO).unwrap());
        assert_eq!(csid, "sourceSystem.example.org");
        assert_eq!(tree, TreeId::trunk(4));

        // Non-UUID object id.
        assert!(matches!(
            parse_object_version_id("not-a-uuid::sys::1"),
            Err(VersionIdError::NotAUuid(_))
        ));
    }

    /// Branch `version_tree_id`s decode into their `(trunk, branch, version)`
    /// triple and round-trip through the wire form (RM common master06 §Version
    /// tree; A1 rm-common-change-control-R7 — the former trunk-only rejection
    /// is retired).
    #[test]
    fn branch_ids_are_first_class() {
        let (vo_id, tree) = parse_version_uid(&format!("{VO}::sys::2.1.4")).unwrap();
        assert_eq!(vo_id, Uuid::parse_str(VO).unwrap());
        assert_eq!(tree, TreeId::branch(2, 1, 4));
        assert_eq!(tree.columns(), (2, 1, 4));
        assert_eq!(tree.to_string(), "2.1.4");
        assert_eq!(TreeId::from_columns(2, 1, 4), tree);
        assert_eq!(TreeId::from_columns(3, 0, 0), TreeId::trunk(3));
        assert_eq!(TreeId::trunk(3).to_string(), "3");

        // A malformed branch tail (two parts) is not a valid VERSION_TREE_ID.
        assert!(parse_version_uid(&format!("{VO}::sys::2.1")).is_err());
    }

    #[test]
    fn uid_based_id_accepts_bare_hier_object_id() {
        let (vo_id, version) = parse_uid_based_id(VO).unwrap();
        assert_eq!(vo_id, Uuid::parse_str(VO).unwrap());
        assert_eq!(version, None);

        let (vo_id, version) = parse_uid_based_id(&format!("{VO}::sys::2")).unwrap();
        assert_eq!(vo_id, Uuid::parse_str(VO).unwrap());
        assert_eq!(version, Some(TreeId::trunk(2)));

        // A `::`-carrying id must be a *valid* OBJECT_VERSION_ID — the old
        // splitter silently returned (uuid, None) for a malformed tail.
        assert!(parse_uid_based_id(&format!("{VO}::sys")).is_err());
        assert!(parse_uid_based_id("garbage").is_err());
    }

    #[test]
    fn if_match_extraction() {
        // Full OBJECT_VERSION_ID (BASE-strict; object_id may be any UID form).
        assert_eq!(
            expected_from_if_match("\"abc::sys::3\""),
            Some(TreeId::trunk(3))
        );
        assert_eq!(
            expected_from_if_match("abc::sys::3"),
            Some(TreeId::trunk(3))
        );
        // A branch precondition is honoured, not dropped.
        assert_eq!(
            expected_from_if_match("abc::sys::2.1.1"),
            Some(TreeId::branch(2, 1, 1))
        );
        // Bare integer.
        assert_eq!(expected_from_if_match("2"), Some(TreeId::trunk(2)));
        // Unparseable → no precondition.
        assert_eq!(expected_from_if_match("garbage"), None);
        // Malformed OVID shapes do not leak a version out of the wrong slot.
        assert_eq!(expected_from_if_match("a::b::c::3"), None);
        assert_eq!(expected_from_if_match("abc::3"), None);
    }
}
