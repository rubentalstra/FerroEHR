//! Hand-written RM class invariant for `CLUSTER`.
//!
//! Only the inherited LOCATABLE `Archetype_node_id_valid`.
//!
//! NOTE: openEHR's CLUSTER spec has an "items not empty" invariant, but the
//! reference implementation archie does **not** enforce it (no `@Invariant`), so
//! we do not either — enforcing it would over-reject relative to the reference.

use crate::data_structures::representation::cluster::Cluster;
use crate::validate::{InvariantViolation, Validate};

impl Validate for Cluster {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        crate::validate::generated::archetype_node_id_core("CLUSTER", &self.archetype_node_id, out);
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

    fn cluster(node_id: &str) -> Cluster {
        Cluster {
            name: text("cluster"),
            archetype_node_id: node_id.to_owned(),
            uid: None,
            links: Vec::new(),
            archetype_details: None,
            feeder_audit: None,
            items: Vec::new(),
        }
    }

    #[test]
    fn empty_items_still_valid() {
        assert!(cluster("at0001").invariants().is_empty());
    }

    #[test]
    fn empty_node_id_invalid() {
        assert_eq!(
            cluster("").invariants()[0].message,
            "Invariant Archetype_node_id_valid failed on type CLUSTER"
        );
    }
}
