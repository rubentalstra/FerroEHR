//! `OBJECT_VERSION_ID` / `VERSION_TREE_ID` decoding — the identification law of
//! the versioning core (S-01..S-06).
//!
//! Spec: `docs/specs/openehr/BASE/docs/base_types/master05-identification_package.adoc`
//! §"Identifying Versions" + §Syntaxes: an `OBJECT_VERSION_ID` is exactly three
//! `::`-delimited parts, `object_id '::' creating_system_id '::' version_tree_id`,
//! and a `VERSION_TREE_ID` is `trunk_version [ '.' branch_number '.'
//! branch_version ]` with every part `>= 1`
//! (`VERSION_TREE_ID.Trunk_version_valid` / `.Branch_validity`; RM common
//! `master06-change_control_package.adoc` §Version tree). The strict three-part
//! parse lives in `openehr-base` (`ObjectVersionId::from_str`); this module adds
//! the storage-model typing on top: the `object_id` must be the UUID `vo_id`
//! key, and the `version_tree_id` decodes into a [`TreeId`].
//!
//! One strict decoder replaces the several hand-rolled `::` splitters the
//! service used to carry. Branch ids are first-class (RM common master06
//! §Version tree: "To support branching, a further pair of numbers is added …
//! Both of these numbers also start at '1'").

use std::fmt;
use std::str::FromStr;

use openehr_base::base_types::identification::version_tree_id::VersionTreeId;
use openehr_base::prelude::{ObjectVersionId, Uid};
use openehr_its::rest::runtime::ApiError;
use uuid::Uuid;

use crate::service::ServiceError;

/// A decoded `VERSION_TREE_ID`: the trunk version plus, for a branch version,
/// the `(branch_number, branch_version)` pair (both `>= 1` per BASE
/// `VERSION_TREE_ID`; RM common master06 §Version tree).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TreeId {
    /// The trunk version this id sits on (first lexical part).
    pub(crate) trunk: i32,
    /// `None` for a trunk version; `Some((branch_number, branch_version))` for a
    /// branch version.
    pub(crate) branch: Option<(i32, i32)>,
}

impl TreeId {
    /// A trunk version id.
    pub(crate) const fn trunk(version: i32) -> Self {
        Self {
            trunk: version,
            branch: None,
        }
    }

    /// A branch version id (`trunk.branch_number.branch_version`).
    pub(crate) const fn branch(trunk: i32, branch_number: i32, branch_version: i32) -> Self {
        Self {
            trunk,
            branch: Some((branch_number, branch_version)),
        }
    }

    /// Whether this is a trunk version id.
    pub(crate) const fn is_trunk(self) -> bool {
        self.branch.is_none()
    }

    /// The storage triple `(trunk_version, branch_number, branch_version)` —
    /// `(t, 0, 0)` for a trunk row.
    pub(crate) const fn columns(self) -> (i32, i32, i32) {
        match self.branch {
            None => (self.trunk, 0, 0),
            Some((b, v)) => (self.trunk, b, v),
        }
    }

