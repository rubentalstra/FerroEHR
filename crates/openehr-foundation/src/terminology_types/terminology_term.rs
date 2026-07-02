//! `Terminology_term` — a standalone term from a terminology.
//!
//! openEHR class: `Terminology_term`, package
//! `base.foundation_types.terminology`.
//! Inherits: `Any`.
//!
//! Leaf type representing a standalone term from a terminology, which
//! consists of the term text and the code, i.e. a concept reference. Per the
//! terminology chapter overview, this allows the receiver or reader of the
//! data to avoid a terminology lookup to obtain the rubric, e.g. for display
//! purposes.
use super::super::primitive_types::any::Any;
use super::super::primitive_types::string::OpenEhrString;
use super::terminology_code::TerminologyCode;

/// Leaf, non-abstract class with two attributes — transcribed as a plain
/// struct, matching the treatment given to `TerminologyCode` in this same
/// module.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TerminologyTerm {
    /// `concept: Terminology_code` (`1..1`).
    ///
    /// Reference to the terminology concept formally representing this
    /// term.
    pub concept: TerminologyCode,

    /// `text: String` (`1..1`).
    ///
    /// Text of term.
    pub text: OpenEhrString,
}

impl Any for TerminologyTerm {
    fn is_equal(&self, other: &Self) -> bool {
        self.concept == other.concept && self.text == other.text
    }

    fn type_of(&self) -> String {
        "Terminology_term".to_string()
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.terminology §Class Definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/terminology_term.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master07-terminology.adoc §Class Definitions / terminology_term.adoc §Terminology_term Class
//   confidence: high
//   todos: 0
//   note: leaf struct, two attributes transcribed 1:1; text uses OpenEhrString matching TerminologyCode's own field-type convention in this module.
// ─────────────────────────────────────────────
