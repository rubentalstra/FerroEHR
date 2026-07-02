//! `VERSION_TREE_ID` — version tree identifier for one version.
//!
//! openEHR class: `VERSION_TREE_ID`, package
//! `base.base_types.identification`.
//!
//! Version tree identifier for one version. Lexical form:
//! `trunk_version [ '.' branch_number '.' branch_version ]`.
//!
//! PORT NOTE: `VERSION_TREE_ID` is a standalone value type in the spec —
//! it does not inherit `UID`, `OBJECT_ID`, or `UID_BASED_ID`, and is used
//! as the third component of `OBJECT_VERSION_ID`'s lexical form rather than
//! being an `OBJECT_ID` itself. It is transcribed here as its own struct
//! with no embedding relationship to the `UID`/`OBJECT_ID` clusters.

/// Canonical `_type` discriminator string for this class in serialized
/// form. `VERSION_TREE_ID` is not itself wrapped by any tagged enum in this
/// package (it is a plain component of `OBJECT_VERSION_ID`'s lexical form,
/// not an `OBJECT_ID`/`UID`-hierarchy member — see the type-level PORT NOTE
/// above), so unlike `HierObjectId`/`ObjectVersionId` the struct-level
/// `#[serde(rename = "VERSION_TREE_ID")]` below is the only `_type`
/// mechanism available to it, and (per the same struct-level-rename caveat
/// noted on `hier_object_id::TYPE_NAME`) it is inert for a standalone
/// struct under `#[derive(Serialize)]` — no `_type` key is actually emitted
/// on the wire yet. Kept for documentation/precedent-consistency.
pub const TYPE_NAME: &str = "VERSION_TREE_ID";

/// `VERSION_TREE_ID` — string form of the identifier plus functions that
/// parse its `trunk_version [ '.' branch_number '.' branch_version ]`
/// lexical structure.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename = "VERSION_TREE_ID")]
pub struct VersionTreeId {
    /// `value`: string form of this identifier.
    ///
    /// Invariant `Value_valid`: `not value.is_empty`.
    ///
    /// TODO(port): invariants not yet enforced by a constructor/`Validate`
    /// impl; see the full invariant list in the doc comment on
    /// [`VersionTreeId::is_branch`] and below.
    pub value: String,
}

impl VersionTreeId {
    /// `trunk_version(): String`.
    ///
    /// Trunk version number; numbering starts at 1. The part of `value`
    /// before the first `.`, or the whole string if there is no `.`.
    ///
    /// Invariant `Trunk_version_valid`: `trunk_version /= Void and then
    /// trunk_version.is_integer and then trunk_version.as_integer >= 1`.
    pub fn trunk_version(&self) -> String {
        self.value
            .split_once('.')
            .map_or_else(|| self.value.clone(), |(trunk, _rest)| trunk.to_string())
    }

    /// `is_branch(): Boolean`.
    ///
    /// True if this version identifier represents a branch, i.e. has
    /// `branch_number` and `branch_version` parts.
    ///
    /// Invariant `Is_branch_validity`: `is_branch xor branch_number = Void`.
    pub fn is_branch(&self) -> bool {
        self.value.matches('.').count() >= 2
    }

    /// `branch_number(): String`.
    ///
    /// Number of branch from the trunk point; numbering starts at 1.
    ///
    /// Invariant `Branch_number_valid`: `branch_number /= Void implies
    /// branch_number.is_integer and then branch_number.as_integer >= 1`.
    ///
    /// TODO(port): the spec functions `branch_number`/`branch_version` are
    /// typed as non-optional `String` in the class table (1..1), yet the
    /// invariants text treats them as possibly `Void` (absent) when the
    /// identifier is not a branch (`(branch_number = Void and
    /// branch_version = Void) xor (branch_number /= Void and
    /// branch_version /= Void)`). This is a spec-internal tension between
    /// the attribute table's declared cardinality and the invariant text's
    /// `Void` checks; transcribed here returning an empty `String` for the
    /// non-branch case (mirroring `UID_BASED_ID.extension()`'s
    /// empty-string-for-absent convention) rather than introducing an
    /// `Option<String>` the class table does not declare. Flagged for
    /// review rather than silently resolved.
    pub fn branch_number(&self) -> String {
        let mut parts = self.value.splitn(3, '.');
        let _trunk = parts.next();
        match (parts.next(), parts.next()) {
            (Some(number), Some(_version)) => number.to_string(),
            _ => String::new(),
        }
    }

    /// `branch_version(): String`.
    ///
    /// Version of the branch; numbering starts at 1.
    ///
    /// Invariant `Branch_version_valid`: `branch_version /= Void implies
    /// branch_version.is_integer and then branch_version.as_integer >= 1`.
    ///
    /// See the `TODO(port)` on [`VersionTreeId::branch_number`] regarding
    /// the empty-string-for-absent convention used here.
    pub fn branch_version(&self) -> String {
        let mut parts = self.value.splitn(3, '.');
        let _trunk = parts.next();
        match (parts.next(), parts.next()) {
            (Some(_number), Some(version)) => version.to_string(),
            _ => String::new(),
        }
    }

    /// `is_first(): Boolean`.
    ///
    /// Invariant `Is_first_validity`: `not is_first xor
    /// trunk_version.is_equal("1")`.
    ///
    /// PORT NOTE: `is_first` is referenced only in the `Is_first_validity`
    /// invariant text; it does not appear in the class's Functions table.
    /// Transcribed as a derived method matching the invariant's own
    /// definition (`is_first` holds exactly when `trunk_version` is `"1"`)
    /// rather than inventing an independent attribute.
    pub fn is_first(&self) -> bool {
        self.trunk_version() == "1"
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §VERSION_TREE_ID — docs/research/spec-cache/BASE-1.2.0/uml_classes/version_tree_id.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / version_tree_id.adoc §VERSION_TREE_ID Class
//   confidence: medium
//   todos: 2
//   note: branch_number/branch_version return empty String for the non-branch case rather than Option<String>, resolving a table-vs-invariant cardinality tension in the spec text; is_first derived from the Is_first_validity invariant since it has no Functions-table entry of its own.
// ─────────────────────────────────────────────
