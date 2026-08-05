//! Hand-written RM class invariants for `COMPOSITION`.
//!
//! Mirrors archie `Composition` (non-terminology) + inherited LOCATABLE:
//! - `Is_archetype_root`: a COMPOSITION is an archetype root, so
//!   `archetype_details` must be present.
//! - `Archetype_node_id_valid`: `archetype_node_id` non-empty.
//!
//! NOTE: archie's `Category_validity`, `Territory_valid`, `Language_valid`
//! are terminology-bound (deferred to the composition validator + `openehr-term`),
//! and its `Content valid` invariant is `ignored`. The openEHR spec constraints
//! that archie does **not** enforce (composer present, persistent-category ⇒
//! context rules, content are ENTRY/SECTION) are likewise not implemented, to
//! match the reference behaviour (content typing is already guaranteed
//! structurally by the `ContentItem` enum).

use crate::v1_1::composition::composition::Composition;
use openehr_base::validate::{InvariantViolation, Validate};

impl Validate for Composition {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        crate::v1_1::validate::generated::composition_core(
            self.archetype_details.is_some(),
            &self.archetype_node_id,
            out,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v1_1::common::archetyped::archetyped::Archetyped;
    use crate::v1_1::common::generic::party_proxy::PartyProxy;
    use crate::v1_1::common::generic::party_self::PartySelf;
    use crate::v1_1::data_types::text::code_phrase::CodePhrase;
    use crate::v1_1::data_types::text::dv_coded_text::DvCodedText;
    use crate::v1_1::data_types::text::dv_text::{DvText, DvTextData};
    use openehr_base::v1_2::prelude::{ArchetypeId, TerminologyId};

    fn text(value: &str) -> DvText {
        DvText::DvText(DvTextData {
            value: value.to_owned(),
            hyperlink: None,
            formatting: None,
            mappings: openehr_base::containers::present_nonempty(Vec::new()),
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

    fn category() -> DvCodedText {
        DvCodedText {
            value: "event".to_owned(),
            hyperlink: None,
            formatting: None,
            mappings: openehr_base::containers::present_nonempty(Vec::new()),
            language: None,
            encoding: None,
            defining_code: code("openehr", "433"),
        }
    }

    fn composition() -> Composition {
        Composition {
            name: text("Encounter"),
            archetype_node_id: "openEHR-EHR-COMPOSITION.encounter.v1".to_owned(),
            uid: None,
            links: openehr_base::containers::present_nonempty(Vec::new()),
            archetype_details: Some(Archetyped {
                archetype_id: ArchetypeId {
                    value: "openEHR-EHR-COMPOSITION.encounter.v1".to_owned(),
                },
                template_id: None,
                rm_version: "1.1.0".to_owned(),
            }),
            feeder_audit: None,
            language: code("ISO_639-1", "en"),
            territory: code("ISO_3166-1", "GB"),
            category: category(),
            context: None,
            composer: PartyProxy::PartySelf(PartySelf { external_ref: None }),
            content: openehr_base::containers::present_nonempty(Vec::new()),
        }
    }

    #[test]
    fn valid_composition() {
        assert!(composition().invariants().is_empty());
    }

    #[test]
    fn missing_archetype_details_invalid() {
        let mut c = composition();
        c.archetype_details = None;
        let v = c.invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Is_archetype_root failed on type COMPOSITION"),
            "got {v:?}"
        );
    }
}
