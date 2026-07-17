//! Hand-written RM class invariants for `EVALUATION`.
//!
//! Inherited `Entry` + LOCATABLE invariants (`Is_archetypeRoot`,
//! `Archetype_node_id_valid`). See `observation_impl` for the NOTE on the
//! deferred terminology-bound `Entry` invariants.

use crate::composition::content::entry::evaluation::Evaluation;
use crate::validate::{InvariantViolation, Validate, push_entry_root_invariants};

impl Validate for Evaluation {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        push_entry_root_invariants(
            out,
            "EVALUATION",
            self.archetype_details.is_some(),
            &self.archetype_node_id,
        );
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
    use crate::common::archetyped::archetyped::Archetyped;
    use crate::common::generic::party_proxy::PartyProxy;
    use crate::common::generic::party_self::PartySelf;
    use crate::data_structures::item_structure::item_structure::ItemStructure;
    use crate::data_structures::item_structure::item_tree::ItemTree;
    use crate::data_types::text::code_phrase::CodePhrase;
    use crate::data_types::text::dv_text::{DvText, DvTextData};
    use openehr_base::prelude::{ArchetypeId, TerminologyId};

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

    fn code(terminology: &str, code: &str) -> CodePhrase {
        CodePhrase {
            terminology_id: TerminologyId {
                value: terminology.to_owned(),
            },
            code_string: code.to_owned(),
            preferred_term: None,
        }
    }

    fn data() -> ItemStructure {
        ItemStructure::ItemTree(Box::new(ItemTree {
            name: text("tree"),
            archetype_node_id: "at0002".to_owned(),
            uid: None,
            links: Vec::new(),
            archetype_details: None,
            feeder_audit: None,
            items: Vec::new(),
        }))
    }

    fn evaluation() -> Evaluation {
        Evaluation {
            name: text("Problem"),
            archetype_node_id: "openEHR-EHR-EVALUATION.problem.v1".to_owned(),
            uid: None,
            links: Vec::new(),
            archetype_details: Some(Archetyped {
                archetype_id: ArchetypeId {
                    value: "openEHR-EHR-EVALUATION.problem.v1".to_owned(),
                },
                template_id: None,
                rm_version: "1.1.0".to_owned(),
            }),
            feeder_audit: None,
            language: code("ISO_639-1", "en"),
            encoding: code("IANA_character-sets", "UTF-8"),
            other_participations: Vec::new(),
            workflow_id: None,
            subject: PartyProxy::PartySelf(PartySelf { external_ref: None }),
            provider: None,
            protocol: None,
            guideline_id: None,
            data: data(),
        }
    }

    #[test]
    fn valid_evaluation() {
        assert!(evaluation().invariants().is_empty());
    }

    #[test]
    fn missing_archetype_details_invalid() {
        let mut e = evaluation();
        e.archetype_details = None;
        assert!(
            e.invariants()
                .iter()
                .any(|m| m.message == "Invariant Is_archetypeRoot failed on type EVALUATION")
        );
    }
}
