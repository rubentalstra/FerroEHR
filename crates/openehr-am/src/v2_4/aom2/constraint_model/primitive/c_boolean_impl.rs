//! Hand-written AOM2 `C_BOOLEAN` spec functions.
//!
//! Spec source (vendored):
//! `AM/docs/UML/classes/org.openehr.am.aom2.c_boolean.adoc` §Functions.

use crate::v2_4::aom2::constraint_model::primitive::c_boolean::CBoolean;

impl CBoolean {
    /// Returns true if any Boolean value would be allowed.
    ///
    /// `any_allowed` (`org.openehr.am.aom2.c_boolean.adoc` §Functions),
    /// post-condition `Result = constraint.is_empty`. The attribute is `0..1`,
    /// so absent and present-but-empty both read as "no constraint stated".
    #[must_use]
    pub fn any_allowed(&self) -> bool {
        self.constraint.as_deref().is_none_or(<[bool]>::is_empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boolean(constraint: Option<Vec<bool>>) -> CBoolean {
        CBoolean {
            parent: None,
            soc_parent: None,
            rm_type_name: "Boolean".to_owned(),
            occurrences: None,
            node_id: "at9999".to_owned(),
            alternative_ids: None,
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value: None,
            is_enumerated_type_constraint: None,
            constraint,
        }
    }

    #[test]
    fn an_unstated_constraint_allows_any_value() {
        assert!(boolean(None).any_allowed());
        assert!(boolean(Some(Vec::new())).any_allowed());
    }

    #[test]
    fn one_permitted_value_is_already_a_constraint() {
        assert!(!boolean(Some(vec![true])).any_allowed());
        assert!(!boolean(Some(vec![true, false])).any_allowed());
    }
}
