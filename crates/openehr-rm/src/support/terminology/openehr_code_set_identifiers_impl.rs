//! Hand-written spec constants/functions for `OPENEHR_CODE_SET_IDENTIFIERS`.
//!
//! Spec: RM 1.2.0
//! `docs/specs/openehr/RM/docs/UML/classes/`
//! `org.openehr.rm.support.openehr_code_set_identifiers.adoc` — the code-set
//! identifier constants (each `1..1`) and `valid_code_set_id`.

use super::openehr_code_set_identifiers::OpenehrCodeSetIdentifiersData;

impl OpenehrCodeSetIdentifiersData {
    /// `Code_set_id_character_sets`.
    pub const CODE_SET_ID_CHARACTER_SETS: &'static str = "character sets";
    /// `Code_set_id_compression_algorithms`.
    pub const CODE_SET_ID_COMPRESSION_ALGORITHMS: &'static str = "compression algorithms";
    /// `Code_set_id_countries`.
    pub const CODE_SET_ID_COUNTRIES: &'static str = "countries";
    /// `Code_set_integrity_check_algorithms` (the spec names this constant
    /// without the `_id` infix).
    pub const CODE_SET_INTEGRITY_CHECK_ALGORITHMS: &'static str = "integrity check algorithms";
    /// `Code_set_id_languages`.
    pub const CODE_SET_ID_LANGUAGES: &'static str = "languages";
    /// `Code_set_id_media_types`.
    pub const CODE_SET_ID_MEDIA_TYPES: &'static str = "media types";
    /// `Code_set_id_normal_statuses`.
    pub const CODE_SET_ID_NORMAL_STATUSES: &'static str = "normal statuses";

    /// The complete enumerated code-set identifier set.
    pub const ALL_CODE_SET_IDS: &'static [&'static str] = &[
        Self::CODE_SET_ID_CHARACTER_SETS,
        Self::CODE_SET_ID_COMPRESSION_ALGORITHMS,
        Self::CODE_SET_ID_COUNTRIES,
        Self::CODE_SET_INTEGRITY_CHECK_ALGORITHMS,
        Self::CODE_SET_ID_LANGUAGES,
        Self::CODE_SET_ID_MEDIA_TYPES,
        Self::CODE_SET_ID_NORMAL_STATUSES,
    ];

    /// `valid_code_set_id`: True iff `an_id` is a valid (enumerated) openEHR
    /// code-set identifier.
    #[must_use]
    pub fn valid_code_set_id(an_id: &str) -> bool {
        Self::ALL_CODE_SET_IDS.contains(&an_id)
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
    use super::super::openehr_code_set_identifiers::OpenehrCodeSetIdentifiersData;

    /// The RM class enumerates exactly 7 code-set ids; `valid_code_set_id`
    /// accepts each and nothing else.
    #[test]
    fn code_set_id_set_is_the_enumerated_seven() {
        assert_eq!(OpenehrCodeSetIdentifiersData::ALL_CODE_SET_IDS.len(), 7);
        for id in OpenehrCodeSetIdentifiersData::ALL_CODE_SET_IDS {
            assert!(
                OpenehrCodeSetIdentifiersData::valid_code_set_id(id),
                "{id} must be valid"
            );
        }
        for bad in ["", "language", "media types ", "charsets"] {
            assert!(
                !OpenehrCodeSetIdentifiersData::valid_code_set_id(bad),
                "{bad} must be invalid"
            );
        }
    }
}
