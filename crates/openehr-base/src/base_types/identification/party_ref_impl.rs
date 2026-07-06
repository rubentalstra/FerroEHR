//! Hand-written RM/BASE class invariants (ADR-003) for `PARTY_REF`.
//!
//! Mirrors archie `PartyRef`:
//! - `Type_validity`: `type` is one of the demographic/party types.
//! - `Namespace_valid`: inherited from `OBJECT_REF`.

use super::object_ref_impl::namespace_valid;
use super::party_ref::PartyRef;
use crate::validate::{InvariantViolation, Validate};

/// The party/demographic `type` values a `PARTY_REF` may point at (archie
/// `PartyRef.VALID_PARTY_TYPES`).
const VALID_PARTY_TYPES: &[&str] = &[
    "PERSON",
    "ORGANISATION",
    "GROUP",
    "AGENT",
    "ROLE",
    "PARTY",
    "ACTOR",
];

impl Validate for PartyRef {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if !VALID_PARTY_TYPES.contains(&self.r#type.as_str()) {
            out.push(InvariantViolation::here(
                "Invariant Type_validity failed on type PARTY_REF",
            ));
        }
        if !namespace_valid(&self.namespace) {
            out.push(InvariantViolation::here(
                "Invariant Namespace_valid failed on type PARTY_REF",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base_types::identification::hier_object_id::HierObjectId;
    use crate::base_types::identification::object_id::ObjectId;

    fn party_ref(namespace: &str, ty: &str) -> PartyRef {
        PartyRef {
            namespace: namespace.to_owned(),
            r#type: ty.to_owned(),
            id: ObjectId::HierObjectId(HierObjectId {
                value: "abc".to_owned(),
            }),
        }
    }

    #[test]
    fn valid_party_ref() {
        assert!(party_ref("local", "PERSON").invariants().is_empty());
    }

    #[test]
    fn invalid_type() {
        let v = party_ref("local", "SOMEONE").invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Type_validity failed on type PARTY_REF"),
            "got {v:?}"
        );
    }

    #[test]
    fn invalid_namespace() {
        let v = party_ref("A*", "AGENT").invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Namespace_valid failed on type PARTY_REF"),
            "got {v:?}"
        );
    }
}
