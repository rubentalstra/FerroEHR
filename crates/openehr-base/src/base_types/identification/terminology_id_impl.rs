//! Hand-written accessor functions (ADR-003) for `TERMINOLOGY_ID`.
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
