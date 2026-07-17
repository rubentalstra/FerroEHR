//! Hand-written RM class invariants for `PARTY_IDENTIFIED`.
//!
//! Mirrors archie `PartyIdentified`:
//! - `Basic_validity`: at least one of `name`, `identifiers`, `external_ref`.
//! - `Name_valid`: if present, `name` is non-empty.

use crate::common::generic::party_identified::PartyIdentifiedData;
use crate::validate::{InvariantViolation, Validate};

/// The `Basic_validity` / `Name_valid` core over the projected inputs — one
/// source for the typed impls (`PARTY_IDENTIFIED` here, `PARTY_RELATED` which
/// inherits both) and the value-level fast path (`validate::fast`).
pub(crate) fn push_party_identified_invariants(
    rm_type: &str,
    name: Option<&str>,
    has_identifiers: bool,
    has_external_ref: bool,
    out: &mut Vec<InvariantViolation>,
) {
    if name.is_none() && !has_identifiers && !has_external_ref {
        out.push(InvariantViolation::here(format!(
            "Invariant Basic_validity failed on type {rm_type}"
        )));
    }
    if name.is_some_and(str::is_empty) {
        out.push(InvariantViolation::here(format!(
            "Invariant Name_valid failed on type {rm_type}"
        )));
    }
}

impl Validate for PartyIdentifiedData {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        push_party_identified_invariants(
            "PARTY_IDENTIFIED",
            self.name.as_deref(),
            !self.identifiers.is_empty(),
            self.external_ref.is_some(),
            out,
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

    fn party(name: Option<&str>) -> PartyIdentifiedData {
        PartyIdentifiedData {
            external_ref: None,
            name: name.map(str::to_owned),
            identifiers: Vec::new(),
        }
    }

    #[test]
    fn named_party_valid() {
        assert!(party(Some("Dr Jones")).invariants().is_empty());
    }

    #[test]
    fn no_identity_invalid() {
        let v = party(None).invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Basic_validity failed on type PARTY_IDENTIFIED"),
            "got {v:?}"
        );
    }

    #[test]
    fn empty_name_invalid() {
        let v = party(Some("")).invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Name_valid failed on type PARTY_IDENTIFIED"),
            "got {v:?}"
        );
    }
}
