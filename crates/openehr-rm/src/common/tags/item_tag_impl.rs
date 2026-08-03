//! Hand-written RM class invariants for `ITEM_TAG`.
//!
//! Spec: RM 1.2.0
//! `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.item_tag.adoc`:
//! - `Inv_key_valid`: `not key.is_empty and key.is_justified` (no leading or
//!   trailing whitespace).
//! - `Inv_value_valid`: `value /= Void implies not value.is_empty`.

use crate::common::tags::item_tag::ItemTag;
use crate::validate::{InvariantViolation, Validate};

impl ItemTag {
    /// `Inv_key_valid` as a predicate on a candidate key: `not key.is_empty and
    /// key.is_justified`.
    ///
    /// This is the ONE implementation of the invariant. [`Validate`] evaluates
    /// it on a whole instance, and a caller that must judge a key *before* an
    /// `ITEM_TAG` can be constructed — a wire payload whose `target`/`owner_id`
    /// the server has not yet assigned — evaluates the same function rather
    /// than restating the rule.
    ///
    /// NOTE: `is_justified` is defined in no released BASE section (`String`
    /// declares no such function), so the Meaning column of
    /// `org.openehr.rm.common.item_tag.adoc` is the only text that says what it
    /// means: "May not be empty or contain leading or trailing whitespace".
    /// That sentence is what this predicate implements.
    #[must_use]
    pub fn key_valid(key: &str) -> bool {
        !key.is_empty() && key.trim() == key
    }

    /// `Inv_value_valid` as a predicate on a candidate value:
    /// `value /= Void implies not value.is_empty`. The ONE implementation, for
    /// the same reason as [`ItemTag::key_valid`].
    #[must_use]
    pub fn value_valid(value: Option<&str>) -> bool {
        value != Some("")
    }
}

impl Validate for ItemTag {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if !ItemTag::key_valid(&self.key) {
            out.push(InvariantViolation::here(
                "Invariant Inv_key_valid failed on type ITEM_TAG",
            ));
        }
        if !ItemTag::value_valid(self.value.as_deref()) {
            out.push(InvariantViolation::here(
                "Invariant Inv_value_valid failed on type ITEM_TAG",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openehr_base::prelude::{HierObjectId, ObjectId, ObjectRef, ObjectRefData, UidBasedId};

    fn tag(key: &str, value: Option<&str>) -> ItemTag {
        ItemTag {
            key: key.to_owned(),
            value: value.map(str::to_owned),
            target: UidBasedId::HierObjectId(
                HierObjectId::new("87284370-2D4B-4e3d-A3F3-F303D2F4F34B".to_owned())
                    .expect("a well-formed identifier"),
            ),
            target_path: None,
            owner_id: ObjectRef::ObjectRef(ObjectRefData {
                namespace: "local".to_owned(),
                r#type: "EHR".to_owned(),
                id: ObjectId::HierObjectId(
                    HierObjectId::new("b5e19c3a-16c8-4d3e-8b1a-1b1c9dd07f11".to_owned())
                        .expect("a well-formed identifier"),
                ),
            }),
        }
    }

    #[test]
    fn valid_tag() {
        assert!(tag("problem-list", None).invariants().is_empty());
        assert!(tag("severity", Some("high")).invariants().is_empty());
    }

    #[test]
    fn key_must_be_non_empty_and_justified() {
        for bad in ["", " padded", "padded ", " both "] {
            let v = tag(bad, None).invariants();
            assert!(
                v.iter()
                    .any(|m| m.message == "Invariant Inv_key_valid failed on type ITEM_TAG"),
                "{bad:?} should fail, got {v:?}"
            );
        }
    }

    #[test]
    fn set_value_must_be_non_empty() {
        let v = tag("k", Some("")).invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Inv_value_valid failed on type ITEM_TAG"),
            "got {v:?}"
        );
    }

    #[test]
    fn the_predicates_and_the_instance_check_agree() {
        // The predicates ARE the invariant bodies, so a caller that can only
        // reach the predicates (a wire payload with no target/owner_id yet)
        // gets exactly the verdict `Validate` would give on the instance.
        for key in ["", " padded", "padded ", " both ", "ok"] {
            assert_eq!(
                ItemTag::key_valid(key),
                !tag(key, None)
                    .invariants()
                    .iter()
                    .any(|m| m.message == "Invariant Inv_key_valid failed on type ITEM_TAG"),
                "key {key:?}"
            );
        }
        for value in [None, Some(""), Some("v")] {
            assert_eq!(
                ItemTag::value_valid(value),
                !tag("k", value)
                    .invariants()
                    .iter()
                    .any(|m| m.message == "Invariant Inv_value_valid failed on type ITEM_TAG"),
                "value {value:?}"
            );
        }
    }
}
