//! Hand-written RM class invariants for `ELEMENT`.
//!
//! Mirrors archie `Element` (non-terminology invariants) + inherited LOCATABLE:
//! - `Inv_null_flavour_indicated`: exactly one of `value` / `null_flavour` is
//!   present (XOR).
//! - `Inv_null_reason_valid`: a `null_reason` implies `value` is absent.
//! - `Archetype_node_id_valid`: `archetype_node_id` non-empty.
//!
//! NOTE: archie's `Inv_null_flavour_valid` (the null flavour code belongs
//! to the openEHR "null flavours" group) is terminology-bound — deferred to the
//! composition validator + `openehr-term`.

use crate::v1_2::data_structures::representation::element::Element;
use openehr_base::validate::{InvariantViolation, Validate};

/// The ELEMENT invariant core over the projected presence flags — one source
/// for the typed impl and the value-level fast path (`validate::fast`).
pub(crate) fn push_element_invariants(
    has_value: bool,
    has_null_flavour: bool,
    has_null_reason: bool,
    archetype_node_id: &str,
    out: &mut Vec<InvariantViolation>,
) {
    // Inv_null_flavour_indicated: exactly one of value / null_flavour.
    if has_value == has_null_flavour {
        out.push(InvariantViolation::here(
            "Invariant Inv_null_flavour_indicated failed on type ELEMENT",
        ));
    }
    // Inv_null_reason_valid: a null reason only applies when value is absent.
    if has_null_reason && has_value {
        out.push(InvariantViolation::here(
            "Invariant Inv_null_reason_valid failed on type ELEMENT",
        ));
    }
    crate::v1_2::validate::generated::archetype_node_id_core("ELEMENT", archetype_node_id, out);
}

impl Validate for Element {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        push_element_invariants(
            self.value.is_some(),
            self.null_flavour.is_some(),
            self.null_reason.is_some(),
            &self.archetype_node_id,
            out,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v1_2::data_types::basic::data_value::DataValue;
    use crate::v1_2::data_types::basic::dv_boolean::DvBoolean;
    use crate::v1_2::data_types::text::code_phrase::CodePhrase;
    use crate::v1_2::data_types::text::dv_coded_text::DvCodedText;
    use crate::v1_2::data_types::text::dv_text::{DvText, DvTextData};
    use openehr_base::v1_3::prelude::TerminologyId;

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

    fn null_flavour() -> DvCodedText {
        DvCodedText {
            value: "unknown".to_owned(),
            hyperlink: None,
            formatting: None,
            mappings: openehr_base::containers::present_nonempty(Vec::new()),
            language: None,
            encoding: None,
            defining_code: CodePhrase {
                terminology_id: TerminologyId {
                    value: "openehr".to_owned(),
                },
                code_string: "253".to_owned(),
                preferred_term: None,
            },
        }
    }

    fn element() -> Element {
        Element {
            name: text("element"),
            archetype_node_id: "at0001".to_owned(),
            uid: None,
            links: openehr_base::containers::present_nonempty(Vec::new()),
            archetype_details: None,
            feeder_audit: None,
            null_flavour: None,
            value: Some(DataValue::DvBoolean(DvBoolean { value: true })),
            null_reason: None,
        }
    }

    #[test]
    fn value_only_valid() {
        assert!(element().invariants().is_empty());
    }

    #[test]
    fn null_flavour_only_valid() {
        let mut e = element();
        e.value = None;
        e.null_flavour = Some(null_flavour());
        assert!(e.invariants().is_empty());
    }

    #[test]
    fn both_value_and_null_flavour_invalid() {
        let mut e = element();
        e.null_flavour = Some(null_flavour());
        let v = e.invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Inv_null_flavour_indicated failed on type ELEMENT"),
            "got {v:?}"
        );
    }

    #[test]
    fn null_reason_with_value_invalid() {
        let mut e = element();
        e.null_reason = Some(text("some reason"));
        let v = e.invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Inv_null_reason_valid failed on type ELEMENT"),
            "got {v:?}"
        );
    }

    #[test]
    fn empty_node_id_invalid() {
        let mut e = element();
        e.archetype_node_id = String::new();
        let v = e.invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Archetype_node_id_valid failed on type ELEMENT")
        );
    }
}
