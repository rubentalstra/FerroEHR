//! The optional-container convention shared by every crate that builds openEHR
//! model objects.
//!
//! Hand-written; preserved across `openehr-codegen` regeneration (it carries no
//! generated-file marker, so `write_crate` keeps it and `lib.rs` auto-declares
//! `pub mod containers;`).
//!
//! A model attribute whose declared existence is `0..1` and whose type is a
//! container emits as `Option<Vec<T>>`, because absence and present-but-
//! emptiness are two distinct states the models rely on: the
//! `x /= Void implies not x.is_empty` invariant family (e.g.
//! `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.locatable.adoc`
//! §Invariants, `Links_valid`) has nothing to judge unless both are
//! representable.
//!
//! That leaves exactly one decision for every builder that produces such an
//! attribute by COLLECTING parsed or converted members: which of the two states
//! an empty collection means. This module is the single owner of that decision
//! ([`present`]) so it is made once, with its reasoning, rather than re-decided
//! per crate.

/// Wrap a collected member list in the optional-container shape, mapping an
/// empty list to `None`.
///
/// **An empty collected list means the attribute was ABSENT in the source.**
/// Every serialization openEHR defines writes a member list by writing its
/// members, so "no members" and "no attribute" are the same input text and a
/// builder cannot have observed anything else:
///
/// - canonical JSON omits an empty list rather than writing `[]`
///   (`docs/specs/openehr/ITS-REST/specifications/docs/overview/Resources.md`
///   §JSON Format: "The RM attributes (even required ones) that are `Null` or
///   an empty list (array) SHOULD be absent when serialized as JSON");
/// - canonical XML has no representation for it at all — a repeated element
///   with zero occurrences IS absence;
/// - the ODIN/ADL persistence forms attach a member list to a keyword block
///   that is written only when it carries content
///   (`docs/specs/openehr/AM/docs/ADL2/master04-syntax.adoc` §Structure,
///   `docs/specs/openehr/LANG/docs/ODIN/master04-syntax.adoc` §Objects).
///
/// So `Some(vec![])` would assert a state no source syntax produces. The one
/// place present-but-empty legitimately arises is a JSON reader that saw a
/// literal `[]`, and that reader constructs the `Option` directly
/// (`openehr_its::json_codec::runtime::optional_container_field`) rather than
/// going through this function — which is exactly why the two paths are
/// separate.
#[must_use]
pub fn present<T>(members: Vec<T>) -> Option<Vec<T>> {
    (!members.is_empty()).then_some(members)
}

#[cfg(test)]
mod tests {
    use super::present;

    #[test]
    fn an_empty_member_list_is_absence() {
        assert_eq!(present(Vec::<u8>::new()), None);
    }

    #[test]
    fn a_populated_member_list_is_presence() {
        assert_eq!(present(vec![1_u8, 2]), Some(vec![1, 2]));
    }
}
