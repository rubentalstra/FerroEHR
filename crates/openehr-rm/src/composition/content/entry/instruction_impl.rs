//! Hand-written RM class invariants for `INSTRUCTION`.
//!
//! Inherited `Entry` + LOCATABLE invariants (`Is_archetypeRoot`,
//! `Archetype_node_id_valid`). See `observation_impl` for the PORT NOTE.
//! archie's own `Instruction.Activities_valid` is `ignored`.

use crate::composition::content::entry::instruction::Instruction;
use crate::validate::{InvariantViolation, Validate, push_entry_root_invariants};

impl Validate for Instruction {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        push_entry_root_invariants(
            out,
            "INSTRUCTION",
            self.archetype_details.is_some(),
            &self.archetype_node_id,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::archetyped::archetyped::Archetyped;
    use crate::common::generic::party_proxy::PartyProxy;
    use crate::common::generic::party_self::PartySelf;
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

    fn instruction() -> Instruction {
        Instruction {
            name: text("Medication order"),
            archetype_node_id: "openEHR-EHR-INSTRUCTION.medication.v1".to_owned(),
            uid: None,
            links: Vec::new(),
            archetype_details: Some(Archetyped {
                archetype_id: ArchetypeId {
                    value: "openEHR-EHR-INSTRUCTION.medication.v1".to_owned(),
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
            narrative: text("Take once daily"),
            expiry_time: None,
            wf_definition: None,
            activities: Vec::new(),
        }
    }

    #[test]
    fn valid_instruction() {
        assert!(instruction().invariants().is_empty());
    }

    #[test]
    fn missing_archetype_details_invalid() {
        let mut i = instruction();
        i.archetype_details = None;
        assert!(
            i.invariants()
                .iter()
                .any(|m| m.message == "Invariant Is_archetypeRoot failed on type INSTRUCTION")
        );
    }
}
