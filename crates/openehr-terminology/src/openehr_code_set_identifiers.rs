//! `OPENEHR_CODE_SET_IDENTIFIERS` — list of identifiers for code sets in the
//! openEHR terminology.
//!
//! openEHR class: `OPENEHR_CODE_SET_IDENTIFIERS` (RM 1.1.0,
//! `rm.support.terminology`). Constants-only class → zero-sized struct with
//! associated consts and fns (P1 precedent); `TERMINOLOGY_SERVICE`'s
//! `Inherit` of this class is realised as direct calls.

/// `OPENEHR_CODE_SET_IDENTIFIERS` (constants-only spec class).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenehrCodeSetIdentifiers;

impl OpenehrCodeSetIdentifiers {
    /// Spec constant `Code_set_id_character_sets`.
    pub const CODE_SET_ID_CHARACTER_SETS: &'static str = "character sets";
    /// Spec constant `Code_set_id_compression_algorithms`.
    pub const CODE_SET_ID_COMPRESSION_ALGORITHMS: &'static str = "compression algorithms";
    /// Spec constant `Code_set_id_countries`.
    pub const CODE_SET_ID_COUNTRIES: &'static str = "countries";
    /// Spec constant `Code_set_integrity_check_algorithms` — the one
    /// published name WITHOUT the `_id_` infix; preserved exactly as spec'd.
    pub const CODE_SET_INTEGRITY_CHECK_ALGORITHMS: &'static str = "integrity check algorithms";
    /// Spec constant `Code_set_id_languages`.
    pub const CODE_SET_ID_LANGUAGES: &'static str = "languages";
    /// Spec constant `Code_set_id_media_types`.
    pub const CODE_SET_ID_MEDIA_TYPES: &'static str = "media types";
    /// Spec constant `Code_set_id_normal_statuses`.
    pub const CODE_SET_ID_NORMAL_STATUSES: &'static str = "normal statuses";

    /// The 7 code-set identifiers above, for membership checks.
    /// PORT NOTE: helper, not a spec member.
    const ALL_CODE_SET_IDS: [&'static str; 7] = [
        Self::CODE_SET_ID_CHARACTER_SETS,
        Self::CODE_SET_ID_COMPRESSION_ALGORITHMS,
        Self::CODE_SET_ID_COUNTRIES,
        Self::CODE_SET_INTEGRITY_CHECK_ALGORITHMS,
        Self::CODE_SET_ID_LANGUAGES,
        Self::CODE_SET_ID_MEDIA_TYPES,
        Self::CODE_SET_ID_NORMAL_STATUSES,
    ];

    /// Spec function `valid_code_set_id(an_id): Boolean` — validity function
    /// to test if an identifier is in the set defined by this class.
    #[must_use]
    pub fn valid_code_set_id(an_id: &str) -> bool {
        Self::ALL_CODE_SET_IDS.contains(&an_id)
    }
}

#[cfg(test)]
mod tests {
    use super::OpenehrCodeSetIdentifiers as Ids;

    #[test]
    fn validity_function_accepts_exactly_the_published_set() {
        assert!(Ids::valid_code_set_id("languages"));
        assert!(Ids::valid_code_set_id("integrity check algorithms"));
        assert!(!Ids::valid_code_set_id("ISO_639-1"));
        assert!(!Ids::valid_code_set_id("no such code set"));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 support §terminology_package — docs/research/spec-cache/RM-1.1.0/support/uml_classes/openehr_code_set_identifiers.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: openehr_code_set_identifiers.adoc (7 constants + 1 function)
//   confidence: high
//   todos: 0
//   note: Code_set_integrity_check_algorithms's missing _id_ infix is the spec's own naming, preserved
// ─────────────────────────────────────────────
