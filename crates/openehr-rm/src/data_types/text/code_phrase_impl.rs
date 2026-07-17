//! Hand-written RM class invariant for `CODE_PHRASE`.
//!
//! `Code_string_valid` (archie `CodePhrase`, `nullOrNotEmpty`): the code string
//! must be non-empty.

use crate::data_types::text::code_phrase::CodePhrase;
use crate::validate::{InvariantViolation, Validate};

/// The `Code_string_valid` core over the projected input — one source for the
/// typed impl and the value-level fast path (`validate::fast`).
pub(crate) fn push_code_phrase_invariants(code_string: &str, out: &mut Vec<InvariantViolation>) {
    if code_string.is_empty() {
        out.push(InvariantViolation::here(
            "Invariant Code_string_valid failed on type CODE_PHRASE",
        ));
    }
}

impl Validate for CodePhrase {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        push_code_phrase_invariants(&self.code_string, out);
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
    use openehr_base::prelude::TerminologyId;

    fn code_phrase(code: &str) -> CodePhrase {
        CodePhrase {
            terminology_id: TerminologyId {
                value: "local".to_owned(),
            },
            code_string: code.to_owned(),
            preferred_term: None,
        }
    }

    #[test]
    fn valid_code_string() {
        assert!(code_phrase("at0001").invariants().is_empty());
    }

    #[test]
    fn empty_code_string_invalid() {
        let v = code_phrase("").invariants();
        assert_eq!(v.len(), 1);
        assert_eq!(
            v[0].message,
            "Invariant Code_string_valid failed on type CODE_PHRASE"
        );
    }
}
