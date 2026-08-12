// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-FileCopyrightText: openEHR Foundation
// SPDX-License-Identifier: MIT AND Apache-2.0

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
/// `terminology_id = name-str, [ '(', name-str, ')' ]` with
/// `name-str = letter, { letter | digit | '_' | '-' | '/' | '+' }`.
///
/// The name is `name-str` exactly: an interior space, and the `:` and `.` of a
/// URI, are all outside it.
///
/// NOTE: the version part drops `name-str`'s leading-letter requirement,
/// because every example the same chapter gives — `ICD9(1999)`,
/// `ICD10AM(3rd_ed)`, `ICD10AM(4th_ed)` (§Terminology Identifiers) — starts its
/// version with a digit, so reading `name-str` there would refuse the released
/// text's own identifiers (#2283).
#[must_use]
pub(crate) fn is_valid_terminology_id(value: &str) -> bool {
    /// The `name-str` body: `letter | digit | '_' | '-' | '/' | '+'`.
    fn body(s: &str) -> bool {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '/' | '+'))
    }
    /// `name-str = letter, { letter | digit | '_' | '-' | '/' | '+' }`.
    fn name_str(s: &str) -> bool {
        s.starts_with(|c: char| c.is_ascii_alphabetic()) && body(s)
    }
    match value.split_once('(') {
        None => name_str(value),
        Some((name, rest)) => rest
            .strip_suffix(')')
            .is_some_and(|version| name_str(name) && body(version)),
    }
}

impl crate::validate::Validate for TerminologyId {
    fn validate_invariants(&self, out: &mut Vec<crate::validate::InvariantViolation>) {
        if !is_valid_terminology_id(&self.value) {
            out.push(crate::validate::InvariantViolation::here(
                "Invariant Value_valid failed on type TERMINOLOGY_ID (the master05 \
                 §Syntaxes production `terminology_id = name-str, [ '(', name-str, \
                 ')' ]`, where a name-str starts with a letter and continues with \
                 letters, digits, '_', '-', '/' or '+')",
            ));
        }
    }
}

#[cfg(test)]
mod validity_tests {
    use super::*;

    /// The accepted forms are the production's, and they cover every
    /// terminology this implementation actually carries.
    #[test]
    fn terminology_id_follows_the_name_str_production() {
        for ok in [
            "openehr",
            "local",
            "x",
            "ISO_639-1",
            "ISO_3166-1",
            "IANA_character-sets",
            "IANA_media-types",
            "SNOMED-CT",
            "ICD10",
            "Unicode",
            // The chapter's own versioned examples (§Terminology Identifiers).
            "ICD9(1999)",
            "ICD10AM(3rd_ed)",
            "ICD10AM(4th_ed)",
            // The production admits '/' and '+' in a name-str.
            "some/terminology+ext",
        ] {
            assert!(is_valid_terminology_id(ok), "{ok:?} must be valid");
        }
    }

    /// Every refusal is asserted, so a silently loosened reader is a failing
    /// build rather than a quiet drift (`.claude/rules/spec-adherence.md`).
    #[test]
    fn terminology_id_refuses_what_the_production_forbids() {
        for bad in [
            "",
            // An interior space: the spec's own example is `SNOMED-CT`.
            "SNOMED CT",
            "SNOMED CT ",
            // A URI: master05 admits neither ':' nor '.' in a name-str.
            "http://snomed.info/sct",
            "https://vsac.nlm.nih.gov/valueset/2.16.840.1.113762.1.4.1010.2",
            // name-str must START with a letter.
            "1CD10",
            "_leading",
            // An unclosed or empty version group.
            "x(",
            "ICD10AM(3rd_ed",
            "ICD10AM()",
            // The version drops only the leading-letter rule, not the character
            // class.
            "ICD10AM(3rd ed)",
            "ICD10AM(1999.1)",
            "bad\u{7}id",
        ] {
            assert!(!is_valid_terminology_id(bad), "{bad:?} must be invalid");
        }
    }
}
