//! Hand-written RM/BASE class invariants + accessor functions for
//! `VERSION_TREE_ID`.
//!
//! Spec: BASE 1.3.0
//! `docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.base_types.version_tree_id.adoc`.
//! Lexical form: `trunk_version [ '.' branch_number '.' branch_version ]`.
//!
//! Invariants (archie `VersionTreeId`):
//! - `Value_valid`: value non-empty.
//! - `Value_format_valid`: value is the trunk form `[1-9][0-9]*` or the branch
//!   form `[1-9][0-9]*.[1-9][0-9]*.[1-9][0-9]*` (numbering starts at 1, so the
//!   spec's `Trunk_version_valid`/`Branch_number_valid`/`Branch_version_valid`
//!   (each segment ≥ 1) and `Branch_validity`/`Is_branch_validity` are all
//!   subsumed by this single format check).
//!
//! Accessor functions (`trunk_version`, `is_branch`, `is_first`,
//! `branch_number`, `branch_version`) decompose the `value` string.

use super::lexical::{IdError, is_positive_int};
use super::version_tree_id::VersionTreeId;
use crate::validate::{InvariantViolation, Validate};
use std::str::FromStr;

/// Trunk version: `[1-9][0-9]*` (numbering starts at 1).
fn is_trunk(s: &str) -> bool {
    is_positive_int(s)
}

/// Branch version: `[1-9][0-9]*.[1-9][0-9]*.[1-9][0-9]*`.
fn is_branch_form(s: &str) -> bool {
    matches!(s.split('.').collect::<Vec<_>>().as_slice(),
        [trunk, branch_no, branch_ver]
        if is_trunk(trunk) && is_positive_int(branch_no) && is_positive_int(branch_ver))
}

/// `true` if `value` is a well-formed trunk or branch identifier.
#[must_use]
pub(crate) fn is_valid_version_tree(value: &str) -> bool {
    is_trunk(value) || is_branch_form(value)
}

impl VersionTreeId {
    /// The trunk version number (BASE `VERSION_TREE_ID.trunk_version`): the part
    /// before the first `.`, or the whole value if there is no `.`.
    #[must_use]
    pub fn trunk_version(&self) -> &str {
        self.value
            .split_once('.')
            .map_or(self.value.as_str(), |(t, _)| t)
    }

    /// `true` if this identifier represents a branch, i.e. has three
    /// `.`-separated parts (BASE `VERSION_TREE_ID.is_branch`).
    #[must_use]
    pub fn is_branch(&self) -> bool {
        self.value.split('.').count() == 3
    }

    /// `true` if this is the first version, i.e. `trunk_version` is `1` (BASE
    /// `VERSION_TREE_ID.is_first`).
    #[must_use]
    pub fn is_first(&self) -> bool {
        self.trunk_version() == "1"
    }

    /// The branch number (the second `.`-separated part), or `None` for a trunk
    /// identifier (BASE `VERSION_TREE_ID.branch_number`).
    #[must_use]
    pub fn branch_number(&self) -> Option<&str> {
        match self.value.split('.').collect::<Vec<_>>().as_slice() {
            [_, b, _] => Some(b),
            _ => None,
        }
    }

    /// The branch version (the third `.`-separated part), or `None` for a trunk
    /// identifier (BASE `VERSION_TREE_ID.branch_version`).
    #[must_use]
    pub fn branch_version(&self) -> Option<&str> {
        match self.value.split('.').collect::<Vec<_>>().as_slice() {
            [_, _, v] => Some(v),
            _ => None,
        }
    }
}

impl FromStr for VersionTreeId {
    type Err = IdError;

    /// Parse a `VERSION_TREE_ID`, enforcing the lexical form strictly (BASE
    /// 1.3.0). Returns [`IdError::VersionTree`] for anything that is neither a
    /// bare trunk nor a full `N.N.N` branch.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(IdError::Empty);
        }
        if !is_valid_version_tree(s) {
            return Err(IdError::VersionTree(s.to_owned()));
        }
        Ok(Self {
            value: s.to_owned(),
        })
    }
}

impl Validate for VersionTreeId {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if self.value.is_empty() {
            out.push(InvariantViolation::here(
                "Invariant Value_valid failed on type VERSION_TREE_ID",
            ));
        }
        // Format check short-circuits `true` on empty (matches archie).
        if !self.value.is_empty() && !is_valid_version_tree(&self.value) {
            out.push(InvariantViolation::here(
                "Invariant Value_format_valid failed on type VERSION_TREE_ID",
            ));
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;

    fn vtid(value: &str) -> Vec<InvariantViolation> {
        VersionTreeId {
            value: value.to_owned(),
        }
        .invariants()
    }

    #[test]
    fn valid_forms() {
        assert!(vtid("1").is_empty());
        assert!(vtid("42").is_empty());
        assert!(vtid("1.2.3").is_empty());
    }

    #[test]
    fn empty_fails_value_valid_only() {
        let v = vtid("");
        assert_eq!(v.len(), 1);
        assert_eq!(
            v[0].message,
            "Invariant Value_valid failed on type VERSION_TREE_ID"
        );
    }

    #[test]
    fn malformed_fails_format() {
        for bad in [
            "a", "1.2", "1.2.a", "1/2/2", "-1", "1.-1.1", "1.1.-1", "01", "1.0.1",
        ] {
            let v = vtid(bad);
            assert!(
                v.iter()
                    .any(|m| m.message
                        == "Invariant Value_format_valid failed on type VERSION_TREE_ID"),
                "{bad:?} should fail format, got {v:?}"
            );
        }
    }

    #[test]
    fn accessors_trunk() {
        let t = VersionTreeId {
            value: "1".to_owned(),
        };
        assert_eq!(t.trunk_version(), "1");
        assert!(!t.is_branch());
        assert!(t.is_first());
        assert_eq!(t.branch_number(), None);
        assert_eq!(t.branch_version(), None);
    }

    #[test]
    fn accessors_branch() {
        let b = VersionTreeId {
            value: "2.1.4".to_owned(),
        };
        assert_eq!(b.trunk_version(), "2");
        assert!(b.is_branch());
        assert!(!b.is_first());
        assert_eq!(b.branch_number(), Some("1"));
        assert_eq!(b.branch_version(), Some("4"));
    }

    #[test]
    fn from_str_strict() {
        assert_eq!("1".parse::<VersionTreeId>().unwrap().value, "1".to_owned());
        assert!("1.2.3".parse::<VersionTreeId>().is_ok());
        assert_eq!("".parse::<VersionTreeId>(), Err(IdError::Empty));
        assert!(matches!(
            "1.2".parse::<VersionTreeId>(),
            Err(IdError::VersionTree(_))
        ));
    }
}
