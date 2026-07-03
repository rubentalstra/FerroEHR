//! `TERMINOLOGY_ACCESS` — proxy access to one terminology.
//!
//! openEHR interface: `TERMINOLOGY_ACCESS` (RM 1.1.0,
//! `rm.support.terminology`) → Rust trait, plus the bundled implementation
//! over a parsed [`Terminology`].

use crate::bundle::Terminology;
use crate::openehr_terminology_group_identifiers::OpenehrTerminologyGroupIdentifiers;
use crate::terminology_code::TerminologyCode;

/// `TERMINOLOGY_ACCESS` interface.
///
/// The published table carries several editorial defects, each transcribed
/// per evident intent and flagged rather than silently resolved:
/// - `all_codes(): CODE_PHRASE` — evidently `List<CODE_PHRASE>` (its own
///   description says "all codes"); transcribed as `Vec<TerminologyCode>`.
/// - `has_code_for_group_id ()` is printed with an empty parameter list,
///   but its description names `a_code` and `group_id`; transcribed with
///   both parameters.
/// - `rubric_for_code(a_code)` — the description mentions a language, which
///   the signature omits; here the language is the one this access object
///   was opened for (one access per language bundle).
pub trait TerminologyAccess {
    /// Spec `id(): String` — identification of this terminology.
    fn id(&self) -> &str;

    /// Spec `all_codes()` — all codes known in this terminology.
    fn all_codes(&self) -> Vec<TerminologyCode>;

    /// Spec `codes_for_group_id(a_group_id)` — all codes under the grouper.
    /// Accepts either the space-form group id from
    /// [`OpenehrTerminologyGroupIdentifiers`] (e.g. `audit change type`) or
    /// the XML `openehr_id` underscore form (`audit_change_type`).
    fn codes_for_group_id(&self, a_group_id: &str) -> Vec<TerminologyCode>;

    /// Spec `codes_for_group_name(a_lang, a_name)` — all codes under the
    /// grouper whose name in `a_lang` is `a_name`.
    fn codes_for_group_name(&self, a_lang: &str, a_name: &str) -> Vec<TerminologyCode>;

    /// Spec `has_code_for_group_id(group_id, a_code)` — true if `a_code` is
    /// known in group `group_id` in this terminology.
    fn has_code_for_group_id(&self, a_group_id: &str, a_code: &TerminologyCode) -> bool;

    /// Spec `rubric_for_code(a_code): String` — the rubric of `a_code` in
    /// this access object's language. `None` where the spec's `String`
    /// return has no defined value for an unknown code (PORT NOTE: archie
    /// returns null there; groups are scanned in document order, so for the
    /// SPECPR-51 duplicate id `532` this yields the first occurrence —
    /// `complete`, from `version lifecycle state`).
    fn rubric_for_code(&self, a_code: &str) -> Option<String>;
}

/// [`TerminologyAccess`] over one parsed language bundle.
#[derive(Debug, Clone, Copy)]
pub struct BundledTerminologyAccess<'a> {
    terminology: &'a Terminology,
}

impl<'a> BundledTerminologyAccess<'a> {
    /// Wraps a parsed terminology document.
    #[must_use]
    pub fn new(terminology: &'a Terminology) -> Self {
        Self { terminology }
    }

    /// The language this access object serves (from the bundle's
    /// `language` attribute).
    #[must_use]
    pub fn language(&self) -> &str {
        &self.terminology.language
    }

    fn code(concept_id: &str) -> TerminologyCode {
        TerminologyCode::new(
            OpenehrTerminologyGroupIdentifiers::TERMINOLOGY_ID_OPENEHR,
            concept_id,
        )
    }
}

impl TerminologyAccess for BundledTerminologyAccess<'_> {
    fn id(&self) -> &str {
        &self.terminology.name
    }

    fn all_codes(&self) -> Vec<TerminologyCode> {
        // Duplicates (SPECPR-51 id=532) are preserved, mirroring the bundle.
        self.terminology
            .groups
            .iter()
            .flat_map(|g| g.concepts.iter())
            .map(|c| Self::code(&c.id))
            .collect()
    }

    fn codes_for_group_id(&self, a_group_id: &str) -> Vec<TerminologyCode> {
        match self.terminology.group(a_group_id) {
            Some(group) => group.concepts.iter().map(|c| Self::code(&c.id)).collect(),
            None => Vec::new(),
        }
    }

    fn codes_for_group_name(&self, a_lang: &str, a_name: &str) -> Vec<TerminologyCode> {
        if a_lang != self.terminology.language {
            // Group names are language-specific; this bundle only knows its
            // own language's names.
            return Vec::new();
        }
        match self.terminology.groups.iter().find(|g| g.name == a_name) {
            Some(group) => group.concepts.iter().map(|c| Self::code(&c.id)).collect(),
            None => Vec::new(),
        }
    }

    fn has_code_for_group_id(&self, a_group_id: &str, a_code: &TerminologyCode) -> bool {
        self.terminology
            .group(a_group_id)
            .is_some_and(|g| g.concepts.iter().any(|c| c.id == a_code.code_string))
    }

    fn rubric_for_code(&self, a_code: &str) -> Option<String> {
        self.terminology
            .groups
            .iter()
            .flat_map(|g| g.concepts.iter())
            .find(|c| c.id == a_code)
            .map(|c| c.rubric.clone())
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 support §terminology_package — docs/research/spec-cache/RM-1.1.0/support/uml_classes/terminology_access.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: terminology_access.adoc (6 functions)
//   confidence: medium
//   todos: 0
//   note: three published-table editorial defects transcribed per intent with PORT NOTEs (see trait docs); CODE_PHRASE stand-in per terminology_code.rs
// ─────────────────────────────────────────────
