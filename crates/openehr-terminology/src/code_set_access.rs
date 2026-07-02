//! `CODE_SET_ACCESS` — proxy access to one code set.
//!
//! openEHR interface: `CODE_SET_ACCESS` (RM 1.1.0,
//! `rm.support.terminology`) → Rust trait, plus the bundled implementation
//! over a parsed [`CodeSet`].

use crate::bundle::CodeSet;
use crate::terminology_code::TerminologyCode;

/// `CODE_SET_ACCESS` interface.
pub trait CodeSetAccess {
    /// Spec `id(): String` — external identifier of this code set
    /// (e.g. `ISO_639-1`, `openehr_normal_statuses`).
    fn id(&self) -> &str;

    /// Spec `all_codes(): List<CODE_PHRASE>` — all codes known in this code
    /// set.
    fn all_codes(&self) -> Vec<TerminologyCode>;

    /// Spec `has_lang(a_lang): Boolean` — true if this code set knows about
    /// `a_lang`.
    ///
    /// PORT NOTE: meaningful for the `languages` code set (ISO 639-1), where
    /// it is code membership by definition; for every other code set the
    /// spec gives no distinct semantics, so this defers to [`Self::has_code`]
    /// — flagged, not silently invented.
    fn has_lang(&self, a_lang: &str) -> bool;

    /// Spec `has_code(a_code): Boolean` — true if this code set knows about
    /// `a_code`.
    fn has_code(&self, a_code: &str) -> bool;
}

/// [`CodeSetAccess`] over one parsed `<codeset>`.
#[derive(Debug, Clone, Copy)]
pub struct BundledCodeSetAccess<'a> {
    code_set: &'a CodeSet,
}

impl<'a> BundledCodeSetAccess<'a> {
    /// Wraps a parsed code set.
    #[must_use]
    pub fn new(code_set: &'a CodeSet) -> Self {
        Self { code_set }
    }

    /// The internal openEHR name of this code set, space form
    /// (e.g. `normal statuses`).
    #[must_use]
    pub fn openehr_name(&self) -> &str {
        &self.code_set.name
    }
}

impl CodeSetAccess for BundledCodeSetAccess<'_> {
    fn id(&self) -> &str {
        &self.code_set.external_id
    }

    fn all_codes(&self) -> Vec<TerminologyCode> {
        self.code_set
            .codes
            .iter()
            .map(|c| TerminologyCode::new(self.code_set.external_id.clone(), c.value.clone()))
            .collect()
    }

    fn has_lang(&self, a_lang: &str) -> bool {
        self.has_code(a_lang)
    }

    fn has_code(&self, a_code: &str) -> bool {
        self.code_set.codes.iter().any(|c| c.value == a_code)
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 support §terminology_package — docs/research/spec-cache/RM-1.1.0/support/uml_classes/code_set_access.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: code_set_access.adoc (4 functions)
//   confidence: high
//   todos: 0
//   note: has_lang delegates to has_code with a PORT NOTE (spec defines no distinct semantics outside the languages set)
// ─────────────────────────────────────────────
