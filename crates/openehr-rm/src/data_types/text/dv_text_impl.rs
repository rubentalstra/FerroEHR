//! Hand-written RM class invariants for `DV_TEXT` / `DV_CODED_TEXT`.
//!
//! Spec: RM 1.2.0
//! `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.dv_text.adoc`:
//! `Valid_value: not value.is_empty` and
//! `Formatting_valid: formatting /= void implies not formatting.is_empty`.
//! `DV_CODED_TEXT` inherits both (`dv_coded_text.adoc`); its own
//! `defining_code` presence/type is structural (non-optional field).
//! `Mappings_valid` (present ⇒ non-empty list) is checked at the JSON level
//! by the composition validator (post-deserialize an absent and a
//! present-empty list are the same `Vec`); `Language_valid`/`Encoding_valid`
//! (code sets) live in the validator's terminology pass.

use crate::data_types::text::dv_text::DvText;
use crate::validate::{InvariantViolation, Validate};

impl Validate for DvText {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        let (ty, value, formatting) = match self {
            DvText::DvText(t) => ("DV_TEXT", &t.value, t.formatting.as_deref()),
            DvText::DvCodedText(t) => ("DV_CODED_TEXT", &t.value, t.formatting.as_deref()),
        };
        if value.is_empty() {
            out.push(InvariantViolation::here(format!(
                "Invariant Valid_value failed on type {ty} (value must be non-empty)"
            )));
        }
        if formatting.is_some_and(str::is_empty) {
            out.push(InvariantViolation::here(format!(
                "Invariant Formatting_valid failed on type {ty}"
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::text::dv_text::DvTextData;

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

    /// `Valid_value`: an empty rubric is rejected; `Formatting_valid`: a
    /// present-empty formatting string is rejected (`dv_text.adoc`).
    #[test]
    fn value_and_formatting_invariants() {
        let mut out = Vec::new();
        text("ok").validate_invariants(&mut out);
        assert!(out.is_empty(), "non-empty value passes: {out:?}");

        let mut out = Vec::new();
        text("").validate_invariants(&mut out);
        assert!(
            out.iter().any(|m| m.message.contains("Valid_value")),
            "empty value must fail: {out:?}"
        );

        let mut with_fmt = text("ok");
        if let DvText::DvText(t) = &mut with_fmt {
            t.formatting = Some(String::new());
        }
        let mut out = Vec::new();
        with_fmt.validate_invariants(&mut out);
        assert!(
            out.iter().any(|m| m.message.contains("Formatting_valid")),
            "empty formatting must fail: {out:?}"
        );
    }
}
