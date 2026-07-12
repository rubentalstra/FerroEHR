//! Hand-written RM class invariant for `SECTION`.
//!
//! Only the inherited LOCATABLE `Archetype_node_id_valid`. archie's own
//! `Section.Items_valid` is `ignored`.

use crate::composition::content::navigation::section::Section;
use crate::validate::{InvariantViolation, Validate, push_archetype_node_id_valid};

impl Validate for Section {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        push_archetype_node_id_valid(out, "SECTION", &self.archetype_node_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::text::dv_text::{DvText, DvTextData};

    fn section(node_id: &str) -> Section {
        Section {
            name: DvText::DvText(DvTextData {
                value: "section".to_owned(),
                hyperlink: None,
                formatting: None,
                mappings: Vec::new(),
                language: None,
                encoding: None,
            }),
            archetype_node_id: node_id.to_owned(),
            uid: None,
            links: Vec::new(),
            archetype_details: None,
            feeder_audit: None,
            items: Vec::new(),
        }
    }

    #[test]
    fn valid_section() {
        assert!(section("at0001").invariants().is_empty());
    }

    #[test]
    fn empty_node_id_invalid() {
        assert_eq!(
            section("").invariants()[0].message,
            "Invariant Archetype_node_id_valid failed on type SECTION"
        );
    }
}
