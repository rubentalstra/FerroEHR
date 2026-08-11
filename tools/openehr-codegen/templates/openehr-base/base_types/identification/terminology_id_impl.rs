//! Hand-written accessor functions for `TERMINOLOGY_ID`.
//!
//! Spec: BASE 1.3.0
//! `docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.base_types.terminology_id.adoc`.
//! Lexical form: `name [ '(' version ')' ]`, e.g. `SNOMED-CT`, `ICD10AM(3rd_ed)`.

use super::terminology_id::TerminologyId;

impl TerminologyId {
    /// The terminology name, i.e. the part before any `(version)` suffix (BASE
    /// `TERMINOLOGY_ID.name`). Distinct names correspond to distinct
    /// terminologies (`ICD10AM` vs `ICD10`).
    #[must_use]
    pub fn name(&self) -> &str {
        match self.value.split_once('(') {
            Some((n, _)) => n,
            None => &self.value,
        }
    }

    /// The terminology version, i.e. the part inside a trailing `(...)`, or the
    /// empty string when versioning is not used (BASE
    /// `TERMINOLOGY_ID.version_id`).
    #[must_use]
    pub fn version_id(&self) -> &str {
        self.value
            .split_once('(')
            .and_then(|(_, rest)| rest.strip_suffix(')'))
            .unwrap_or("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tid(v: &str) -> TerminologyId {
        TerminologyId {
            value: v.to_owned(),
        }
    }

    #[test]
    fn unversioned() {
        let t = tid("SNOMED-CT");
        assert_eq!(t.name(), "SNOMED-CT");
        assert_eq!(t.version_id(), "");
    }

    #[test]
    fn versioned() {
        let t = tid("ICD10AM(3rd_ed)");
        assert_eq!(t.name(), "ICD10AM");
        assert_eq!(t.version_id(), "3rd_ed");
    }

    #[test]
    fn unclosed_parenthesis_yields_empty_version() {
        let t = tid("ICD10AM(3rd_ed");
        assert_eq!(t.name(), "ICD10AM");
        assert_eq!(t.version_id(), "");
    }
}

/// Lexical validity per BASE base_types master05 §Syntaxes:
/// `terminology_id = name-str [ '(' name-str ')' ]` with
/// `name-str = letter { letter | digit | '_' | '-' | '/' | '+' }`.
///
/// This check is LOOSER than that production, which is a known divergence
/// tracked on its own issue rather than a reading of the spec: master05
/// §"Terminology Identifiers" only gives EXAMPLES ("Examples of terminology
/// identifiers include: `SNOMED-CT`, `ICD9(1999)`") and names UMLS as "the best
/// authoritative source for the name part", while real integrations identify
/// terminologies by URI (`http://snomed.info/sct`), which `name-str` forbids.
#[must_use]
pub(crate) fn is_valid_terminology_id(value: &str) -> bool {
    // TODO(#2258): enforce the master05 §Syntaxes `name-str` production, or
    // register the URI form as an adjudicated acceptance — the current check is
    // well-formedness only: non-empty, printable, not ending in whitespace,
    // with a non-empty `(version)` suffix when one is present.
    fn name_ok(s: &str) -> bool {
        !s.is_empty() && !s.ends_with(' ') && s.chars().all(|c| !c.is_control())
    }
    match (value.split_once('('), value.contains("://")) {
        // URI-form ids are taken whole (any parentheses belong to the URI).
        (_, true) | (None, false) => name_ok(value),
        (Some((name, rest)), false) => {
            name_ok(name.trim_end())
                && rest
                    .strip_suffix(')')
                    .is_some_and(|version| !version.is_empty())
        }
    }
}

impl crate::validate::Validate for TerminologyId {
    fn validate_invariants(&self, out: &mut Vec<crate::validate::InvariantViolation>) {
        if !is_valid_terminology_id(&self.value) {
            out.push(crate::validate::InvariantViolation::here(
                "Invariant Value_valid failed on type TERMINOLOGY_ID (a non-empty \
                 printable name, optionally with a non-empty '(version)' suffix — \
                 BASE base_types master05 §Syntaxes + §Terminology Identifiers)",
            ));
        }
    }
}

#[cfg(test)]
mod validity_tests {
    use super::*;

    #[test]
    fn terminology_id_lexical_form() {
        for ok in [
            "openehr",
            "ISO_639-1",
            "SNOMED CT",
            "ICD10AM(3rd_ed)",
            "local",
            // §Terminology Identifiers opens the space ("not limited to");
            // FHIR system URIs are the integration reality.
            "http://snomed.info/sct",
            "https://vsac.nlm.nih.gov/valueset/2.16.840.1.113762.1.4.1010.2",
        ] {
            assert!(is_valid_terminology_id(ok), "{ok} must be valid");
        }
        for bad in ["", "SNOMED CT ", "x(", "bad\u{7}id"] {
            assert!(!is_valid_terminology_id(bad), "{bad:?} must be invalid");
        }
    }
}
