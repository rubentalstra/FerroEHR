//! `Character` — minimal interface of the built-in Character type.
//!
//! openEHR class: `Character`, package `base.foundation_types.primitive_types`.
//! Inherits: `Ordered`.
//!
//! Type representing the minimal interface of a built-in Character type: a
//! type whose value is a member of an 8-bit character-set (ISO:
//! "repertoire"). Declares no functions of its own beyond those inherited
//! from `Ordered`.
use super::any::Any;
use super::ordered::Ordered;

/// Transcribed as a transparent newtype over `char` per `docs/PORTING.md`
/// Section 14.2.
///
/// PORT NOTE: the spec describes `Character` as an 8-bit character-set
/// member ("repertoire"), i.e. closer to a single byte in a fixed encoding
/// than to Rust's `char` (a 32-bit Unicode scalar value). `String`, the
/// sibling class in this same cluster, is explicitly documented by the spec
/// chapter overview as Unicode/UTF-8 ("It is assumed in the openEHR
/// specifications that Unicode is supported by the type `String` ... In
/// openEHR, UTF-8 encoding is assumed."), which implies `Character` as
/// `String`'s element type should, in a UTF-8 world, be a Unicode scalar
/// value rather than a raw octet — that is exactly Rust's `char`. Chosen
/// over `u8` (which is used for the separate, explicitly-8-bit-valued
/// `Octet` class in this same cluster) to avoid conflating the two distinct
/// spec types. Flagged here rather than assumed silently, since the spec
/// text for `Character` itself is terse and does not resolve this by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Character(pub char);

impl Any for Character {
    fn is_equal(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn type_of(&self) -> String {
        "Character".to_string()
    }
}

impl Ordered for Character {
    fn less_than(&self, other: &Self) -> bool {
        self.0 < other.0
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.primitive_types — docs/research/spec-cache/BASE-1.2.0/uml_classes/character.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-primitive_types.adoc §Class Definitions / character.adoc §Character Class
//   confidence: medium
//   todos: 0
//   note: chose Rust char (Unicode scalar) over u8 for the UTF-8/Unicode reasons documented on the struct; revisit if a later RM class needs raw single-octet character semantics instead.
// ─────────────────────────────────────────────
