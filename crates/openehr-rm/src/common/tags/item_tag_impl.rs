//! Hand-written RM class invariants for `ITEM_TAG`.
//!
//! Spec: RM 1.2.0
//! `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.item_tag.adoc`:
//! - `Inv_key_valid`: `not key.is_empty and key.is_justified` (no leading or
//!   trailing whitespace).
//! - `Inv_value_valid`: `value /= Void implies not value.is_empty`.

use crate::common::tags::item_tag::ItemTag;
use crate::validate::{InvariantViolation, Validate};

impl Validate for ItemTag {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        let justified = self.key.trim() == self.key;
        if self.key.is_empty() || !justified {
            out.push(InvariantViolation::here(
                "Invariant Inv_key_valid failed on type ITEM_TAG",
            ));
        }
        if self.value.as_deref() == Some("") {
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
            target: UidBasedId::HierObjectId(HierObjectId {
                value: "87284370-2D4B-4e3d-A3F3-F303D2F4F34B".to_owned(),
            }),
            target_path: None,
            owner_id: ObjectRef::ObjectRef(ObjectRefData {
                namespace: "local".to_owned(),
                r#type: "EHR".to_owned(),
                id: ObjectId::HierObjectId(HierObjectId {
                    value: "b5e19c3a-16c8-4d3e-8b1a-1b1c9dd07f11".to_owned(),
                }),
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
}
