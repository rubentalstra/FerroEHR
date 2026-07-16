//! Hand-written RM class invariants for `OBSERVATION`.
//!
//! Mirrors archie's `Entry` (non-terminology) + inherited LOCATABLE:
//! - `Is_archetypeRoot`: an ENTRY is an archetype root, so `archetype_details`
//!   must be present.
//! - `Archetype_node_id_valid`: `archetype_node_id` non-empty.
//!
//! PORT NOTE: archie's `Entry.Language_valid` / `Encoding_valid` are
//! terminology-bound (deferred), and `Subject_validity` /
//! `Other_participations_valid` are `ignored`. `OBSERVATION` has no own
//! invariant.

use crate::composition::content::entry::observation::Observation;
use crate::validate::{InvariantViolation, Validate, push_entry_root_invariants};

impl Validate for Observation {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        push_entry_root_invariants(
            out,
            "OBSERVATION",
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
    use crate::data_structures::history::history::History;
    use crate::data_structures::item_structure::item_structure::ItemStructure;
    use crate::data_types::quantity::date_time::dv_date_time::DvDateTime;
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

    fn data() -> History<ItemStructure> {
        History {
            name: text("history"),
            archetype_node_id: "at0002".to_owned(),
            uid: None,
            links: Vec::new(),
            archetype_details: None,
            feeder_audit: None,
            origin: DvDateTime {
                normal_status: None,
                normal_range: None,
                other_reference_ranges: Vec::new(),
                magnitude_status: None,
                accuracy: None,
                value: "2021-01-01T00:00:00".to_owned(),
            },
            period: None,
            duration: None,
            summary: None,
            events: Vec::new(),
        }
    }

    fn observation() -> Observation {
        Observation {
            name: text("BP"),
            archetype_node_id: "openEHR-EHR-OBSERVATION.bp.v1".to_owned(),
            uid: None,
            links: Vec::new(),
            archetype_details: Some(Archetyped {
                archetype_id: ArchetypeId {
                    value: "openEHR-EHR-OBSERVATION.bp.v1".to_owned(),
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
            state: None,
        }
    }

    #[test]
    fn valid_observation() {
        assert!(observation().invariants().is_empty());
    }

    #[test]
    fn missing_archetype_details_invalid() {
        let mut o = observation();
        o.archetype_details = None;
        let v = o.invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Is_archetypeRoot failed on type OBSERVATION"),
            "got {v:?}"
        );
    }
}
