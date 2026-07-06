//! Hand-written RM/BASE class invariants (ADR-003) for `VERSION_TREE_ID`.
//!
//! Mirrors archie `VersionTreeId`:
//! - `Value_valid`: value non-empty.
//! - `Value_format_valid`: value is empty, or matches the trunk form
//!   `[1-9][0-9]*`, or the branch form `[1-9][0-9]*.[0-9]+.[0-9]+`.

use super::version_tree_id::VersionTreeId;
use crate::validate::{InvariantViolation, Validate};

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// Trunk version: `[1-9][0-9]*`.
fn is_trunk(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty() && (b'1'..=b'9').contains(&b[0]) && b[1..].iter().all(u8::is_ascii_digit)
}

/// Branch version: `[1-9][0-9]*.[0-9]+.[0-9]+`.
fn is_branch(s: &str) -> bool {
    matches!(s.split('.').collect::<Vec<_>>().as_slice(),
        [trunk, branch_no, branch_ver]
        if is_trunk(trunk) && all_digits(branch_no) && all_digits(branch_ver))
}

impl Validate for VersionTreeId {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if self.value.is_empty() {
            out.push(InvariantViolation::here(
                "Invariant Value_valid failed on type VERSION_TREE_ID",
            ));
        }
        // Format check short-circuits `true` on empty (matches archie).
        if !self.value.is_empty() && !is_trunk(&self.value) && !is_branch(&self.value) {
            out.push(InvariantViolation::here(
                "Invariant Value_format_valid failed on type VERSION_TREE_ID",
            ));
        }
    }
}

#[cfg(test)]
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
        for bad in ["a", "1.2", "1.2.a", "1/2/2", "-1", "1.-1.1", "1.1.-1", "01"] {
            let v = vtid(bad);
            assert!(
                v.iter()
                    .any(|m| m.message
                        == "Invariant Value_format_valid failed on type VERSION_TREE_ID"),
                "{bad:?} should fail format, got {v:?}"
            );
        }
    }
}
