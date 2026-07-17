//! Hand-written RM/BASE class invariant for `OBJECT_REF`.
//!
//! `Namespace_valid` (archie `ObjectRef`): `namespace` matches the openEHR
//! namespace regex `[a-zA-Z][a-zA-Z0-9_.:/&?=+-]*` (the special values `local`
//! and `unknown` are ordinary matches of it).

use super::object_ref::ObjectRefData;
use crate::validate::{InvariantViolation, Validate};

/// `true` when `ns` matches the openEHR namespace regex
/// `[a-zA-Z][a-zA-Z0-9_.:/&?=+-]*`. Shared with `PartyRef` (which inherits the
/// `OBJECT_REF` invariant).
#[must_use]
pub(crate) fn namespace_valid(ns: &str) -> bool {
    let mut chars = ns.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(c, '_' | '.' | ':' | '/' | '&' | '?' | '=' | '+' | '-')
    })
}

impl Validate for ObjectRefData {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if !namespace_valid(&self.namespace) {
            out.push(InvariantViolation::here(
                "Invariant Namespace_valid failed on type OBJECT_REF",
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

    #[test]
    fn namespace_regex() {
        assert!(namespace_valid("local"));
        assert!(namespace_valid("unknown"));
        assert!(namespace_valid("some.name-space_1"));
        assert!(!namespace_valid("1badhjklcd"));
        assert!(!namespace_valid("A*"));
        assert!(!namespace_valid(""));
    }
}