    /// A [`TreeId`] from the storage triple; `(t, 0, 0)` is a trunk id.
    pub(crate) const fn from_columns(trunk: i32, branch_number: i32, branch_version: i32) -> Self {
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

/// Format an `OBJECT_VERSION_ID` wire value from its three parts (`object_id ::
/// creating_system_id :: version_tree_id`; BASE master05 §Syntaxes). The single
/// place the versioning builders synthesize a version id, so its shape is
/// consistent with what [`parse_object_version_id`] accepts.
pub(crate) fn object_version_id(vo_id: Uuid, creating_system_id: &str, tree: TreeId) -> String {
    format!("{vo_id}::{creating_system_id}::{tree}")
}

/// Composite-identifier equality (G-09): case-insensitive comparison of a
/// `creating_system_id` (BASE `base_types` master05 §"Composite Identifiers and
/// Case": composite identifiers are "case-preserving" **and** "case-insensitive
/// — two identifiers identical apart from case … identify the same thing").
///
/// The stored value is preserved verbatim (case-preserving); this is the one
/// place versioning decides whether two `creating_system_id`s denote the same
/// originating system — used by the tree-placement decision to tell "continue
/// my own lineage" from "fork a branch off a copy made elsewhere". The
/// characters are basic-latin (master05 §Character Set), so ASCII case-folding
/// is the correct fold; the Turkish `I/i` caveat (master05 §Composite
/// Identifiers and Case) does not apply to an ASCII system id.
///
/// PORT NOTE (G-09, master05 §Composite Identifiers and Case): storage keeps
/// `creating_system_id` verbatim, and the DB uniqueness that also needs the
/// case-fold is a storage-boundary concern cross-checked in
/// `docs/spec-audit/rm-common-change-control`; versioning enforces the
/// case-insensitive *equality* here.
pub(crate) fn eq_composite_id(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Why a version-id string was rejected. Converts into [`ApiError`] (`400`,
/// path/header parameters), [`ServiceError`] (`422`, payload fields) and
/// [`ehrbase_sm::SmError`] (`400`, SM catalog arguments) at each call site's
/// natural severity.
#[derive(Debug, thiserror::Error)]
pub(crate) enum VersionIdError {
    /// Not a well-formed BASE `OBJECT_VERSION_ID` (wrong part count, empty
    /// component, malformed `version_tree_id`).
    #[error("malformed OBJECT_VERSION_ID {raw:?}: {source}")]
    Malformed {
        raw: String,
        source: openehr_base::base_types::identification::lexical::IdError,
    },
    /// The `object_id` part is not a UUID (this CDR keys versioned objects by
    /// UUID `vo_id` — S-03).
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
pub(crate) fn parse_tree_id(raw: &str) -> Result<TreeId, VersionIdError> {
    let tree =
        VersionTreeId::from_str(raw).map_err(|_| VersionIdError::OutOfRange(raw.to_owned()))?;
    TreeId::from_version_tree(&tree, raw)
}

/// Parse a full `OBJECT_VERSION_ID` (`{object_id}::{creating_system_id}::{version_tree_id}`)
/// into the storage key pair (`vo_id`, [`TreeId`]).
pub(crate) fn parse_version_uid(raw: &str) -> Result<(Uuid, TreeId), VersionIdError> {
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
pub(crate) fn parse_object_version_id(raw: &str) -> Result<(Uuid, String, TreeId), VersionIdError> {
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
/// version-id argument) into the storage key pair (`vo_id`, [`TreeId`]).
pub(crate) fn components(ovid: &ObjectVersionId) -> Result<(Uuid, TreeId), VersionIdError> {
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
pub(crate) fn parse_uid_based_id(raw: &str) -> Result<(Uuid, Option<TreeId>), VersionIdError> {
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
pub(crate) fn expected_from_if_match(if_match: &str) -> Option<TreeId> {
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
        // Round-trips through the wire formatter.
        assert_eq!(object_version_id(vo_id, &csid, tree), raw);

        assert!(matches!(
            parse_object_version_id("not-a-uuid::sys::1"),
            Err(VersionIdError::NotAUuid(_))
        ));
    }

    /// Branch `version_tree_id`s decode into their `(trunk, branch, version)`
    /// triple and round-trip through the wire form (RM common master06 §Version
    /// tree — branch ids are first-class).
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

        // A `::`-carrying id must be a *valid* OBJECT_VERSION_ID.
        assert!(parse_uid_based_id(&format!("{VO}::sys")).is_err());
        assert!(parse_uid_based_id("garbage").is_err());
    }

    #[test]
    fn if_match_extraction() {
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

    /// G-09: composite-identifier equality is case-insensitive
    /// (BASE `base_types` master05 §"Composite Identifiers and Case").
    #[test]
    fn composite_identifier_equality_is_case_insensitive() {
        assert!(eq_composite_id("EHRBase-RS.local", "ehrbase-rs.local"));
        assert!(eq_composite_id("sys", "SYS"));
        assert!(!eq_composite_id("system.a", "system.b"));
    }
}
