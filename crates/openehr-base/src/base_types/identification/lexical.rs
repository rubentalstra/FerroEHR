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
//! (the error type + the `UID` subtype builder + digit predicates + the
//! composite-identifier case rule).
//!
//! Spec: `docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.base_types.*`
//! and `docs/specs/openehr/BASE/docs/base_types/master05-identification_package.adoc`.

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

/// Composite-identifier equality: `true` iff `a` and `b` are the same
/// identifier under the openEHR case rule — BASE `base_types`
/// `master05-identification_package.adoc` §"Composite Identifiers and Case":
/// "two identifiers identical apart from case are considered to be identical,
/// and therefore to identify the same thing".
///
/// This is the ONE comparison every composite identifier goes through —
/// `UID_BASED_ID` values ([`super::uid_based_id::UidBasedId::is_equal`]),
/// archetype and template ids, `INTERNET_ID` system ids. The sibling rule of
/// the same section, case-**preserving** ("not change case due to persistence,
/// copying, transfer or other computation processes"), belongs to whoever
/// stores the value: nothing here rewrites a stored string, only the
/// *comparison* folds case.
///
/// The fold is ASCII, which is exactly the section's intent: §"Composite
/// Identifiers and Language" restricts the human-readable identifier sections
/// to the basic latin character set, and §"Composite Identifiers and Case"
/// explicitly carves out languages where case does not exist (the Turkish
/// `I/i` caveat) — a Unicode-locale fold would *re-introduce* that hazard, so
/// [`str::eq_ignore_ascii_case`] is the correct, locale-safe choice.
#[must_use]
pub fn composite_ids_equal(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// The comparison/keying form of a composite identifier: the value with ASCII
/// case folded away, so that two identifiers are the same identifier exactly
/// when their keys are equal (BASE `base_types`
/// `master05-identification_package.adoc` §"Composite Identifiers and Case" —
/// the same rule [`composite_ids_equal`] decides pairwise).
///
/// For a caller that needs a *key* rather than a comparison — a hash-map entry,
/// a cache key, a SQL `lower()` predicate — this is the single derivation, so a
/// keyed lookup can never disagree with a pairwise comparison. It is
/// case-**preserving** in the spec's sense: the derived key is for lookup only,
/// never a replacement for the stored value.
#[must_use]
pub fn composite_id_key(id: &str) -> String {
    id.to_ascii_lowercase()
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
    match s.as_bytes() {
        [first, rest @ ..] => (b'1'..=b'9').contains(first) && rest.iter().all(u8::is_ascii_digit),
        [] => false,
    }
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

    /// BASE `master05` §"Composite Identifiers and Case": two identifiers
    /// identical apart from case identify the same thing, and the pairwise
    /// comparison agrees with the derived key.
    #[test]
    fn composite_id_case_rule() {
        for (a, b) in [
            ("openEHR.org", "OPENEHR.ORG"),
            ("uk.nhs.ehr1", "UK.NHS.EHR1"),
            ("FerroEHR.local", "ferroehr.local"),
            ("sys", "SYS"),
            (
                "87284370-2D4B-4E3D-A3F3-F303D2F4F34B",
                "87284370-2d4b-4e3d-a3f3-f303d2f4f34b",
            ),
        ] {
            assert!(composite_ids_equal(a, b), "{a} vs {b}");
            assert_eq!(composite_id_key(a), composite_id_key(b));
        }
        assert!(!composite_ids_equal("system.a", "system.b"));
        assert_ne!(composite_id_key("system.a"), composite_id_key("system.b"));
        // Case-preserving: neither function rewrites its input.
        let original = "openEHR.org";
        assert_eq!(original, "openEHR.org");
        assert_eq!(composite_id_key(original), "openehr.org");
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
