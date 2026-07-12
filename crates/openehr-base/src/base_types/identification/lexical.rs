//! Hand-written lexical-form parsing shared by the BASE identification types
//! (hand-written spec behaviour; auto-declared beside the `// @generated` files).
//!
//! openEHR BASE 1.3.0 defines each identifier class by a *lexical form* (a
//! grammar over the `value` string) plus accessor functions that decompose it
//! (`UID_BASED_ID.root`/`extension`, `OBJECT_VERSION_ID.object_id`/…,
//! `VERSION_TREE_ID.trunk_version`/…, `ARCHETYPE_ID.rm_originator`/…,
//! `TERMINOLOGY_ID.name`/…). The generator emits only the `{ value: String }`
//! struct; the accessors and a fallible strict parser live here and in the
//! sibling `*_impl.rs` files. This module holds the pieces multiple types share
//! (the error type + the `UID` subtype builder + digit predicates).
//!
//! Spec: `docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.base_types.*`.

use crate::base_types::identification::internet_id::InternetId;
use crate::base_types::identification::iso_oid::IsoOid;
use crate::base_types::identification::uid::Uid;
use crate::base_types::identification::uuid::Uuid;

/// Error raised when an identifier string does not conform to its openEHR
/// lexical form (BASE 1.3.0). Returned by the `FromStr`/`TryFrom<&str>`
/// implementations on the identification value types.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdError {
    /// The identifier string was empty (violates `UID.Value_valid` and the
    /// non-empty requirement of every identifier lexical form).
    #[error("empty identifier value")]
    Empty,
    /// A `::`/`.`-delimited component that must be present was empty.
    #[error("empty {0} component in identifier")]
    EmptyComponent(&'static str),
    /// The value had the wrong number of `::`-delimited parts for its type
    /// (e.g. an `OBJECT_VERSION_ID` without exactly three parts).
    #[error("expected {expected} '::'-delimited parts, found {found}")]
    PartCount {
        /// The number of parts the lexical form requires.
        expected: usize,
        /// The number of parts actually present.
        found: usize,
    },
    /// A `VERSION_TREE_ID` was neither a bare trunk (`N`) nor a full branch
    /// (`N.N.N`) with each segment a positive integer.
    #[error("malformed VERSION_TREE_ID: {0:?}")]
    VersionTree(String),
    /// An `ARCHETYPE_ID` did not match
    /// `rm_originator-rm_name-rm_entity.concept{-spec}*.vN`.
    #[error("malformed ARCHETYPE_ID: {0:?}")]
    Archetype(String),
}

/// `true` for a non-empty string of ASCII digits.
#[must_use]
pub(crate) fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// A positive integer with no leading zero: `[1-9][0-9]*` (used by the
/// `VERSION_TREE_ID` trunk/branch segments — numbering starts at 1).
#[must_use]
pub(crate) fn is_positive_int(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty() && (b'1'..=b'9').contains(&b[0]) && b[1..].iter().all(u8::is_ascii_digit)
}

/// Build a concrete [`Uid`] from a root/identifier string, choosing the subtype
/// by lexical form (BASE 1.3.0 `UID` hierarchy): a valid RFC-4122 UUID becomes
/// [`Uuid`]; an OID (dot-separated groups of digits, at least two groups)
/// becomes [`IsoOid`]; anything else becomes [`InternetId`]. This mirrors the
/// reference implementation's `UID.create`/`build` dispatch — the wire form of a
/// UID carries no `_type`, so the subtype is inferred from the string.
#[must_use]
pub(crate) fn make_uid(value: &str) -> Uid {
    if let Ok(u) = value.parse::<uuid::Uuid>() {
        return Uid::Uuid(Uuid { value: u });
    }
    if is_oid(value) {
        return Uid::IsoOid(IsoOid {
            value: value.to_owned(),
        });
    }
    Uid::InternetId(InternetId {
        value: value.to_owned(),
    })
}

/// `true` for an ISO OID lexical form: two or more dot-separated groups, each a
/// non-empty run of digits (e.g. `1.2.840.113554`).
#[must_use]
fn is_oid(s: &str) -> bool {
    let mut groups = 0usize;
    for g in s.split('.') {
        if !all_digits(g) {
            return false;
        }
        groups += 1;
    }
    groups >= 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_uid_picks_subtype() {
        assert!(matches!(
            make_uid("2fdbf3f0-1c0a-4a0e-9f2a-3b7f6b1e9c11"),
            Uid::Uuid(_)
        ));
        assert!(matches!(make_uid("1.2.840.113554"), Uid::IsoOid(_)));
        assert!(matches!(make_uid("openehr.org"), Uid::InternetId(_)));
        // A single digit group is not an OID (needs >= 2 groups) → internet id.
        assert!(matches!(make_uid("12345"), Uid::InternetId(_)));
    }

    #[test]
    fn positive_int_rules() {
        assert!(is_positive_int("1"));
        assert!(is_positive_int("42"));
        assert!(!is_positive_int("0"));
        assert!(!is_positive_int("01"));
        assert!(!is_positive_int(""));
        assert!(!is_positive_int("1a"));
    }
}
