//! Hand-written RM class invariants (ADR-003) for `ACTIVITY`.
//!
//! - `Action_archetype_id_valid` (archie `Activity`, `nullOrNotEmpty`):
//!   `action_archetype_id` non-empty.
//! - `Archetype_node_id_valid`: inherited LOCATABLE.

use crate::composition::content::entry::activity::Activity;
use crate::validate::{InvariantViolation, Validate, push_archetype_node_id_valid};

impl Validate for Activity {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if self.action_archetype_id.is_empty() {
            out.push(InvariantViolation::here(
                "Invariant Action_archetype_id_valid failed on type ACTIVITY",
            ));
        }
        push_archetype_node_id_valid(out, "ACTIVITY", &self.archetype_node_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_structures::item_structure::item_structure::ItemStructure;
    use crate::data_structures::item_structure::item_tree::ItemTree;
    use crate::data_types::text::dv_text::{DvText, DvTextData};

    fn text(value: &str) -> DvText {
        DvText::DvText(DvTextData {
            value: value.to_owned(),
            hyperlink: None,
            formatting: None,
            mappings: Vec::new(),
            language: None,
            encoding: None,
        })
    }

    fn description() -> ItemStructure {
        ItemStructure::ItemTree(Box::new(ItemTree {
            name: text("tree"),
            archetype_node_id: "at0003".to_owned(),
            uid: None,
            links: Vec::new(),
            archetype_details: None,
            feeder_audit: None,
            items: Vec::new(),
        }))
    }

    fn activity(action_archetype_id: &str) -> Activity {
        Activity {
            name: text("activity"),
            archetype_node_id: "at0001".to_owned(),
            uid: None,
            links: Vec::new(),
            archetype_details: None,
            feeder_audit: None,
            timing: None,
            action_archetype_id: action_archetype_id.to_owned(),
            description: description(),
        }
    }

    #[test]
    fn valid_activity() {
        assert!(activity("/.*/").invariants().is_empty());
    }

    #[test]
    fn empty_action_archetype_id_invalid() {
        let v = activity("").invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Action_archetype_id_valid failed on type ACTIVITY"),
            "got {v:?}"
        );
    }
}
