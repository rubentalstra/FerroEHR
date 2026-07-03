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

use openehr_foundation::serde_support::{TypeName, TypeTag};

/// Canonical `_type` discriminator string for this class in serialized
/// form. P4/ADR-002 update: this const single-sources the string carried by
/// the struct's own self-tagging `type_tag` field below (via the
/// [`TypeName`] impl), so a serialized `VersionTreeId` emits
/// `{"_type": "VERSION_TREE_ID", ...}` itself even though it is never
/// wrapped by any subtype-set enum in this package.
pub const TYPE_NAME: &str = "VERSION_TREE_ID";

/// `VERSION_TREE_ID` — string form of the identifier plus functions that
/// parse its `trunk_version [ '.' branch_number '.' branch_version ]`
/// lexical structure.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct VersionTreeId {
    /// Canonical `_type` discriminator (`"VERSION_TREE_ID"`), always
    /// serialized first; tolerated-absent and validated-if-present on input
    /// (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `value`: string form of this identifier.
    ///
    /// Invariant `Value_valid`: `not value.is_empty`.
    ///
    /// Together with `Trunk_version_valid`, `Branch_number_valid`,
    /// `Branch_version_valid`, and `Branch_validity`, this pins the lexical
    /// form to `[1-9][0-9]*(\.[1-9][0-9]*\.[1-9][0-9]*)?` — a 1- or 3-part
    /// dot-separated identifier whose parts are integers `>= 1`. Enforced
    /// by [`VersionTreeId::new`] (ADR-003 decision 8); struct-literal
    /// construction remains possible for unchecked wire data and is
    /// re-checkable via [`VersionTreeId::is_valid_value`].
    pub value: String,
}

/// Error raised by [`VersionTreeId::new`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VersionTreeIdError {
    /// The value violates the class invariants: it must be a 1-part
    /// (`trunk_version`) or 3-part (`trunk_version.branch_number.
    /// branch_version`) dot-separated identifier whose parts are integers
    /// starting at 1 (no leading zeros, no 2-part form).
    #[error("invalid VERSION_TREE_ID {0:?}: must match [1-9][0-9]*(\\.[1-9][0-9]*\\.[1-9][0-9]*)?")]
    InvalidSyntax(String),
}

impl TypeName for VersionTreeId {
    const NAME: &'static str = TYPE_NAME;
}

impl VersionTreeId {
    /// Fallible constructor enforcing the class invariants (`Value_valid`,
    /// `Trunk_version_valid`, `Branch_number_valid`,
    /// `Branch_version_valid`, `Branch_validity`) over the lexical form
    /// `trunk_version [ '.' branch_number '.' branch_version ]`.
    pub fn new(value: impl Into<String>) -> Result<Self, VersionTreeIdError> {
        let value = value.into();
        if !Self::is_valid_value(&value) {
            return Err(VersionTreeIdError::InvalidSyntax(value));
        }
        Ok(Self {
            type_tag: TypeTag::new(),
            value,
        })
    }

    /// `true` when `value` satisfies every class invariant, i.e. matches
    /// `[1-9][0-9]*(\.[1-9][0-9]*\.[1-9][0-9]*)?`.
    ///
    /// PORT NOTE: implemented with plain string checks rather than the
    /// `regex` crate — a dependency is not warranted for this pattern.
    #[must_use]
    pub fn is_valid_value(value: &str) -> bool {
        fn is_positive_integer(part: &str) -> bool {
            // `[1-9][0-9]*`: numbering starts at 1, so no leading zeros
            // and no bare "0".
            let mut bytes = part.bytes();
            matches!(bytes.next(), Some(b'1'..=b'9')) && bytes.all(|b| b.is_ascii_digit())
        }

        let parts: Vec<&str> = value.split('.').collect();
        matches!(parts.len(), 1 | 3) && parts.into_iter().all(is_positive_integer)
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_accepts_one_and_three_part_forms() {
        for valid in ["1", "2", "42", "1.2.3", "10.20.30", "1.1.1"] {
            assert!(
                VersionTreeId::new(valid).is_ok(),
                "expected {valid:?} to be valid"
            );
        }
    }

    #[test]
    fn new_rejects_invariant_violations() {
        for invalid in [
            "",        // Value_valid
            "0",       // Trunk_version_valid: >= 1
            "1.2",     // Branch_validity: both branch parts or neither
            "1.0.0",   // Branch_number_valid / Branch_version_valid: >= 1
            "1.2.0",   // Branch_version_valid: >= 1
            "01",      // no leading zeros ([1-9][0-9]*)
            "1.02.3",  // no leading zeros in branch_number
            "1.2.3.4", // no 4-part form
            "a",       // not an integer
            "1..3",    // empty middle part
            "-1",      // not a positive integer
        ] {
            assert_eq!(
                VersionTreeId::new(invalid),
                Err(VersionTreeIdError::InvalidSyntax(invalid.to_string())),
                "expected {invalid:?} to be rejected"
            );
        }
    }

    #[test]
    fn trunk_only_id_has_no_branch_parts() {
        let id = VersionTreeId::new("1").expect("valid");
        assert_eq!(id.trunk_version(), "1");
        assert!(!id.is_branch());
        assert_eq!(id.branch_number(), "");
        assert_eq!(id.branch_version(), "");
        assert!(id.is_first());
    }

    #[test]
    fn branched_id_exposes_branch_parts() {
        let id = VersionTreeId::new("2.1.4").expect("valid");
        assert_eq!(id.trunk_version(), "2");
        assert!(id.is_branch());
        assert_eq!(id.branch_number(), "1");
        assert_eq!(id.branch_version(), "4");
        assert!(!id.is_first());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §VERSION_TREE_ID — docs/research/spec-cache/BASE-1.2.0/uml_classes/version_tree_id.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / version_tree_id.adoc §VERSION_TREE_ID Class
//   confidence: medium
//   todos: 1
//   note: VersionTreeId::new + is_valid_value enforce the full invariant set (ADR-003 §8); branch_number/branch_version return empty String for the non-branch case rather than Option<String>, resolving a table-vs-invariant cardinality tension in the spec text; is_first derived from the Is_first_validity invariant since it has no Functions-table entry of its own. P4/ADR-002: self-tags via TypeTag<Self> first field (NAME single-sourced from TYPE_NAME); inert struct-level #[serde(rename)] deleted.
// ─────────────────────────────────────────────
