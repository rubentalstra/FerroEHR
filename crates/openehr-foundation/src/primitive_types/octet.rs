//! `Octet` — minimal interface of the built-in Octet type.
//!
//! openEHR class: `Octet`, package `base.foundation_types.primitive_types`.
//! Inherits: `Ordered`.
//!
//! Type representing the minimal interface of a built-in Octet type: a type
//! whose value is an 8-bit value. Declares no functions of its own beyond
//! those inherited from `Ordered`.
//!
//! PORT NOTE: this is `Octet`, not "Byte" — see
//! `.claude/rules/rm-transcription.md` "Known hazards".
use super::any::Any;
use super::ordered::Ordered;

/// Transcribed as a transparent newtype over `u8` per `docs/PORTING.md`
/// Section 14.2 (`byte`/8-bit value → `u8`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Octet(pub u8);

impl Any for Octet {
    fn is_equal(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn type_of(&self) -> String {
        "Octet".to_string()
    }
}

impl Ordered for Octet {
    fn less_than(&self, other: &Self) -> bool {
        self.0 < other.0
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/octet.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / octet.adoc §Octet Class
//   confidence: high
//   todos: 0
//   note: named Octet per spec, not Byte, per the RM transcription rule's known-hazards list.
// ─────────────────────────────────────────────
