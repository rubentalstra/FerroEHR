// @generated-from-template templates/openehr-rm/common/resource/authored_resource_impl.rs — DO NOT EDIT; edit the source and re-run `openehr-codegen -- emit`.
//! Hand-written RM class invariants for `AUTHORED_RESOURCE`.
//!
//! Spec: RM 1.2.0
//! `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.authored_resource.adoc`
//! §Invariants — `Revision_history_valid`
//! (`is_controlled xor revision_history = Void`), evaluated by the generated
//! core only when `is_controlled` is PRESENT: the attribute is `0..1` and an
//! `xor` against a Void operand is not evaluable, so an absent flag asserts
//! nothing (refusing there would invent a prohibition the released text does
//! not contain). `Current_revision_valid` constrains the DERIVED
//! `current_revision()` function (§Functions), not stored data — adjudicated
//! in the generated register. The terminology-backed
//! `Original_language_valid` stays with the terminology binding table;
//! `Translations_valid`/`Description_valid` are cross-member map rules over
//! `translations`, realized where a whole authored resource is ingested.
//! `Languages_available_valid` (`languages_available.has (original_language)`)
//! constrains the derived `languages_available()` function, which builds its
//! result from `original_language` — so it holds by that function's own
//! definition, the same venue as `Current_revision_valid`.

use crate::v1_2::common::resource::authored_resource::AuthoredResource;
use openehr_base::validate::{InvariantViolation, Validate};

impl Validate for AuthoredResource {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        crate::v1_2::validate::generated::authored_resource_core(
            self.is_controlled,
            self.revision_history.is_some(),
            out,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resource(is_controlled: Option<bool>) -> AuthoredResource {
        AuthoredResource {
            original_language: crate::v1_2::data_types::text::code_phrase::CodePhrase {
                terminology_id:
                    openehr_base::v1_3::base_types::identification::terminology_id::TerminologyId {
                        value: "ISO_639-1".to_owned(),
                    },
                code_string: "en".to_owned(),
                preferred_term: None,
            },
            is_controlled,
            translations: None,
            description: None,
            revision_history: None,
        }
    }

    #[test]
    fn uncontrolled_without_history_passes() {
        assert!(resource(Some(false)).invariants().is_empty());
    }

    #[test]
    fn controlled_without_history_is_a_violation() {
        let v = resource(Some(true)).invariants();
        assert!(
            v.iter()
                .any(|m| m.message.contains("Revision_history_valid")),
            "got {v:?}"
        );
    }

    #[test]
    fn absent_is_controlled_asserts_nothing() {
        assert!(resource(None).invariants().is_empty());
    }
}
