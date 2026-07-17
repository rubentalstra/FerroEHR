//! Hand-written RM class invariant for `FOLDER`.
//!
//! Only the inherited LOCATABLE `Archetype_node_id_valid`. archie's own
//! `Folder.Folders_valid` is `ignored`.

use crate::common::directory::folder::Folder;
use crate::validate::{InvariantViolation, Validate, push_archetype_node_id_valid};

impl Validate for Folder {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        push_archetype_node_id_valid(out, "FOLDER", &self.archetype_node_id);
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
    use crate::data_types::text::dv_text::{DvText, DvTextData};

    fn folder(node_id: &str) -> Folder {
        Folder {
            name: DvText::DvText(DvTextData {
                value: "folder".to_owned(),
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
            folders: Vec::new(),
            details: None,
        }
    }

    #[test]
    fn valid_folder() {
        assert!(folder("at0001").invariants().is_empty());
    }

    #[test]
    fn empty_node_id_invalid() {
        assert_eq!(
            folder("").invariants()[0].message,
            "Invariant Archetype_node_id_valid failed on type FOLDER"
        );
    }
}
